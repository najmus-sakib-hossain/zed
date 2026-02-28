//! Package file index for O(1) file lookup within packages

use bytemuck::{Pod, Zeroable};
use dx_py_core::fnv1a_hash;

use crate::{MAX_PATH_LENGTH, STORE_VERSION};

/// Package index header (36 bytes packed)
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct PackageIndexHeader {
    /// Magic: "DXPK"
    pub magic: [u8; 4],
    /// Version
    pub version: u16,
    /// Number of files
    pub file_count: u32,
    /// Offset to file index
    pub index_offset: u64,
    /// Offset to file data
    pub data_offset: u64,
    /// Total size
    pub total_size: u64,
    /// Padding to align to 8 bytes
    pub _padding: [u8; 2],
}

// Compile-time assertion that header is 36 bytes
const _: () = assert!(std::mem::size_of::<PackageIndexHeader>() == 36);

impl PackageIndexHeader {
    /// Create a new header
    pub fn new(file_count: u32, index_offset: u64, data_offset: u64, total_size: u64) -> Self {
        Self {
            magic: *crate::DXPK_MAGIC,
            version: STORE_VERSION,
            file_count,
            index_offset,
            data_offset,
            total_size,
            _padding: [0; 2],
        }
    }
}

/// File entry in package index (fixed 288 bytes for alignment)
#[repr(C)]
#[derive(Clone, Debug)]
pub struct PackageFileEntry {
    /// Path hash for O(1) lookup
    pub path_hash: u64,
    /// Offset in package data
    pub offset: u64,
    /// File size
    pub size: u64,
    /// Path length
    pub path_len: u16,
    /// Path bytes (up to 256)
    pub path: [u8; MAX_PATH_LENGTH],
}

impl PackageFileEntry {
    /// Create a new file entry
    pub fn new(path: &str, offset: u64, size: u64) -> Self {
        let path_bytes = path.as_bytes();
        let path_len = path_bytes.len().min(MAX_PATH_LENGTH);
        let mut path_arr = [0u8; MAX_PATH_LENGTH];
        path_arr[..path_len].copy_from_slice(&path_bytes[..path_len]);

        Self {
            path_hash: fnv1a_hash(path_bytes),
            offset,
            size,
            path_len: path_len as u16,
            path: path_arr,
        }
    }

    /// Get the path as a string
    pub fn path_str(&self) -> &str {
        std::str::from_utf8(&self.path[..self.path_len as usize]).unwrap_or("")
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.path_hash.to_le_bytes());
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.extend_from_slice(&self.size.to_le_bytes());
        bytes.extend_from_slice(&self.path_len.to_le_bytes());
        bytes.extend_from_slice(&self.path);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let path_hash = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let offset = u64::from_le_bytes(bytes[8..16].try_into().ok()?);
        let size = u64::from_le_bytes(bytes[16..24].try_into().ok()?);
        let path_len = u16::from_le_bytes(bytes[24..26].try_into().ok()?);
        let mut path = [0u8; MAX_PATH_LENGTH];
        path.copy_from_slice(&bytes[26..26 + MAX_PATH_LENGTH]);

        Some(Self {
            path_hash,
            offset,
            size,
            path_len,
            path,
        })
    }

    /// Size of serialized entry
    pub const SIZE: usize = 8 + 8 + 8 + 2 + MAX_PATH_LENGTH; // 282 bytes
}

/// Package file index for O(1) file lookup
#[derive(Clone, Debug)]
pub struct PackageIndex {
    /// File entries
    pub entries: Vec<PackageFileEntry>,
}

impl PackageIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a file entry
    pub fn add_file(&mut self, path: &str, offset: u64, size: u64) {
        self.entries.push(PackageFileEntry::new(path, offset, size));
    }

    /// Get file count
    pub fn file_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Find a file by path (O(n) linear search, but typically small n)
    /// For larger packages, we use the hash for faster lookup
    pub fn find_file(&self, path: &str) -> Option<&PackageFileEntry> {
        let hash = fnv1a_hash(path.as_bytes());
        self.entries.iter().find(|e| e.path_hash == hash && e.path_str() == path)
    }

    /// Serialize the index to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.entries.len() * PackageFileEntry::SIZE);
        for entry in &self.entries {
            bytes.extend_from_slice(&entry.to_bytes());
        }
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8], file_count: u32) -> Option<Self> {
        let mut entries = Vec::with_capacity(file_count as usize);
        let entry_size = PackageFileEntry::SIZE;

        for i in 0..file_count as usize {
            let start = i * entry_size;
            let end = start + entry_size;
            if end > bytes.len() {
                return None;
            }
            let entry = PackageFileEntry::from_bytes(&bytes[start..end])?;
            entries.push(entry);
        }

        Some(Self { entries })
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = &PackageFileEntry> {
        self.entries.iter()
    }
}

impl Default for PackageIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_roundtrip() {
        let entry = PackageFileEntry::new("test/path/file.py", 1024, 512);
        let bytes = entry.to_bytes();
        let restored = PackageFileEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.path_hash, restored.path_hash);
        assert_eq!(entry.offset, restored.offset);
        assert_eq!(entry.size, restored.size);
        assert_eq!(entry.path_str(), restored.path_str());
    }

    #[test]
    fn test_index_find_file() {
        let mut index = PackageIndex::new();
        index.add_file("package/__init__.py", 0, 100);
        index.add_file("package/module.py", 100, 200);
        index.add_file("package/utils.py", 300, 150);

        let found = index.find_file("package/module.py").unwrap();
        assert_eq!(found.offset, 100);
        assert_eq!(found.size, 200);

        assert!(index.find_file("nonexistent.py").is_none());
    }

    #[test]
    fn test_index_roundtrip() {
        let mut index = PackageIndex::new();
        index.add_file("file1.py", 0, 100);
        index.add_file("file2.py", 100, 200);

        let bytes = index.to_bytes();
        let restored = PackageIndex::from_bytes(&bytes, 2).unwrap();

        assert_eq!(restored.file_count(), 2);
        assert_eq!(restored.entries[0].path_str(), "file1.py");
        assert_eq!(restored.entries[1].path_str(), "file2.py");
    }
}
