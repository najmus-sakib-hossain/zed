//! DPM Loader with memory mapping for zero-copy access

use dashmap::DashMap;
use memmap2::Mmap;
use std::path::Path;
use std::sync::Arc;

use crate::export_table::ExportTable;
use crate::format::{DpmError, DpmHeader, ExportEntry, ImportEntry};

/// A loaded DPM module with memory-mapped access
pub struct LoadedModule {
    /// Memory-mapped file data
    mmap: Mmap,
    /// Parsed header
    header: DpmHeader,
    /// Export table for O(1) lookup
    export_table: ExportTable,
}

impl LoadedModule {
    /// Get the module header
    pub fn header(&self) -> &DpmHeader {
        &self.header
    }

    /// Get a symbol by name with O(1) lookup
    pub fn get_symbol(&self, name: &str) -> Option<&ExportEntry> {
        self.export_table.get(name)
    }

    /// Get the raw bytes of a section
    pub fn get_section(&self, offset: u32, size: usize) -> Option<&[u8]> {
        let start = offset as usize;
        let end = start.checked_add(size)?;
        if end <= self.mmap.len() {
            Some(&self.mmap[start..end])
        } else {
            None
        }
    }

    /// Get the initialization bytecode
    pub fn get_init_bytecode(&self) -> Option<&[u8]> {
        if self.header.init_bytecode_offset == 0 {
            return None;
        }
        // Read size from first 4 bytes at offset
        let offset = self.header.init_bytecode_offset as usize;
        if offset + 4 > self.mmap.len() {
            return None;
        }
        let size = u32::from_le_bytes([
            self.mmap[offset],
            self.mmap[offset + 1],
            self.mmap[offset + 2],
            self.mmap[offset + 3],
        ]) as usize;
        self.get_section(self.header.init_bytecode_offset + 4, size)
    }

    /// Get imports
    pub fn get_imports(&self) -> Vec<ImportEntry> {
        if self.header.imports_count == 0 {
            return Vec::new();
        }

        let offset = self.header.imports_offset as usize;
        let entry_size = std::mem::size_of::<ImportEntry>();
        let mut imports = Vec::with_capacity(self.header.imports_count as usize);

        for i in 0..self.header.imports_count as usize {
            let start = offset + i * entry_size;
            if start + entry_size <= self.mmap.len() {
                // Safety: ImportEntry is repr(C) and we're reading from aligned offset
                let entry = unsafe {
                    std::ptr::read_unaligned(self.mmap[start..].as_ptr() as *const ImportEntry)
                };
                imports.push(entry);
            }
        }

        imports
    }
}

/// DPM Loader with module caching
pub struct DpmLoader {
    /// Cache of loaded modules
    cache: DashMap<String, Arc<LoadedModule>>,
}

impl DpmLoader {
    /// Create a new DPM loader
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Load a module from a file path
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Arc<LoadedModule>, DpmError> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Check cache first
        if let Some(module) = self.cache.get(&path_str) {
            return Ok(Arc::clone(&module));
        }

        // Load from file
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Parse header
        if mmap.len() < DpmHeader::size() {
            return Err(DpmError::InvalidOffset);
        }

        let header: DpmHeader =
            unsafe { std::ptr::read_unaligned(mmap.as_ptr() as *const DpmHeader) };
        header.validate()?;

        // Verify content hash
        let content_start = DpmHeader::size();
        if content_start < mmap.len() {
            let computed_hash = blake3::hash(&mmap[content_start..]);
            if computed_hash.as_bytes() != &header.content_hash {
                return Err(DpmError::HashMismatch);
            }
        }

        // Build export table from the file
        let export_table = Self::load_export_table(&mmap, &header)?;

        let module = Arc::new(LoadedModule {
            mmap,
            header,
            export_table,
        });

        // Cache the module
        self.cache.insert(path_str, Arc::clone(&module));

        Ok(module)
    }

    /// Load export table from memory-mapped data
    fn load_export_table(mmap: &Mmap, header: &DpmHeader) -> Result<ExportTable, DpmError> {
        if header.exports_count == 0 {
            return Ok(ExportTable::new());
        }

        let offset = header.exports_offset as usize;
        let entry_size = std::mem::size_of::<ExportEntry>();
        let mut exports = Vec::with_capacity(header.exports_count as usize);

        for i in 0..header.exports_count as usize {
            let start = offset + i * entry_size;
            if start + entry_size > mmap.len() {
                return Err(DpmError::InvalidOffset);
            }

            let entry: ExportEntry =
                unsafe { std::ptr::read_unaligned(mmap[start..].as_ptr() as *const ExportEntry) };

            // Read the name string
            let name_offset = entry.name_offset as usize;
            if name_offset >= mmap.len() {
                return Err(DpmError::InvalidOffset);
            }

            // Find null terminator
            let name_end = mmap[name_offset..]
                .iter()
                .position(|&b| b == 0)
                .map(|p| name_offset + p)
                .unwrap_or(mmap.len());

            let name = String::from_utf8_lossy(&mmap[name_offset..name_end]).to_string();
            exports.push((name, entry.kind, entry.value_offset));
        }

        ExportTable::build(&exports)
    }

    /// Get a cached module by path
    pub fn get_cached<P: AsRef<Path>>(&self, path: P) -> Option<Arc<LoadedModule>> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        self.cache.get(&path_str).map(|r| Arc::clone(&r))
    }

    /// Clear the module cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get the number of cached modules
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for DpmLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = DpmLoader::new();
        assert_eq!(loader.cache_size(), 0);
    }
}
