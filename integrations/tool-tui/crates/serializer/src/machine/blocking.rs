//! Blocking I/O fallback implementation
//!
//! This module provides a standard blocking I/O implementation that works
//! on all platforms. It's used as a fallback when platform-specific async
//! I/O is not available.

use std::fs;
use std::io;
use std::path::Path;

use super::AsyncFileIO;

/// Blocking I/O implementation using standard library
///
/// This is the fallback implementation that works on all platforms.
/// It uses synchronous file operations from `std::fs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlockingIO;

impl BlockingIO {
    /// Create a new blocking I/O instance
    pub fn new() -> Self {
        Self
    }
}

impl AsyncFileIO for BlockingIO {
    fn read_sync(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write_sync(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, data)
    }

    fn read_batch_sync(&self, paths: &[&Path]) -> io::Result<Vec<io::Result<Vec<u8>>>> {
        Ok(paths.iter().copied().map(fs::read).collect())
    }

    fn write_batch_sync(&self, files: &[(&Path, &[u8])]) -> io::Result<Vec<io::Result<()>>> {
        Ok(files
            .iter()
            .map(|(path, data)| {
                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                fs::write(path, data)
            })
            .collect())
    }

    fn backend_name(&self) -> &'static str {
        "blocking"
    }

    fn is_available(&self) -> bool {
        true // Always available
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(test)]
    // use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_blocking_read_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let io = BlockingIO::new();

        let data = b"Hello, World!";
        io.write_sync(&path, data).unwrap();

        let read_data = io.read_sync(&path).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_blocking_batch_operations() {
        let dir = TempDir::new().unwrap();
        let io = BlockingIO::new();

        // Write batch
        let files: Vec<_> = (0..3)
            .map(|i| {
                let path = dir.path().join(format!("file{}.txt", i));
                let data = format!("Content {}", i);
                (path, data)
            })
            .collect();

        let write_refs: Vec<_> = files.iter().map(|(p, d)| (p.as_path(), d.as_bytes())).collect();
        let write_results = io.write_batch_sync(&write_refs).unwrap();
        assert!(write_results.iter().all(|r| r.is_ok()));

        // Read batch
        let paths: Vec<_> = files.iter().map(|(p, _)| p.as_path()).collect();
        let read_results = io.read_batch_sync(&paths).unwrap();

        for (i, result) in read_results.iter().enumerate() {
            let data = result.as_ref().unwrap();
            assert_eq!(String::from_utf8_lossy(data), format!("Content {}", i));
        }
    }

    #[test]
    fn test_blocking_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("test.txt");
        let io = BlockingIO::new();

        io.write_sync(&path, b"test").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_blocking_backend_name() {
        let io = BlockingIO::new();
        assert_eq!(io.backend_name(), "blocking");
        assert!(io.is_available());
    }
}
