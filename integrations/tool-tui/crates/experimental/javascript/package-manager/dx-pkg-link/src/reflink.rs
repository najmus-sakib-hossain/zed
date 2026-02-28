//! # Instant Reflink Installation
//!
//! ## The Innovation
//!
//! Traditional linking copies entire files (slow) or hardlinks (fast but can cause issues).
//! Reflinks (copy-on-write) are instant AND safe!
//!
//! Supported filesystems:
//! - Btrfs, XFS, APFS (macOS), ReFS (Windows Server)
//!
//! Fallback chain:
//! 1. Reflink (instant, COW) - Best!
//! 2. Hardlink (instant, shared inode) - Good!
//! 3. Parallel copy (fast) - Fallback

use rayon::prelude::*;
use reflink_copy::reflink;
use std::fs;
use std::io;
use std::path::Path;
use walkdir::WalkDir;

/// Reflink capabilities and installer
pub struct ReflinkLinker {
    /// Supports reflinks (Btrfs, XFS, APFS, ReFS)
    supports_reflink: bool,
    /// Supports hardlinks
    supports_hardlink: bool,
}

impl ReflinkLinker {
    pub fn new() -> Self {
        Self {
            supports_reflink: Self::detect_reflink_support(),
            supports_hardlink: true,
        }
    }

    fn detect_reflink_support() -> bool {
        // Try creating a reflink in temp dir
        let temp = std::env::temp_dir().join(".dx-reflink-test");
        let temp2 = std::env::temp_dir().join(".dx-reflink-test2");

        let _ = std::fs::write(&temp, b"test");
        let result = reflink(&temp, &temp2).is_ok();

        let _ = std::fs::remove_file(&temp);
        let _ = std::fs::remove_file(&temp2);

        result
    }

    /// Link extracted package to node_modules (chooses best method)
    pub fn link(&self, source: &Path, target: &Path) -> io::Result<()> {
        if self.supports_reflink {
            // Best: reflink (instant, COW)
            self.reflink_tree(source, target)
        } else if self.supports_hardlink {
            // Good: hardlinks (instant, shares inode)
            self.hardlink_tree(source, target)
        } else {
            // Fallback: parallel copy
            self.copy_tree(source, target)
        }
    }

    /// Reflink entire directory tree (instant copy-on-write)
    fn reflink_tree(&self, source: &Path, target: &Path) -> io::Result<()> {
        fs::create_dir_all(target)?;

        for entry in WalkDir::new(source) {
            let entry = entry?;
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            let target_path = target.join(relative);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&target_path)?;
            } else {
                // Reflink file (copy-on-write, instant!)
                reflink(entry.path(), &target_path)
                    .or_else(|_| fs::copy(entry.path(), &target_path).map(|_| ()))?;
            }
        }

        Ok(())
    }

    /// Hardlink entire directory tree (instant, shared inodes)
    fn hardlink_tree(&self, source: &Path, target: &Path) -> io::Result<()> {
        fs::create_dir_all(target)?;

        for entry in WalkDir::new(source) {
            let entry = entry?;
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            let target_path = target.join(relative);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&target_path)?;
            } else {
                fs::hard_link(entry.path(), &target_path)?;
            }
        }

        Ok(())
    }

    /// Parallel copy (fallback, still fast!)
    fn copy_tree(&self, source: &Path, target: &Path) -> io::Result<()> {
        let entries: Vec<_> = WalkDir::new(source).into_iter().collect::<Result<Vec<_>, _>>()?;

        // Create directories first (sequential)
        for entry in &entries {
            if entry.file_type().is_dir() {
                let relative = entry
                    .path()
                    .strip_prefix(source)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                fs::create_dir_all(target.join(relative))?;
            }
        }

        // Copy files in parallel
        entries.par_iter().filter(|e| e.file_type().is_file()).try_for_each(|entry| {
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            fs::copy(entry.path(), target.join(relative))?;
            Ok::<_, io::Error>(())
        })?;

        Ok(())
    }

    /// Check if reflinks are supported
    pub fn supports_reflinks(&self) -> bool {
        self.supports_reflink
    }
}

impl Default for ReflinkLinker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_reflink_detection() {
        let linker = ReflinkLinker::new();
        // Just verify it doesn't panic
        let _ = linker.supports_reflinks();
    }

    #[test]
    fn test_copy_tree() -> io::Result<()> {
        let temp = TempDir::new()?;
        let source = temp.path().join("source");
        let target = temp.path().join("target");

        fs::create_dir_all(&source)?;
        fs::write(source.join("test.txt"), b"hello")?;

        let linker = ReflinkLinker::new();
        linker.copy_tree(&source, &target)?;

        assert!(target.join("test.txt").exists());
        Ok(())
    }
}
