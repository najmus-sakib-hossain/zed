//! DPL (Dx Python Lock) format implementation
//!
//! Provides zero-copy access to DPL lock files via memory mapping,
//! with O(1) package lookup using hash tables.

use dx_py_core::{
    fnv1a_hash,
    headers::{DplEntry, DplHeader},
    DPL_MAGIC, MAX_LOCK_SIZE, PROTOCOL_VERSION,
};
use memmap2::Mmap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

/// Zero-copy DPL lock file access
///
/// DplLockFile provides memory-mapped access to DPL format lock files,
/// enabling O(1) package lookup via hash table.
pub struct DplLockFile {
    /// Memory-mapped file contents
    mmap: Mmap,
    /// Cached header (copied for safe access to packed fields)
    header: DplHeader,
}

impl DplLockFile {
    /// Open a DPL lock file with memory mapping
    ///
    /// # Arguments
    /// * `path` - Path to the DPL lock file
    ///
    /// # Returns
    /// * `Ok(DplLockFile)` - Successfully opened lock file
    /// * `Err(Error)` - Failed to open or validate lock file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let metadata = file.metadata()?;

        // Check file size limits
        if metadata.len() > MAX_LOCK_SIZE {
            return Err(Error::PackageTooLarge {
                size: metadata.len(),
                limit: MAX_LOCK_SIZE,
            });
        }

        // Memory map the file
        let mmap = unsafe { Mmap::map(&file)? };

        Self::from_bytes_internal(mmap)
    }

    /// Load a DPL lock file from bytes (for testing)
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        // Create a temporary file and memory map it
        let mut temp = tempfile::NamedTempFile::new()?;
        temp.write_all(&data)?;
        temp.flush()?;

        let file = temp.reopen()?;
        let mmap = unsafe { Mmap::map(&file)? };

        Self::from_bytes_internal(mmap)
    }

    fn from_bytes_internal(mmap: Mmap) -> Result<Self> {
        // Verify minimum size for header
        let header_size = std::mem::size_of::<DplHeader>();
        if mmap.len() < header_size {
            return Err(Error::InvalidMagic {
                expected: *DPL_MAGIC,
                found: [0, 0, 0, 0],
            });
        }

        // Verify magic number
        if &mmap[0..4] != DPL_MAGIC {
            let mut found = [0u8; 4];
            found.copy_from_slice(&mmap[0..4]);
            return Err(Error::InvalidMagic {
                expected: *DPL_MAGIC,
                found,
            });
        }

        // Zero-copy header access via bytemuck
        let header: DplHeader = *bytemuck::from_bytes(&mmap[0..header_size]);

        // Verify protocol version
        if header.version != PROTOCOL_VERSION {
            return Err(Error::UnsupportedVersion(header.version));
        }

        Ok(Self { mmap, header })
    }

    /// Get the lock file header
    #[inline]
    pub fn header(&self) -> &DplHeader {
        &self.header
    }

    /// Get the number of packages in the lock file
    #[inline]
    pub fn package_count(&self) -> u32 {
        self.header.package_count
    }

    /// Get the Python version
    pub fn python_version(&self) -> &str {
        self.header.python_version_str()
    }

    /// Get the platform string
    pub fn platform(&self) -> &str {
        self.header.platform_str()
    }

    /// O(1) package lookup using hash table
    ///
    /// # Arguments
    /// * `package_name` - Name of the package to look up
    ///
    /// # Returns
    /// * `Some(&DplEntry)` - Package found
    /// * `None` - Package not found
    pub fn lookup(&self, package_name: &str) -> Option<DplEntry> {
        let hash = fnv1a_hash(package_name.as_bytes());
        let hash_table_size = self.header.hash_table_size as u64;

        if hash_table_size == 0 {
            return None;
        }

        let slot = (hash % hash_table_size) as usize;

        // Read hash table entry (u32 index into entries array)
        let table_offset = self.header.hash_table_offset as usize;
        let slot_offset = table_offset + slot * 4;

        if slot_offset + 4 > self.mmap.len() {
            return None;
        }

        let entry_idx: u32 = *bytemuck::from_bytes(&self.mmap[slot_offset..slot_offset + 4]);

        // u32::MAX means empty slot
        if entry_idx == u32::MAX {
            return None;
        }

        // Linear probing for collision handling
        let mut current_idx = entry_idx;
        let entries_offset = self.header.entries_offset as usize;
        let entry_size = std::mem::size_of::<DplEntry>();

        for _ in 0..self.header.package_count {
            let entry_offset = entries_offset + (current_idx as usize * entry_size);

            if entry_offset + entry_size > self.mmap.len() {
                return None;
            }

            let entry: DplEntry =
                *bytemuck::from_bytes(&self.mmap[entry_offset..entry_offset + entry_size]);

            // Check if this is the entry we're looking for
            if entry.name_hash == hash && entry.name_str() == package_name {
                return Some(entry);
            }

            // Linear probing: check next slot
            current_idx = (current_idx + 1) % self.header.package_count;
        }

        None
    }

    /// Iterate over all packages in the lock file
    pub fn iter(&self) -> DplIterator<'_> {
        DplIterator {
            lock_file: self,
            index: 0,
        }
    }

    /// Verify integrity with BLAKE3
    pub fn verify(&self) -> bool {
        let entries_offset = self.header.entries_offset as usize;
        let entry_size = std::mem::size_of::<DplEntry>();
        let entries_end = entries_offset + (self.header.package_count as usize * entry_size);

        if entries_end > self.mmap.len() {
            return false;
        }

        let content = &self.mmap[entries_offset..entries_end];
        let computed = blake3::hash(content);

        computed.as_bytes() == &self.header.content_hash
    }

    /// Get raw access to the memory-mapped data
    pub fn raw_data(&self) -> &[u8] {
        &self.mmap
    }
}

/// Iterator over DPL entries
pub struct DplIterator<'a> {
    lock_file: &'a DplLockFile,
    index: u32,
}

impl<'a> Iterator for DplIterator<'a> {
    type Item = DplEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.lock_file.header.package_count {
            return None;
        }

        let entries_offset = self.lock_file.header.entries_offset as usize;
        let entry_size = std::mem::size_of::<DplEntry>();
        let entry_offset = entries_offset + (self.index as usize * entry_size);

        if entry_offset + entry_size > self.lock_file.mmap.len() {
            return None;
        }

        let entry: DplEntry =
            *bytemuck::from_bytes(&self.lock_file.mmap[entry_offset..entry_offset + entry_size]);

        self.index += 1;
        Some(entry)
    }
}

/// Builder for creating DPL lock files
pub struct DplBuilder {
    entries: Vec<DplEntry>,
    python_version: String,
    platform: String,
    /// Mapping of extra names to indices (for bitmap encoding)
    extras_map: std::collections::HashMap<String, u8>,
    next_extra_index: u8,
}

impl DplBuilder {
    /// Create a new DPL builder
    pub fn new(python_version: &str, platform: &str) -> Self {
        Self {
            entries: Vec::new(),
            python_version: python_version.to_string(),
            platform: platform.to_string(),
            extras_map: std::collections::HashMap::new(),
            next_extra_index: 0,
        }
    }

    /// Add a package to the lock file
    pub fn add_package(&mut self, name: &str, version: &str, source_hash: [u8; 32]) -> &mut Self {
        let entry = DplEntry::new(name, version, source_hash);
        self.entries.push(entry);
        self
    }

    /// Add a package with extras to the lock file
    pub fn add_package_with_extras(
        &mut self,
        name: &str,
        version: &str,
        source_hash: [u8; 32],
        extras: &[&str],
    ) -> &mut Self {
        let extras_bitmap = self.encode_extras(extras);
        let entry = DplEntry::new_with_extras(name, version, source_hash, extras_bitmap);
        self.entries.push(entry);
        self
    }

    /// Encode extras into a bitmap
    fn encode_extras(&mut self, extras: &[&str]) -> u64 {
        let mut bitmap = 0u64;
        for extra in extras {
            let index = self.get_or_create_extra_index(extra);
            if index < 64 {
                bitmap |= 1u64 << index;
            }
        }
        bitmap
    }

    /// Get or create an index for an extra name
    fn get_or_create_extra_index(&mut self, extra: &str) -> u8 {
        if let Some(&index) = self.extras_map.get(extra) {
            return index;
        }

        if self.next_extra_index >= 64 {
            return 63; // Cap at 64 extras
        }

        let index = self.next_extra_index;
        self.extras_map.insert(extra.to_string(), index);
        self.next_extra_index += 1;
        index
    }

    /// Get the extras map (for decoding)
    pub fn extras_map(&self) -> &std::collections::HashMap<String, u8> {
        &self.extras_map
    }

    /// Add a package entry directly
    pub fn add_entry(&mut self, entry: DplEntry) -> &mut Self {
        self.entries.push(entry);
        self
    }

    /// Build the DPL lock file as bytes
    pub fn build(&self) -> Vec<u8> {
        let header_size = std::mem::size_of::<DplHeader>();
        let entry_size = std::mem::size_of::<DplEntry>();
        let package_count = self.entries.len() as u32;

        // Hash table size: use 1.5x package count for good load factor
        let hash_table_size = ((package_count as f64 * 1.5) as u32).max(1);
        let hash_table_bytes = hash_table_size as usize * 4;

        // Calculate offsets
        let hash_table_offset = header_size as u32;
        let entries_offset = hash_table_offset + hash_table_bytes as u32;
        let total_size = entries_offset as usize + (package_count as usize * entry_size);

        // Allocate output buffer
        let mut output = vec![0u8; total_size];

        // Build hash table (initialize with u32::MAX = empty)
        let mut hash_table = vec![u32::MAX; hash_table_size as usize];

        // Insert entries into hash table using linear probing
        for (idx, entry) in self.entries.iter().enumerate() {
            let slot = (entry.name_hash % hash_table_size as u64) as usize;

            // Linear probing to find empty slot
            let mut current_slot = slot;
            loop {
                if hash_table[current_slot] == u32::MAX {
                    hash_table[current_slot] = idx as u32;
                    break;
                }
                current_slot = (current_slot + 1) % hash_table_size as usize;
            }
        }

        // Compute content hash (hash of all entries)
        let mut entries_bytes = Vec::with_capacity(package_count as usize * entry_size);
        for entry in &self.entries {
            entries_bytes.extend_from_slice(bytemuck::bytes_of(entry));
        }
        let content_hash = blake3::hash(&entries_bytes);

        // Build header
        let mut python_version_bytes = [0u8; 16];
        let pv_len = self.python_version.len().min(15);
        python_version_bytes[..pv_len].copy_from_slice(&self.python_version.as_bytes()[..pv_len]);

        let mut platform_bytes = [0u8; 32];
        let pl_len = self.platform.len().min(31);
        platform_bytes[..pl_len].copy_from_slice(&self.platform.as_bytes()[..pl_len]);

        let resolved_at =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

        let header = DplHeader {
            magic: *DPL_MAGIC,
            version: PROTOCOL_VERSION,
            package_count,
            _padding: 0,
            hash_table_offset,
            hash_table_size,
            entries_offset,
            python_version: python_version_bytes,
            platform: platform_bytes,
            resolved_at,
            content_hash: *content_hash.as_bytes(),
        };

        // Write header
        output[0..header_size].copy_from_slice(bytemuck::bytes_of(&header));

        // Write hash table
        for (i, &idx) in hash_table.iter().enumerate() {
            let offset = header_size + i * 4;
            output[offset..offset + 4].copy_from_slice(&idx.to_le_bytes());
        }

        // Write entries
        output[entries_offset as usize..].copy_from_slice(&entries_bytes);

        output
    }

    /// Build and write to a file
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let data = self.build();
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpl_builder_roundtrip() {
        let mut builder = DplBuilder::new("3.12.0", "manylinux_2_17_x86_64");
        builder.add_package("requests", "2.31.0", [1u8; 32]);
        builder.add_package("numpy", "1.26.0", [2u8; 32]);
        builder.add_package("pandas", "2.1.0", [3u8; 32]);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        assert_eq!(lock_file.package_count(), 3);
        assert_eq!(lock_file.python_version(), "3.12.0");
        assert_eq!(lock_file.platform(), "manylinux_2_17_x86_64");

        // Test lookup
        let requests = lock_file.lookup("requests").unwrap();
        assert_eq!(requests.name_str(), "requests");
        assert_eq!(requests.version_str(), "2.31.0");

        let numpy = lock_file.lookup("numpy").unwrap();
        assert_eq!(numpy.name_str(), "numpy");
        assert_eq!(numpy.version_str(), "1.26.0");

        // Test non-existent package
        assert!(lock_file.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_dpl_iterator() {
        let mut builder = DplBuilder::new("3.11.0", "win_amd64");
        builder.add_package("flask", "3.0.0", [4u8; 32]);
        builder.add_package("django", "4.2.0", [5u8; 32]);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        let entries: Vec<_> = lock_file.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_dpl_verify() {
        let mut builder = DplBuilder::new("3.12.0", "linux");
        builder.add_package("test", "1.0.0", [0u8; 32]);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        assert!(lock_file.verify());
    }
}
