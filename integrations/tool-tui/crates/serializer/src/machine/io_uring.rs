//! Linux io_uring async I/O implementation
//!
//! This module provides high-performance async I/O using Linux's io_uring
//! interface (kernel 5.1+). io_uring provides true async I/O with minimal
//! syscall overhead through a shared ring buffer between user and kernel space.

use std::io;
use std::path::Path;

use super::AsyncFileIO;

#[cfg(feature = "async-io")]
use io_uring::{opcode, types, IoUring};

/// io_uring-based async I/O implementation for Linux
///
/// Uses io_uring for high-performance file operations with minimal
/// syscall overhead. Falls back to blocking I/O if io_uring is not
/// available on the system.
pub struct IoUringIO {
    #[cfg(feature = "async-io")]
    ring: IoUring,
    #[cfg(not(feature = "async-io"))]
    _phantom: std::marker::PhantomData<()>,
}

impl IoUringIO {
    /// Create a new io_uring I/O instance
    ///
    /// # Errors
    ///
    /// Returns an error if io_uring is not supported on this system
    /// (requires Linux kernel 5.1+).
    #[cfg(feature = "async-io")]
    pub fn new() -> io::Result<Self> {
        let ring = IoUring::new(256)?;
        Ok(Self { ring })
    }

    #[cfg(not(feature = "async-io"))]
    pub fn new() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "io_uring support not compiled (enable 'async-io' feature)",
        ))
    }

    /// Check if io_uring is available on this system
    pub fn is_available() -> bool {
        #[cfg(feature = "async-io")]
        {
            // Try to create a small ring to test availability
            IoUring::new(8).is_ok()
        }
        #[cfg(not(feature = "async-io"))]
        {
            false
        }
    }
}

impl AsyncFileIO for IoUringIO {
    fn read_sync(&self, path: &Path) -> io::Result<Vec<u8>> {
        // For now, use blocking read as a simple implementation
        // A full async implementation would use the ring buffer
        std::fs::read(path)
    }

    fn write_sync(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, data)
    }

    fn read_batch_sync(&self, paths: &[&Path]) -> io::Result<Vec<io::Result<Vec<u8>>>> {
        // TODO: Implement true batch read using io_uring submission queue
        // For now, fall back to sequential reads
        Ok(paths.iter().map(|p| std::fs::read(p)).collect())
    }

    fn write_batch_sync(&self, files: &[(&Path, &[u8])]) -> io::Result<Vec<io::Result<()>>> {
        // TODO: Implement true batch write using io_uring submission queue
        // For now, fall back to sequential writes
        Ok(files
            .iter()
            .map(|(path, data)| {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                std::fs::write(path, data)
            })
            .collect())
    }

    fn backend_name(&self) -> &'static str {
        "io_uring"
    }

    fn is_available(&self) -> bool {
        Self::is_available()
    }
}

#[cfg(all(test, feature = "async-io"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_uring_availability() {
        // This test just checks if we can detect io_uring availability
        let available = IoUringIO::is_available();
        println!("io_uring available: {}", available);
    }

    #[test]
    fn test_uring_read_write() {
        if !IoUringIO::is_available() {
            println!("Skipping test: io_uring not available");
            return;
        }

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let io = IoUringIO::new().unwrap();

        let data = b"Hello from io_uring!";
        io.write_sync(&path, data).unwrap();

        let read_data = io.read_sync(&path).unwrap();
        assert_eq!(read_data, data);
    }
}
