//! DPP (Dx Python Package) format implementation
//!
//! Provides zero-copy access to DPP packages via memory mapping.

use dx_py_core::{
    headers::{DppHeader, DppMetadata},
    DPP_MAGIC, MAX_PACKAGE_SIZE, PROTOCOL_VERSION,
};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

use crate::{Error, Result};

/// Zero-copy DPP package access
///
/// DppPackage provides memory-mapped access to DPP format packages,
/// enabling O(1) metadata retrieval without parsing.
pub struct DppPackage {
    /// Memory-mapped file contents
    mmap: Mmap,
    /// Cached header (copied for safe access to packed fields)
    header: DppHeader,
}

impl DppPackage {
    /// Open a DPP package with memory mapping (zero-copy)
    ///
    /// # Arguments
    /// * `path` - Path to the DPP package file
    ///
    /// # Returns
    /// * `Ok(DppPackage)` - Successfully opened package
    /// * `Err(Error)` - Failed to open or validate package
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let metadata = file.metadata()?;

        // Check file size limits
        if metadata.len() > MAX_PACKAGE_SIZE {
            return Err(Error::PackageTooLarge {
                size: metadata.len(),
                limit: MAX_PACKAGE_SIZE,
            });
        }

        // Memory map the file
        let mmap = unsafe { Mmap::map(&file)? };

        // Verify minimum size for header
        if mmap.len() < std::mem::size_of::<DppHeader>() {
            return Err(Error::InvalidMagic {
                expected: *DPP_MAGIC,
                found: [0, 0, 0, 0],
            });
        }

        // Verify magic number
        if &mmap[0..4] != DPP_MAGIC {
            let mut found = [0u8; 4];
            found.copy_from_slice(&mmap[0..4]);
            return Err(Error::InvalidMagic {
                expected: *DPP_MAGIC,
                found,
            });
        }

        // Zero-copy header access via bytemuck
        let header: DppHeader = *bytemuck::from_bytes(&mmap[0..64]);

        // Verify protocol version
        if header.version != PROTOCOL_VERSION {
            return Err(Error::UnsupportedVersion(header.version));
        }

        // Verify integrity (BLAKE3 hash of content after header)
        let content_start = std::mem::size_of::<DppHeader>();
        let content_end = header.total_size as usize;

        if content_end > mmap.len() {
            return Err(Error::IntegrityError);
        }

        let content = &mmap[content_start..content_end];
        let computed = blake3::hash(content);
        let computed_truncated = &computed.as_bytes()[..20];

        if computed_truncated != header.blake3_hash {
            return Err(Error::IntegrityError);
        }

        Ok(Self { mmap, header })
    }

    /// Get the package header
    #[inline]
    pub fn header(&self) -> &DppHeader {
        &self.header
    }

    /// Get the BLAKE3 hash of the package content
    #[inline]
    pub fn content_hash(&self) -> &[u8; 20] {
        &self.header.blake3_hash
    }

    /// Get the total package size
    #[inline]
    pub fn total_size(&self) -> u64 {
        self.header.total_size
    }

    /// Get the uncompressed size
    #[inline]
    pub fn uncompressed_size(&self) -> u64 {
        self.header.uncompressed_size
    }

    /// O(1) metadata access - no parsing!
    ///
    /// Returns a reference to the metadata section header.
    #[inline]
    pub fn metadata(&self) -> &DppMetadata {
        let offset = self.header.metadata_offset as usize;
        bytemuck::from_bytes(&self.mmap[offset..offset + std::mem::size_of::<DppMetadata>()])
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        let meta = self.metadata();
        let offset = self.header.metadata_offset as usize + meta.name_offset();
        let len = meta.name_len as usize;
        std::str::from_utf8(&self.mmap[offset..offset + len]).unwrap_or("")
    }

    /// Get the package version
    pub fn version(&self) -> &str {
        let meta = self.metadata();
        let offset = self.header.metadata_offset as usize + meta.version_offset();
        let len = meta.version_len as usize;
        std::str::from_utf8(&self.mmap[offset..offset + len]).unwrap_or("")
    }

    /// Get the python_requires constraint
    pub fn python_requires(&self) -> &str {
        let meta = self.metadata();
        let offset = self.header.metadata_offset as usize + meta.python_requires_offset();
        let len = meta.python_requires_len as usize;
        if len == 0 {
            return "";
        }
        std::str::from_utf8(&self.mmap[offset..offset + len]).unwrap_or("")
    }

    /// Get the pre-compiled bytecode section
    pub fn bytecode(&self) -> &[u8] {
        let start = self.header.bytecode_offset as usize;
        let end = self.header.native_offset as usize;
        if start >= end || end > self.mmap.len() {
            return &[];
        }
        &self.mmap[start..end]
    }

    /// Get the native extensions section
    pub fn native(&self) -> &[u8] {
        let start = self.header.native_offset as usize;
        let end = self.header.deps_offset as usize;
        if start >= end || end > self.mmap.len() {
            return &[];
        }
        &self.mmap[start..end]
    }

    /// Get the dependency graph section
    pub fn deps(&self) -> &[u8] {
        let start = self.header.deps_offset as usize;
        let end = self.header.total_size as usize;
        if start >= end || end > self.mmap.len() {
            return &[];
        }
        &self.mmap[start..end]
    }

    /// Get raw access to the memory-mapped data
    pub fn raw_data(&self) -> &[u8] {
        &self.mmap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpp_header_size() {
        assert_eq!(std::mem::size_of::<DppHeader>(), 64);
    }
}
