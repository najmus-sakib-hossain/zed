//! Layout index with memory-mapped access

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use dx_py_core::fnv1a_hash;
use memmap2::Mmap;

use crate::headers::{LayoutEntry, LayoutIndexHeader};
use crate::{LayoutError, LayoutResult, DXLC_MAGIC, LAYOUT_VERSION};

/// Layout index with O(1) lookup
pub struct LayoutIndex {
    /// Path to index file
    path: PathBuf,
    /// Memory-mapped index data
    mmap: Option<Mmap>,
    /// Cached header
    header: LayoutIndexHeader,
}

impl LayoutIndex {
    /// Open or create a layout index
    pub fn open<P: AsRef<Path>>(path: P) -> LayoutResult<Self> {
        let path = path.as_ref().to_path_buf();

        if path.exists() {
            Self::load(&path)
        } else {
            Self::create(&path)
        }
    }

    /// Load existing index
    fn load(path: &Path) -> LayoutResult<Self> {
        let file = File::open(path)?;
        let mmap =
            unsafe { Mmap::map(&file).map_err(|e| LayoutError::IndexCorrupted(e.to_string()))? };

        // Verify minimum size
        let header_size = std::mem::size_of::<LayoutIndexHeader>();
        if mmap.len() < header_size {
            return Err(LayoutError::IndexCorrupted("file too small".to_string()));
        }

        // Verify magic
        if &mmap[0..4] != DXLC_MAGIC {
            let mut found = [0u8; 4];
            found.copy_from_slice(&mmap[0..4]);
            return Err(LayoutError::InvalidMagic {
                expected: *DXLC_MAGIC,
                found,
            });
        }

        let header: LayoutIndexHeader = *bytemuck::from_bytes(&mmap[0..header_size]);

        if header.version != LAYOUT_VERSION {
            return Err(LayoutError::UnsupportedVersion(header.version));
        }

        Ok(Self {
            path: path.to_path_buf(),
            mmap: Some(mmap),
            header,
        })
    }

    /// Create new empty index
    fn create(path: &Path) -> LayoutResult<Self> {
        let header = LayoutIndexHeader::new(0, 64, 0, 64);

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write empty index
        let mut file = File::create(path)?;
        file.write_all(bytemuck::bytes_of(&header))?;
        file.flush()?;

        Ok(Self {
            path: path.to_path_buf(),
            mmap: None,
            header,
        })
    }

    /// Get layout count
    pub fn layout_count(&self) -> u32 {
        self.header.layout_count
    }

    /// O(1) lookup for layout by project hash
    pub fn get(&self, project_hash: &[u8; 32]) -> Option<LayoutEntry> {
        let mmap = self.mmap.as_ref()?;

        if self.header.hash_table_size == 0 {
            return None;
        }

        let hash = fnv1a_hash(project_hash);
        let slot = (hash % self.header.hash_table_size as u64) as usize;

        // Read hash table entry
        let table_offset = self.header.hash_table_offset as usize;
        let slot_offset = table_offset + slot * 4;

        if slot_offset + 4 > mmap.len() {
            return None;
        }

        let entry_idx: u32 = *bytemuck::from_bytes(&mmap[slot_offset..slot_offset + 4]);

        // u32::MAX means empty slot
        if entry_idx == u32::MAX {
            return None;
        }

        // Linear probing for collision handling
        let entries_offset = self.header.entries_offset as usize;
        let entry_size = std::mem::size_of::<LayoutEntry>();
        let mut current_idx = entry_idx;

        for _ in 0..self.header.layout_count {
            let entry_offset = entries_offset + (current_idx as usize * entry_size);

            if entry_offset + entry_size > mmap.len() {
                return None;
            }

            let entry: LayoutEntry =
                *bytemuck::from_bytes(&mmap[entry_offset..entry_offset + entry_size]);

            if &entry.project_hash == project_hash {
                return Some(entry);
            }

            current_idx = (current_idx + 1) % self.header.layout_count;
        }

        None
    }

    /// Check if layout exists
    pub fn contains(&self, project_hash: &[u8; 32]) -> bool {
        self.get(project_hash).is_some()
    }

    /// Add a layout entry (rebuilds index)
    pub fn add(&mut self, entry: LayoutEntry) -> LayoutResult<()> {
        // Load all existing entries
        let mut entries = self.all_entries();

        // Check if already exists (update if so)
        if let Some(pos) = entries.iter().position(|e| e.project_hash == entry.project_hash) {
            entries[pos] = entry;
        } else {
            entries.push(entry);
        }

        // Rebuild index
        self.rebuild(entries)
    }

    /// Remove a layout entry
    pub fn remove(&mut self, project_hash: &[u8; 32]) -> LayoutResult<bool> {
        let mut entries = self.all_entries();
        let original_len = entries.len();
        entries.retain(|e| &e.project_hash != project_hash);

        if entries.len() < original_len {
            self.rebuild(entries)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get all entries
    fn all_entries(&self) -> Vec<LayoutEntry> {
        let Some(mmap) = &self.mmap else {
            return Vec::new();
        };

        let entries_offset = self.header.entries_offset as usize;
        let entry_size = std::mem::size_of::<LayoutEntry>();
        let mut entries = Vec::with_capacity(self.header.layout_count as usize);

        for i in 0..self.header.layout_count as usize {
            let offset = entries_offset + i * entry_size;
            if offset + entry_size <= mmap.len() {
                let entry: LayoutEntry = *bytemuck::from_bytes(&mmap[offset..offset + entry_size]);
                entries.push(entry);
            }
        }

        entries
    }

    /// Rebuild the index with new entries
    fn rebuild(&mut self, entries: Vec<LayoutEntry>) -> LayoutResult<()> {
        let header_size = std::mem::size_of::<LayoutIndexHeader>();
        let entry_size = std::mem::size_of::<LayoutEntry>();
        let layout_count = entries.len() as u32;

        // Hash table size: 1.5x entry count for good load factor
        let hash_table_size = ((layout_count as f64 * 1.5) as u32).max(1);
        let hash_table_bytes = hash_table_size as usize * 4;

        // Calculate offsets
        let hash_table_offset = header_size as u32;
        let entries_offset = hash_table_offset + hash_table_bytes as u32;
        let total_size = entries_offset as usize + (layout_count as usize * entry_size);

        // Build hash table
        let mut hash_table = vec![u32::MAX; hash_table_size as usize];

        for (idx, entry) in entries.iter().enumerate() {
            let hash = fnv1a_hash(&entry.project_hash);
            let mut slot = (hash % hash_table_size as u64) as usize;

            // Linear probing
            loop {
                if hash_table[slot] == u32::MAX {
                    hash_table[slot] = idx as u32;
                    break;
                }
                slot = (slot + 1) % hash_table_size as usize;
            }
        }

        // Build header
        let header = LayoutIndexHeader::new(
            layout_count,
            hash_table_offset,
            hash_table_size,
            entries_offset,
        );

        // Assemble output
        let mut output = vec![0u8; total_size];
        output[0..header_size].copy_from_slice(bytemuck::bytes_of(&header));

        // Write hash table
        for (i, &idx) in hash_table.iter().enumerate() {
            let offset = header_size + i * 4;
            output[offset..offset + 4].copy_from_slice(&idx.to_le_bytes());
        }

        // Write entries
        for (i, entry) in entries.iter().enumerate() {
            let offset = entries_offset as usize + i * entry_size;
            output[offset..offset + entry_size].copy_from_slice(bytemuck::bytes_of(entry));
        }

        // Write atomically
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, &output)?;
        fs::rename(&temp_path, &self.path)?;

        // Reload
        *self = Self::load(&self.path)?;

        Ok(())
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = LayoutEntry> + '_ {
        self.all_entries().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_create_and_add() {
        let temp = TempDir::new().unwrap();
        let index_path = temp.path().join("layouts.dxc");

        let mut index = LayoutIndex::open(&index_path).unwrap();
        assert_eq!(index.layout_count(), 0);

        let entry = LayoutEntry::new([1u8; 32], "layout_001", 5, 1024);
        index.add(entry).unwrap();

        assert_eq!(index.layout_count(), 1);
        assert!(index.contains(&[1u8; 32]));
    }

    #[test]
    fn test_index_lookup() {
        let temp = TempDir::new().unwrap();
        let index_path = temp.path().join("layouts.dxc");

        let mut index = LayoutIndex::open(&index_path).unwrap();

        let entry1 = LayoutEntry::new([1u8; 32], "layout_001", 5, 1024);
        let entry2 = LayoutEntry::new([2u8; 32], "layout_002", 10, 2048);

        index.add(entry1).unwrap();
        index.add(entry2).unwrap();

        let found = index.get(&[1u8; 32]).unwrap();
        assert_eq!(found.layout_name_str(), "layout_001");
        // Copy packed field to avoid unaligned reference
        let pkg_count = { found.package_count };
        assert_eq!(pkg_count, 5);

        let found = index.get(&[2u8; 32]).unwrap();
        assert_eq!(found.layout_name_str(), "layout_002");

        assert!(index.get(&[3u8; 32]).is_none());
    }

    #[test]
    fn test_index_remove() {
        let temp = TempDir::new().unwrap();
        let index_path = temp.path().join("layouts.dxc");

        let mut index = LayoutIndex::open(&index_path).unwrap();

        index.add(LayoutEntry::new([1u8; 32], "layout_001", 5, 1024)).unwrap();
        index.add(LayoutEntry::new([2u8; 32], "layout_002", 10, 2048)).unwrap();

        assert_eq!(index.layout_count(), 2);

        index.remove(&[1u8; 32]).unwrap();

        assert_eq!(index.layout_count(), 1);
        assert!(!index.contains(&[1u8; 32]));
        assert!(index.contains(&[2u8; 32]));
    }
}
