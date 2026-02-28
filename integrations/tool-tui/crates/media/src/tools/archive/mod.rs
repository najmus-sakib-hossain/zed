//! Archive and compression tools.
//!
//! This module provides 10 archive manipulation tools:
//! 1. Zip Creator - Create ZIP archives
//! 2. Zip Extractor - Extract ZIP archives
//! 3. Tar Creator - Create TAR archives
//! 4. Tar Extractor - Extract TAR archives
//! 5. Compressor - Compress files (gzip, bzip2, xz)
//! 6. Decompressor - Decompress files
//! 7. Archive List - List archive contents
//! 8. Archive Encrypt - Encrypt archives
//! 9. Archive Split - Split large archives
//! 10. Archive Merge - Merge split archives
//!
//! ## Native Processing
//!
//! Enable the `archive-core` feature for native Rust archive processing
//! without external tools. Uses `zip`, `tar`, and `flate2` crates.

pub mod compress;
pub mod decompress;
pub mod encrypt;
pub mod list;
pub mod merge;
pub mod native;
pub mod rar;
pub mod sevenz;
pub mod split;
pub mod tar;
pub mod zip;

pub use compress::*;
pub use decompress::*;
pub use encrypt::*;
pub use list::*;
pub use merge::*;
pub use native::*;
pub use rar::*;
pub use sevenz::*;
pub use split::*;
pub use tar::*;
pub use zip::*;

use crate::error::Result;
use std::path::Path;

/// Archive tools collection.
pub struct ArchiveTools;

impl ArchiveTools {
    /// Create a new ArchiveTools instance.
    pub fn new() -> Self {
        Self
    }

    /// Create ZIP archive.
    pub fn create_zip<P: AsRef<Path>>(&self, inputs: &[P], output: P) -> Result<super::ToolOutput> {
        zip::create_zip(inputs, output)
    }

    /// Extract ZIP archive.
    pub fn extract_zip<P: AsRef<Path>>(
        &self,
        input: P,
        output_dir: P,
    ) -> Result<super::ToolOutput> {
        zip::extract_zip(input, output_dir)
    }

    /// Create TAR archive.
    pub fn create_tar<P: AsRef<Path>>(&self, inputs: &[P], output: P) -> Result<super::ToolOutput> {
        tar::create_tar(inputs, output)
    }

    /// Extract TAR archive.
    pub fn extract_tar<P: AsRef<Path>>(
        &self,
        input: P,
        output_dir: P,
    ) -> Result<super::ToolOutput> {
        tar::extract_tar(input, output_dir)
    }

    /// Compress file with gzip.
    pub fn gzip<P: AsRef<Path>>(&self, input: P, output: P) -> Result<super::ToolOutput> {
        compress::gzip(input, output)
    }

    /// Decompress gzip file.
    pub fn gunzip<P: AsRef<Path>>(&self, input: P, output: P) -> Result<super::ToolOutput> {
        decompress::gunzip(input, output)
    }

    /// List archive contents.
    pub fn list<P: AsRef<Path>>(&self, input: P) -> Result<super::ToolOutput> {
        list::list_archive(input)
    }

    /// Create encrypted archive.
    pub fn encrypt_archive<P: AsRef<Path>>(
        &self,
        inputs: &[P],
        output: P,
        password: &str,
    ) -> Result<super::ToolOutput> {
        encrypt::create_encrypted_zip(inputs, output, password)
    }

    /// Split archive into parts.
    pub fn split_archive<P: AsRef<Path>>(
        &self,
        input: P,
        output_dir: P,
        part_size_mb: u64,
    ) -> Result<super::ToolOutput> {
        split::split_archive(input, output_dir, part_size_mb)
    }

    /// Merge split archives.
    pub fn merge_archives<P: AsRef<Path>>(
        &self,
        parts: &[P],
        output: P,
    ) -> Result<super::ToolOutput> {
        merge::merge_archives(parts, output)
    }
}

impl Default for ArchiveTools {
    fn default() -> Self {
        Self::new()
    }
}
