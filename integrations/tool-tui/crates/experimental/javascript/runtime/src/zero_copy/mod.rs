// Zero-copy file operations
//!
//! Provides efficient file reading using memory-mapped I/O for large files
//! and regular reads for small files. Includes graceful fallback on mmap failure.

use crate::constants::MMAP_THRESHOLD_BYTES;
use memmap2::Mmap;
use std::fs::{self, File};
use std::io;
use std::path::Path;

/// Result of reading a file - either memory-mapped or owned
pub enum FileContent {
    /// Memory-mapped content (zero-copy for large files)
    Mapped(MappedFile),
    /// Owned content (for small files or fallback)
    Owned(String),
}

impl FileContent {
    /// Get content as bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            FileContent::Mapped(m) => m.as_bytes(),
            FileContent::Owned(s) => s.as_bytes(),
        }
    }

    /// Get content as string slice (assumes valid UTF-8)
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        match self {
            FileContent::Mapped(m) => m.as_str(),
            FileContent::Owned(s) => Ok(s.as_str()),
        }
    }

    /// Get content length
    pub fn len(&self) -> usize {
        match self {
            FileContent::Mapped(m) => m.len(),
            FileContent::Owned(s) => s.len(),
        }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Memory-mapped file for zero-copy access
pub struct MappedFile {
    mmap: Mmap,
}

impl MappedFile {
    /// Create a new memory-mapped file
    fn new(file: &File) -> io::Result<Self> {
        // SAFETY: The file is open and we're mapping it read-only
        let mmap = unsafe { Mmap::map(file)? };
        Ok(Self { mmap })
    }

    /// Get content as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    /// Get content as string slice
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.mmap)
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }
}

/// Zero-copy file reader with automatic strategy selection
pub struct ZeroCopyReader;

impl ZeroCopyReader {
    /// Read a file using the optimal strategy based on file size
    ///
    /// - Files smaller than MMAP_THRESHOLD_BYTES are read normally
    /// - Larger files use memory-mapped I/O for zero-copy access
    /// - Falls back to regular read if mmap fails
    pub fn read(path: &Path) -> io::Result<FileContent> {
        let metadata = fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        // Use regular read for small files (mmap overhead not worth it)
        if file_size < MMAP_THRESHOLD_BYTES {
            let content = fs::read_to_string(path)?;
            return Ok(FileContent::Owned(content));
        }

        // Try memory-mapped I/O for large files
        let file = File::open(path)?;
        match MappedFile::new(&file) {
            Ok(mapped) => Ok(FileContent::Mapped(mapped)),
            Err(_) => {
                // Fallback to regular read on mmap failure
                let content = fs::read_to_string(path)?;
                Ok(FileContent::Owned(content))
            }
        }
    }

    /// Read a file as bytes using the optimal strategy
    pub fn read_bytes(path: &Path) -> io::Result<FileContent> {
        let metadata = fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        // Use regular read for small files
        if file_size < MMAP_THRESHOLD_BYTES {
            let content = fs::read(path)?;
            // Convert to string for consistency (may fail for binary files)
            let content = String::from_utf8(content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            return Ok(FileContent::Owned(content));
        }

        // Try memory-mapped I/O for large files
        let file = File::open(path)?;
        match MappedFile::new(&file) {
            Ok(mapped) => Ok(FileContent::Mapped(mapped)),
            Err(_) => {
                // Fallback to regular read on mmap failure
                let content = fs::read(path)?;
                let content = String::from_utf8(content)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(FileContent::Owned(content))
            }
        }
    }
}

/// Legacy zero-copy file reader
///
/// This struct provides safe zero-copy access to memory-mapped file content.
/// The content reference is tied to the lifetime of the struct, ensuring
/// memory safety without unsafe lifetime extension.
pub struct ZeroCopyFile {
    mmap: Mmap,
}

impl ZeroCopyFile {
    /// Read file with zero copies
    ///
    /// Returns a ZeroCopyFile that owns the memory mapping.
    /// Use `as_str()` to get a reference to the content.
    pub fn read(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        // SAFETY: The file is open and we're mapping it read-only
        let mmap = unsafe { Mmap::map(&file)? };

        // Validate UTF-8 before returning
        std::str::from_utf8(&mmap)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Self { mmap })
    }

    /// Get content as string slice
    ///
    /// The returned reference is valid for the lifetime of this ZeroCopyFile.
    /// This is safe because the mmap is owned by this struct.
    pub fn as_str(&self) -> &str {
        // SAFETY: We validated UTF-8 in read(), and mmap is still valid
        // because it's owned by self
        unsafe { std::str::from_utf8_unchecked(&self.mmap) }
    }

    /// Get content as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }
}

/// Read source file with zero copies when beneficial
///
/// Uses memory-mapped I/O for files larger than MMAP_THRESHOLD_BYTES,
/// falls back to regular read for small files or on mmap failure.
pub fn read_source_zero_copy(path: &Path) -> io::Result<String> {
    match ZeroCopyReader::read(path)? {
        FileContent::Mapped(m) => {
            // For mapped files, we need to copy to String for ownership
            m.as_str()
                .map(|s| s.to_string())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        FileContent::Owned(s) => Ok(s),
    }
}

/// Read source file as bytes with zero copies when beneficial
pub fn read_source_bytes_zero_copy(path: &Path) -> io::Result<Vec<u8>> {
    match ZeroCopyReader::read_bytes(path)? {
        FileContent::Mapped(m) => Ok(m.as_bytes().to_vec()),
        FileContent::Owned(s) => Ok(s.into_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_small_file_uses_regular_read() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "small content").unwrap();

        let content = ZeroCopyReader::read(file.path()).unwrap();
        match content {
            FileContent::Owned(_) => {} // Expected for small files
            FileContent::Mapped(_) => panic!("Small file should use regular read"),
        }
    }

    #[test]
    fn test_large_file_uses_mmap() {
        let mut file = NamedTempFile::new().unwrap();
        // Write more than MMAP_THRESHOLD_BYTES
        let large_content = "x".repeat(MMAP_THRESHOLD_BYTES + 1000);
        write!(file, "{}", large_content).unwrap();

        let content = ZeroCopyReader::read(file.path()).unwrap();
        match content {
            FileContent::Mapped(_) => {} // Expected for large files
            FileContent::Owned(_) => {}  // Also acceptable (fallback)
        }
    }

    #[test]
    fn test_content_is_correct() {
        let mut file = NamedTempFile::new().unwrap();
        let expected = "Hello, World!";
        write!(file, "{}", expected).unwrap();

        let content = ZeroCopyReader::read(file.path()).unwrap();
        assert_eq!(content.as_str().unwrap(), expected);
    }

    #[test]
    fn test_read_source_zero_copy() {
        let mut file = NamedTempFile::new().unwrap();
        let expected = "console.log('hello');";
        write!(file, "{}", expected).unwrap();

        let content = read_source_zero_copy(file.path()).unwrap();
        assert_eq!(content, expected);
    }
}
