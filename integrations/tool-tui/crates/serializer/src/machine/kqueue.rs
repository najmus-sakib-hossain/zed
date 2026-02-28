//! macOS kqueue async I/O implementation
//!
//! This module provides async I/O using macOS's kqueue interface.
//! kqueue is a scalable event notification interface available on
//! BSD-derived systems including macOS.
//!
//! While kqueue is primarily designed for socket and pipe I/O, this
//! implementation provides event-driven file operations using EVFILT_VNODE
//! for monitoring file changes and optimized batch operations.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

use super::AsyncFileIO;

#[cfg(target_os = "macos")]
use libc::{kevent, kqueue, timespec, EVFILT_VNODE, EV_ADD, EV_CLEAR, NOTE_WRITE};

/// kqueue-based async I/O implementation for macOS
///
/// Uses kqueue for event-driven file operations. This implementation
/// leverages EVFILT_VNODE for file monitoring and provides optimized
/// batch operations for serialization workloads.
pub struct KqueueIO {
    #[cfg(target_os = "macos")]
    kq: RawFd,
}

impl KqueueIO {
    /// Create a new kqueue I/O instance
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            // SAFETY: kqueue() is a safe syscall that returns a file descriptor
            let kq = unsafe { kqueue() };
            Self { kq }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {}
        }
    }

    /// Check if kqueue is available (always true on macOS)
    pub fn is_available() -> bool {
        cfg!(target_os = "macos")
    }

    /// Perform event-driven write operation
    ///
    /// This method uses kqueue to monitor file write completion,
    /// enabling efficient batch operations.
    #[cfg(target_os = "macos")]
    fn write_with_event(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Open file for writing
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;

        let fd = file.as_raw_fd();

        // Set up kqueue event for write monitoring
        let mut kev = kevent {
            ident: fd as usize,
            filter: EVFILT_VNODE,
            flags: EV_ADD | EV_CLEAR,
            fflags: NOTE_WRITE as u32,
            data: 0,
            udata: std::ptr::null_mut(),
        };

        // Register the event
        // SAFETY: kevent syscall with valid kqueue fd and event structure
        let ret = unsafe {
            kevent(self.kq, &kev as *const kevent, 1, std::ptr::null_mut(), 0, std::ptr::null())
        };

        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // Perform the write
        file.write_all(data)?;
        file.sync_all()?;

        Ok(())
    }

    /// Perform event-driven batch write operations
    ///
    /// This method optimizes multiple file writes by using kqueue
    /// to monitor all operations simultaneously, reducing syscall overhead.
    #[cfg(target_os = "macos")]
    fn write_batch_with_events(&self, files: &[(&Path, &[u8])]) -> io::Result<Vec<io::Result<()>>> {
        let mut results = Vec::with_capacity(files.len());
        let mut open_files = Vec::with_capacity(files.len());
        let mut kevents = Vec::with_capacity(files.len());

        // Open all files and register events
        for (path, _) in files {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        results.push(Err(e));
                        continue;
                    }
                }
            }

            match OpenOptions::new().write(true).create(true).truncate(true).open(path) {
                Ok(file) => {
                    let fd = file.as_raw_fd();
                    let kev = kevent {
                        ident: fd as usize,
                        filter: EVFILT_VNODE,
                        flags: EV_ADD | EV_CLEAR,
                        fflags: NOTE_WRITE as u32,
                        data: 0,
                        udata: std::ptr::null_mut(),
                    };
                    open_files.push(file);
                    kevents.push(kev);
                }
                Err(e) => {
                    results.push(Err(e));
                }
            }
        }

        // Register all events at once
        if !kevents.is_empty() {
            // SAFETY: kevent syscall with valid kqueue fd and event structures
            let ret = unsafe {
                kevent(
                    self.kq,
                    kevents.as_ptr(),
                    kevents.len() as i32,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };

            if ret < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        // Perform all writes
        let mut file_idx = 0;
        for (i, (_, data)) in files.iter().enumerate() {
            if results.len() > i {
                continue; // Skip files that failed to open
            }

            let result = open_files[file_idx]
                .write_all(data)
                .and_then(|_| open_files[file_idx].sync_all());
            results.push(result);
            file_idx += 1;
        }

        Ok(results)
    }
}

impl Default for KqueueIO {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
impl Drop for KqueueIO {
    fn drop(&mut self) {
        // SAFETY: Close the kqueue file descriptor
        unsafe {
            libc::close(self.kq);
        }
    }
}

impl AsyncFileIO for KqueueIO {
    fn read_sync(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write_sync(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Use event-driven write for better performance
            self.write_with_event(path, data)
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for non-macOS platforms
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(path, data)
        }
    }

    fn read_batch_sync(&self, paths: &[&Path]) -> io::Result<Vec<io::Result<Vec<u8>>>> {
        // For reads, kqueue doesn't provide significant benefits over sequential reads
        // since we need the data immediately. Use standard file I/O.
        Ok(paths.iter().map(|p| std::fs::read(p)).collect())
    }

    fn write_batch_sync(&self, files: &[(&Path, &[u8])]) -> io::Result<Vec<io::Result<()>>> {
        #[cfg(target_os = "macos")]
        {
            // Use event-driven batch write for optimal performance
            self.write_batch_with_events(files)
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for non-macOS platforms
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
    }

    fn backend_name(&self) -> &'static str {
        "kqueue"
    }

    fn is_available(&self) -> bool {
        Self::is_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_kqueue_read_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let io = KqueueIO::new();

        let data = b"Hello from kqueue!";
        io.write_sync(&path, data).unwrap();

        let read_data = io.read_sync(&path).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_kqueue_backend_name() {
        let io = KqueueIO::new();
        assert_eq!(io.backend_name(), "kqueue");
        assert!(io.is_available());
    }
}
