//! Windows IOCP (I/O Completion Ports) async I/O implementation
//!
//! This module provides high-performance async I/O using Windows' I/O
//! Completion Ports (IOCP). IOCP is the most efficient async I/O mechanism
//! on Windows, used by high-performance servers and applications.

use std::io;
use std::path::Path;

use super::AsyncFileIO;

#[cfg(all(target_os = "windows", feature = "async-io"))]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::IO::CreateIoCompletionPort,
};

/// IOCP-based async I/O implementation for Windows
///
/// Uses I/O Completion Ports for high-performance file operations.
/// IOCP provides excellent scalability for concurrent I/O operations.
///
/// Note: The current implementation uses blocking I/O as a fallback.
/// A full async implementation would use overlapped I/O with IOCP.
pub struct IocpIO {
    #[cfg(all(target_os = "windows", feature = "async-io"))]
    completion_port: HANDLE,
    #[cfg(not(all(target_os = "windows", feature = "async-io")))]
    _phantom: std::marker::PhantomData<()>,
}

#[cfg(all(target_os = "windows", feature = "async-io"))]
// SAFETY: IocpIO can be safely sent between threads because:
// 1. The completion_port HANDLE is a Windows kernel object handle
// 2. Windows IOCP handles are designed for multi-threaded access
// 3. We only perform thread-safe operations: creation and closing
// 4. The Windows API documentation explicitly states IOCP is thread-safe
unsafe impl Send for IocpIO {}

#[cfg(all(target_os = "windows", feature = "async-io"))]
// SAFETY: IocpIO can be safely shared between threads because:
// 1. IOCP handles support concurrent access from multiple threads
// 2. GetQueuedCompletionStatus/PostQueuedCompletionStatus are thread-safe
// 3. We don't expose any mutable state through shared references
// 4. All operations on the HANDLE are atomic at the kernel level
unsafe impl Sync for IocpIO {}

impl IocpIO {
    /// Create a new IOCP I/O instance
    ///
    /// # Errors
    ///
    /// Returns an error if IOCP creation fails.
    #[cfg(all(target_os = "windows", feature = "async-io"))]
    pub fn new() -> io::Result<Self> {
        // SAFETY: CreateIoCompletionPort is a Windows API function that creates an IOCP handle.
        // We pass INVALID_HANDLE_VALUE to create a new completion port not associated with a file.
        // The function returns a valid HANDLE or null on failure, which we check.
        unsafe {
            let completion_port =
                CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 0);
            if completion_port.is_null() {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { completion_port })
        }
    }

    #[cfg(not(all(target_os = "windows", feature = "async-io")))]
    pub fn new() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "IOCP support not compiled (enable 'async-io' feature on Windows)",
        ))
    }

    /// Check if IOCP is available (always true on Windows with feature enabled)
    pub fn is_available() -> bool {
        #[cfg(all(target_os = "windows", feature = "async-io"))]
        {
            true
        }
        #[cfg(not(all(target_os = "windows", feature = "async-io")))]
        {
            false
        }
    }
}

#[cfg(all(target_os = "windows", feature = "async-io"))]
impl Drop for IocpIO {
    fn drop(&mut self) {
        // SAFETY: CloseHandle is a Windows API function that closes a HANDLE.
        // We check that completion_port is not null before calling CloseHandle.
        // After this call, the HANDLE is invalid, but we're in Drop so the struct is being destroyed.
        unsafe {
            if !self.completion_port.is_null() {
                CloseHandle(self.completion_port);
            }
        }
    }
}

impl AsyncFileIO for IocpIO {
    fn read_sync(&self, path: &Path) -> io::Result<Vec<u8>> {
        // For now, use blocking read
        // A full async implementation would use overlapped I/O with IOCP
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
        // TODO: Implement true batch read using IOCP
        Ok(paths.iter().copied().map(std::fs::read).collect())
    }

    fn write_batch_sync(&self, files: &[(&Path, &[u8])]) -> io::Result<Vec<io::Result<()>>> {
        // TODO: Implement true batch write using IOCP
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
        "iocp"
    }

    fn is_available(&self) -> bool {
        Self::is_available()
    }
}

#[cfg(all(test, target_os = "windows", feature = "async-io"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_iocp_read_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let io = IocpIO::new().unwrap();

        let data = b"Hello from IOCP!";
        io.write_sync(&path, data).unwrap();

        let read_data = io.read_sync(&path).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_iocp_backend_name() {
        let io = IocpIO::new().unwrap();
        assert_eq!(io.backend_name(), "iocp");
        assert!(io.is_available());
    }
}
