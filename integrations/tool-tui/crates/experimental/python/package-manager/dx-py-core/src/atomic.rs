//! Atomic File Operations
//!
//! Provides atomic file operations that ensure data integrity by writing to
//! temporary files first and then atomically renaming them to the target path.
//! This prevents partial writes and corruption on failure.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Atomic file writer that writes to a temporary file and renames on completion.
///
/// If the writer is dropped without calling `commit()`, the temporary file is
/// automatically cleaned up.
pub struct AtomicFile {
    /// Temporary file path
    temp_path: PathBuf,
    /// Target file path
    target_path: PathBuf,
    /// The temporary file handle
    file: Option<File>,
    /// Whether the file has been committed
    committed: bool,
}

impl AtomicFile {
    /// Create a new atomic file writer for the given target path.
    ///
    /// The file will be written to a temporary location in the same directory
    /// as the target, then atomically renamed on commit.
    pub fn new(target: &Path) -> Result<Self> {
        // Create temp file in the same directory to ensure atomic rename works
        let parent = target.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;

        let temp_name = format!(
            ".{}.tmp.{}",
            target.file_name().unwrap_or_default().to_string_lossy(),
            std::process::id()
        );
        let temp_path = parent.join(temp_name);

        let file = File::create(&temp_path)?;

        Ok(Self {
            temp_path,
            target_path: target.to_path_buf(),
            file: Some(file),
            committed: false,
        })
    }

    /// Get a mutable reference to the underlying file.
    pub fn file(&mut self) -> Option<&mut File> {
        self.file.as_mut()
    }

    /// Write data to the file.
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref mut file) = self.file {
            file.write_all(data)?;
            Ok(())
        } else {
            Err(Error::Cache("File already closed".to_string()))
        }
    }

    /// Commit the file by atomically renaming it to the target path.
    ///
    /// This consumes the AtomicFile and returns the target path on success.
    pub fn commit(mut self) -> Result<PathBuf> {
        // Flush and close the file
        if let Some(file) = self.file.take() {
            file.sync_all()?;
            drop(file);
        }

        // Atomic rename
        fs::rename(&self.temp_path, &self.target_path)?;
        self.committed = true;

        Ok(self.target_path.clone())
    }

    /// Abort the write and clean up the temporary file.
    pub fn abort(mut self) {
        self.cleanup();
    }

    /// Clean up the temporary file.
    fn cleanup(&mut self) {
        if !self.committed {
            // Close the file first
            self.file.take();
            // Remove the temp file
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

impl Drop for AtomicFile {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl Write for AtomicFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("File already closed"))?
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("File already closed"))?
            .flush()
    }
}

/// Atomically write data to a file.
///
/// This is a convenience function that creates an AtomicFile, writes the data,
/// and commits it.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let mut atomic = AtomicFile::new(path)?;
    atomic.write_all(data)?;
    atomic.commit()?;
    Ok(())
}

/// Atomically write a string to a file.
pub fn atomic_write_str(path: &Path, content: &str) -> Result<()> {
    atomic_write(path, content.as_bytes())
}

/// Atomic directory creation with cleanup on failure.
///
/// Creates a directory at a temporary location and renames it to the target
/// on success. If any operation fails, the temporary directory is cleaned up.
pub struct AtomicDir {
    /// Temporary directory path
    temp_path: PathBuf,
    /// Target directory path
    target_path: PathBuf,
    /// Whether the directory has been committed
    committed: bool,
}

impl AtomicDir {
    /// Create a new atomic directory for the given target path.
    pub fn new(target: &Path) -> Result<Self> {
        let parent = target.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;

        let temp_name = format!(
            ".{}.tmp.{}",
            target.file_name().unwrap_or_default().to_string_lossy(),
            std::process::id()
        );
        let temp_path = parent.join(temp_name);

        // Remove any existing temp directory
        if temp_path.exists() {
            fs::remove_dir_all(&temp_path)?;
        }

        fs::create_dir_all(&temp_path)?;

        Ok(Self {
            temp_path,
            target_path: target.to_path_buf(),
            committed: false,
        })
    }

    /// Get the temporary directory path for writing files.
    pub fn path(&self) -> &Path {
        &self.temp_path
    }

    /// Commit the directory by atomically renaming it to the target path.
    pub fn commit(mut self) -> Result<PathBuf> {
        // Remove target if it exists
        if self.target_path.exists() {
            fs::remove_dir_all(&self.target_path)?;
        }

        // Atomic rename
        fs::rename(&self.temp_path, &self.target_path)?;
        self.committed = true;

        Ok(self.target_path.clone())
    }

    /// Abort and clean up the temporary directory.
    pub fn abort(mut self) {
        self.cleanup();
    }

    /// Clean up the temporary directory.
    fn cleanup(&mut self) {
        if !self.committed && self.temp_path.exists() {
            let _ = fs::remove_dir_all(&self.temp_path);
        }
    }
}

impl Drop for AtomicDir {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Guard that cleans up a path on drop unless disarmed.
///
/// Useful for ensuring cleanup of partially created resources on failure.
pub struct CleanupGuard {
    path: PathBuf,
    armed: bool,
}

impl CleanupGuard {
    /// Create a new cleanup guard for the given path.
    pub fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    /// Disarm the guard, preventing cleanup on drop.
    pub fn disarm(&mut self) {
        self.armed = false;
    }

    /// Get the path being guarded.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if self.armed && self.path.exists() {
            if self.path.is_dir() {
                let _ = fs::remove_dir_all(&self.path);
            } else {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_file_commit() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        let mut atomic = AtomicFile::new(&target).unwrap();
        atomic.write_all(b"hello world").unwrap();
        atomic.commit().unwrap();

        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello world");
    }

    #[test]
    fn test_atomic_file_abort() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        let mut atomic = AtomicFile::new(&target).unwrap();
        atomic.write_all(b"hello world").unwrap();
        atomic.abort();

        assert!(!target.exists());
    }

    #[test]
    fn test_atomic_file_drop_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        {
            let mut atomic = AtomicFile::new(&target).unwrap();
            atomic.write_all(b"hello world").unwrap();
            // Drop without commit
        }

        assert!(!target.exists());
    }

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        atomic_write(&target, b"hello world").unwrap();

        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello world");
    }

    #[test]
    fn test_atomic_dir_commit() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test_dir");

        let atomic = AtomicDir::new(&target).unwrap();
        fs::write(atomic.path().join("file.txt"), "content").unwrap();
        atomic.commit().unwrap();

        assert!(target.exists());
        assert!(target.join("file.txt").exists());
    }

    #[test]
    fn test_atomic_dir_abort() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test_dir");

        let atomic = AtomicDir::new(&target).unwrap();
        fs::write(atomic.path().join("file.txt"), "content").unwrap();
        atomic.abort();

        assert!(!target.exists());
    }

    #[test]
    fn test_cleanup_guard() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");
        fs::write(&path, "content").unwrap();

        {
            let _guard = CleanupGuard::new(path.clone());
            // Drop without disarm
        }

        assert!(!path.exists());
    }

    #[test]
    fn test_cleanup_guard_disarm() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");
        fs::write(&path, "content").unwrap();

        {
            let mut guard = CleanupGuard::new(path.clone());
            guard.disarm();
        }

        assert!(path.exists());
    }
}
