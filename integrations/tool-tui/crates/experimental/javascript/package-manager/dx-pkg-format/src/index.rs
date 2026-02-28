use bytemuck::{Pod, Zeroable};
use dx_pkg_core::{DxpHeader, Error, Result};
use memmap2::Mmap;

/// File index entry (24 bytes)
#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub struct FileIndexEntry {
    pub path_hash: u64,       // Hash of file path
    pub offset: u64,          // Offset in file data section
    pub size: u32,            // Uncompressed size
    pub compressed_size: u32, // Compressed size (0 if uncompressed)
    pub flags: u8,            // Flags (compression type, etc.)
    pub _reserved: [u8; 3],   // Alignment padding
}

/// File index (hash table for O(1) lookups)
pub struct FileIndex {
    entries: Vec<FileIndexEntry>,
    table_size: u32,
}

impl FileIndex {
    /// Load index from memory-mapped file
    pub fn from_mmap(mmap: &Mmap, header: &DxpHeader) -> Result<Self> {
        let index_offset = header.index_offset as usize;
        let table_size = Self::compute_table_size(header.file_count);
        let entry_size = std::mem::size_of::<FileIndexEntry>();
        let index_bytes = table_size as usize * entry_size;

        if index_offset + index_bytes > mmap.len() {
            return Err(Error::CorruptedData);
        }

        let entries: Vec<FileIndexEntry> = (0..table_size)
            .map(|i| {
                let offset = index_offset + (i as usize * entry_size);
                *bytemuck::from_bytes(&mmap[offset..offset + entry_size])
            })
            .collect();

        Ok(Self {
            entries,
            table_size,
        })
    }

    /// Find file entry by path hash (O(1) with open addressing)
    pub fn find(&self, path_hash: u64) -> Result<&FileIndexEntry> {
        let mut idx = (path_hash % self.table_size as u64) as usize;
        let mut probes = 0;

        loop {
            let entry = &self.entries[idx];

            if entry.path_hash == path_hash {
                return Ok(entry);
            }

            if entry.path_hash == 0 {
                return Err(Error::file_not_found(format!("hash: {}", path_hash)));
            }

            // Quadratic probing
            probes += 1;
            idx = (path_hash as usize + probes * probes) % self.table_size as usize;

            if probes > self.table_size as usize {
                return Err(Error::file_not_found(format!("hash: {}", path_hash)));
            }
        }
    }

    /// List all files (returns hashes, actual paths not stored in index)
    pub fn list(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.path_hash != 0)
            .map(|e| {
                let hash = e.path_hash;
                format!("file_{:016x}", hash)
            })
            .collect()
    }

    /// Compute optimal hash table size (next power of 2, 2x file count)
    fn compute_table_size(file_count: u32) -> u32 {
        let target = file_count * 2;
        target.next_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_table_size() {
        assert_eq!(FileIndex::compute_table_size(10), 32);
        assert_eq!(FileIndex::compute_table_size(100), 256);
        assert_eq!(FileIndex::compute_table_size(1000), 2048);
    }
}
