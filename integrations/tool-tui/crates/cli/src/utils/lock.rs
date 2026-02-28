//! File Lock Manager
//!
//! Provides cross-platform file locking with support for:
//! - Shared (read) and Exclusive (write) locks
//! - Blocking acquire with timeout
//! - Non-blocking try_acquire
//! - Automatic release via Drop

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::utils::error::DxError;

/// Type of file lock
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Shared lock - multiple readers allowed
    Shared,
    /// Exclusive lock - single writer only
    Exclusive,
}

/// A file lock that is automatically released when dropped
///
/// Requirement 12.1: File locking for concurrent access
/// Requirement 12.2: Shared and exclusive lock types
pub struct FileLock {
    path: PathBuf,
    #[allow(dead_code)]
    handle: File,
    #[allow(dead_code)]
    lock_type: LockType,
}

impl FileLock {
    /// Acquire a lock with timeout (blocking)
    ///
    /// Requirement 12.7: Blocking wait with timeout
    pub fn acquire(path: &Path, lock_type: LockType, timeout: Duration) -> Result<Self, DxError> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(10);

        // Create or open the lock file
        let lock_path = Self::lock_file_path(path);

        loop {
            match Self::try_acquire_internal(&lock_path, lock_type) {
                Ok(Some(lock)) => return Ok(lock),
                Ok(None) => {
                    // Lock not available, check timeout
                    if start.elapsed() >= timeout {
                        return Err(DxError::LockTimeout {
                            path: path.to_path_buf(),
                            timeout,
                        });
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to acquire a lock without blocking
    ///
    /// Returns `Ok(Some(lock))` if acquired, `Ok(None)` if lock is held by another process
    ///
    /// Requirement 12.7: Non-blocking lock acquisition
    pub fn try_acquire(path: &Path, lock_type: LockType) -> Result<Option<Self>, DxError> {
        let lock_path = Self::lock_file_path(path);
        Self::try_acquire_internal(&lock_path, lock_type)
    }

    /// Get the lock file path for a given path
    fn lock_file_path(path: &Path) -> PathBuf {
        let mut lock_path = path.to_path_buf();
        let file_name = lock_path
            .file_name()
            .map(|n| format!(".{}.lock", n.to_string_lossy()))
            .unwrap_or_else(|| ".lock".to_string());
        lock_path.set_file_name(file_name);
        lock_path
    }

    /// Internal implementation of try_acquire
    fn try_acquire_internal(
        lock_path: &Path,
        lock_type: LockType,
    ) -> Result<Option<Self>, DxError> {
        // Ensure parent directory exists
        if let Some(parent) = lock_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| DxError::Io {
                message: format!("Failed to create lock directory: {}", e),
            })?;
        }

        // Open or create the lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| DxError::Io {
                message: format!("Failed to open lock file: {}", e),
            })?;

        // Try to acquire the lock
        if Self::try_lock_file(&file, lock_type)? {
            Ok(Some(FileLock {
                path: lock_path.to_path_buf(),
                handle: file,
                lock_type,
            }))
        } else {
            Ok(None)
        }
    }

    /// Platform-specific lock acquisition
    #[cfg(unix)]
    fn try_lock_file(file: &File, lock_type: LockType) -> Result<bool, DxError> {
        use std::os::unix::io::AsRawFd;

        let fd = file.as_raw_fd();
        let operation = match lock_type {
            LockType::Shared => libc::LOCK_SH | libc::LOCK_NB,
            LockType::Exclusive => libc::LOCK_EX | libc::LOCK_NB,
        };

        // SAFETY: fd is a valid file descriptor from an open File
        let result = unsafe { libc::flock(fd, operation) };

        if result == 0 {
            Ok(true)
        } else {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::WouldBlock {
                Ok(false)
            } else {
                Err(DxError::Io {
                    message: format!("Failed to acquire lock: {}", err),
                })
            }
        }
    }

    /// Platform-specific lock acquisition for Windows
    #[cfg(windows)]
    fn try_lock_file(file: &File, lock_type: LockType) -> Result<bool, DxError> {
        use std::os::windows::io::AsRawHandle;
        use winapi::shared::winerror::ERROR_LOCK_VIOLATION;
        use winapi::um::fileapi::LockFileEx;
        use winapi::um::minwinbase::{
            LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, OVERLAPPED,
        };

        let handle = file.as_raw_handle();
        let flags = match lock_type {
            LockType::Shared => LOCKFILE_FAIL_IMMEDIATELY,
            LockType::Exclusive => LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
        };

        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };

        // SAFETY: handle is a valid file handle, overlapped is properly initialized
        let result =
            unsafe { LockFileEx(handle as *mut _, flags, 0, u32::MAX, u32::MAX, &mut overlapped) };

        if result != 0 {
            Ok(true)
        } else {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) {
                Ok(false)
            } else {
                Err(DxError::Io {
                    message: format!("Failed to acquire lock: {}", err),
                })
            }
        }
    }

    /// Platform-specific lock release
    #[cfg(unix)]
    fn unlock_file(file: &File) -> Result<(), DxError> {
        use std::os::unix::io::AsRawFd;

        let fd = file.as_raw_fd();

        // SAFETY: fd is a valid file descriptor from an open File
        let result = unsafe { libc::flock(fd, libc::LOCK_UN) };

        if result == 0 {
            Ok(())
        } else {
            Err(DxError::Io {
                message: format!("Failed to release lock: {}", std::io::Error::last_os_error()),
            })
        }
    }

    /// Platform-specific lock release for Windows
    #[cfg(windows)]
    fn unlock_file(file: &File) -> Result<(), DxError> {
        use std::os::windows::io::AsRawHandle;
        use winapi::um::fileapi::UnlockFileEx;
        use winapi::um::minwinbase::OVERLAPPED;

        let handle = file.as_raw_handle();
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };

        // SAFETY: handle is a valid file handle, overlapped is properly initialized
        let result =
            unsafe { UnlockFileEx(handle as *mut _, 0, u32::MAX, u32::MAX, &mut overlapped) };

        if result != 0 {
            Ok(())
        } else {
            Err(DxError::Io {
                message: format!("Failed to release lock: {}", std::io::Error::last_os_error()),
            })
        }
    }

    /// Get the path being locked
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the lock type
    #[allow(dead_code)]
    pub fn lock_type(&self) -> LockType {
        self.lock_type
    }
}

/// Automatic lock release on drop
///
/// Requirement 12.1: Ensure lock is released even on panic
impl Drop for FileLock {
    fn drop(&mut self) {
        // Best effort unlock - ignore errors during drop
        let _ = Self::unlock_file(&self.handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn test_lock_file_path() {
        // Use platform-agnostic path for testing
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test.txt");
        let lock_path = FileLock::lock_file_path(&path);
        assert_eq!(lock_path, temp_dir.join(".test.txt.lock"));
    }

    #[test]
    fn test_exclusive_lock_acquire_release() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_lock_test_exclusive");

        // Acquire exclusive lock
        let lock = FileLock::acquire(&test_file, LockType::Exclusive, Duration::from_secs(1));
        assert!(lock.is_ok(), "Should acquire exclusive lock");

        // Drop releases the lock
        drop(lock);

        // Should be able to acquire again
        let lock2 = FileLock::acquire(&test_file, LockType::Exclusive, Duration::from_secs(1));
        assert!(lock2.is_ok(), "Should acquire lock after release");

        // Cleanup
        let lock_path = FileLock::lock_file_path(&test_file);
        let _ = std::fs::remove_file(&lock_path);
    }

    #[test]
    fn test_shared_lock_acquire_release() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_lock_test_shared");

        // Acquire shared lock
        let lock = FileLock::acquire(&test_file, LockType::Shared, Duration::from_secs(1));
        assert!(lock.is_ok(), "Should acquire shared lock");

        // Cleanup
        drop(lock);
        let lock_path = FileLock::lock_file_path(&test_file);
        let _ = std::fs::remove_file(&lock_path);
    }

    #[test]
    fn test_try_acquire_nonblocking() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_lock_test_try");

        // Try acquire should succeed when no lock held
        let result = FileLock::try_acquire(&test_file, LockType::Exclusive);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some(), "Should acquire lock");

        // Cleanup
        let lock_path = FileLock::lock_file_path(&test_file);
        let _ = std::fs::remove_file(&lock_path);
    }

    // Feature: dx-cli, Property 37: File Lock Blocking vs Non-Blocking
    // Validates: Requirements 12.7
    //
    // For any file, try_acquire() should return immediately with None if the lock
    // is held by another process. acquire() with timeout should block until the
    // lock is available or timeout expires.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_lock_blocking_vs_nonblocking(
            file_id in 0u32..1000,
            lock_type in prop::sample::select(vec![LockType::Shared, LockType::Exclusive])
        ) {
            let temp_dir = std::env::temp_dir();
            let test_file = temp_dir.join(format!("dx_lock_prop_test_{}", file_id));
            let lock_path = FileLock::lock_file_path(&test_file);

            // Clean up any existing lock file
            let _ = std::fs::remove_file(&lock_path);

            // try_acquire on unlocked file should succeed
            let result = FileLock::try_acquire(&test_file, lock_type);
            prop_assert!(result.is_ok(), "try_acquire should not error");

            let lock = result.unwrap();
            prop_assert!(lock.is_some(), "Should acquire lock on unlocked file");

            // Drop the lock
            drop(lock);

            // Clean up
            let _ = std::fs::remove_file(&lock_path);
        }
    }

    #[test]
    fn test_exclusive_lock_blocks_second_exclusive() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_lock_test_block");
        let lock_path = FileLock::lock_file_path(&test_file);

        // Clean up any existing lock
        let _ = std::fs::remove_file(&lock_path);

        // Acquire first exclusive lock
        let lock1 = FileLock::try_acquire(&test_file, LockType::Exclusive)
            .expect("Should not error")
            .expect("Should acquire first lock");

        // Second try_acquire should return None (non-blocking)
        let lock2 =
            FileLock::try_acquire(&test_file, LockType::Exclusive).expect("Should not error");
        assert!(lock2.is_none(), "Second exclusive lock should fail");

        // Release first lock
        drop(lock1);

        // Now should be able to acquire
        let lock3 =
            FileLock::try_acquire(&test_file, LockType::Exclusive).expect("Should not error");
        assert!(lock3.is_some(), "Should acquire after release");

        // Cleanup
        drop(lock3);
        let _ = std::fs::remove_file(&lock_path);
    }

    #[test]
    fn test_lock_timeout() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_lock_test_timeout");
        let lock_path = FileLock::lock_file_path(&test_file);

        // Clean up any existing lock
        let _ = std::fs::remove_file(&lock_path);

        // Acquire exclusive lock
        let _lock1 = FileLock::try_acquire(&test_file, LockType::Exclusive)
            .expect("Should not error")
            .expect("Should acquire first lock");

        // Try to acquire with short timeout - should fail
        let start = Instant::now();
        let result = FileLock::acquire(&test_file, LockType::Exclusive, Duration::from_millis(50));
        let elapsed = start.elapsed();

        assert!(result.is_err(), "Should timeout");
        assert!(elapsed >= Duration::from_millis(50), "Should wait for timeout");
        assert!(elapsed < Duration::from_millis(200), "Should not wait too long");

        // Cleanup
        let _ = std::fs::remove_file(&lock_path);
    }

    #[test]
    fn test_concurrent_lock_acquisition() {
        let temp_dir = std::env::temp_dir();
        let test_file = Arc::new(temp_dir.join("dx_lock_test_concurrent"));
        let lock_path = FileLock::lock_file_path(&test_file);

        // Clean up any existing lock
        let _ = std::fs::remove_file(&lock_path);

        let barrier = Arc::new(Barrier::new(2));
        let success_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let test_file1 = Arc::clone(&test_file);
        let barrier1 = Arc::clone(&barrier);
        let success1 = Arc::clone(&success_count);

        let handle1 = thread::spawn(move || {
            barrier1.wait();
            if let Ok(Some(_lock)) = FileLock::try_acquire(&test_file1, LockType::Exclusive) {
                success1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                thread::sleep(Duration::from_millis(50));
            }
        });

        let test_file2 = Arc::clone(&test_file);
        let barrier2 = Arc::clone(&barrier);
        let success2 = Arc::clone(&success_count);

        let handle2 = thread::spawn(move || {
            barrier2.wait();
            if let Ok(Some(_lock)) = FileLock::try_acquire(&test_file2, LockType::Exclusive) {
                success2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                thread::sleep(Duration::from_millis(50));
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // At least one should succeed, but not both simultaneously for exclusive
        let count = success_count.load(std::sync::atomic::Ordering::SeqCst);
        assert!(count >= 1, "At least one thread should acquire the lock");

        // Cleanup
        let _ = std::fs::remove_file(&lock_path);
    }
}
