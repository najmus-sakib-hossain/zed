//! Memory-mapped package access

use memmap2::Mmap;
use std::sync::Arc;

use crate::index::PackageIndex;

/// Memory-mapped package data
pub struct MappedPackage {
    /// Memory-mapped file
    mmap: Mmap,
    /// Package hash
    hash: [u8; 32],
    /// Data offset within the file
    data_offset: u64,
}

impl MappedPackage {
    /// Create a new mapped package
    pub fn new(mmap: Mmap, hash: [u8; 32], data_offset: u64) -> Self {
        Self {
            mmap,
            hash,
            data_offset,
        }
    }

    /// Get the package hash
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Get the raw memory-mapped data
    pub fn raw_data(&self) -> &[u8] {
        &self.mmap
    }

    /// Get file data by path (zero-copy slice)
    pub fn get_file(&self, index: &PackageIndex, path: &str) -> Option<&[u8]> {
        let entry = index.find_file(path)?;
        let start = self.data_offset as usize + entry.offset as usize;
        let end = start + entry.size as usize;

        if end <= self.mmap.len() {
            Some(&self.mmap[start..end])
        } else {
            None
        }
    }

    /// Get file data by entry (zero-copy slice)
    pub fn get_file_by_entry(&self, entry: &crate::index::PackageFileEntry) -> Option<&[u8]> {
        let start = self.data_offset as usize + entry.offset as usize;
        let end = start + entry.size as usize;

        if end <= self.mmap.len() {
            Some(&self.mmap[start..end])
        } else {
            None
        }
    }

    /// Iterate over all files with their data
    pub fn files<'a>(
        &'a self,
        index: &'a PackageIndex,
    ) -> impl Iterator<Item = (&'a str, &'a [u8])> {
        index.iter().filter_map(move |entry| {
            let data = self.get_file_by_entry(entry)?;
            Some((entry.path_str(), data))
        })
    }
}

/// Thread-safe wrapper for mapped package
#[allow(dead_code)]
pub type SharedMappedPackage = Arc<MappedPackage>;
