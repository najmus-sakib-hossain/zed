//! Direct tarball extraction - fast path for cold installs
//! No binary conversion, just extract like Bun does
//!
//! Features:
//! - Proper handling of npm tarball "package/" prefix
//! - File permission preservation (Unix)
//! - Symlink support
//! - Hardlink support for cache-based extraction
//! - Descriptive error messages

use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use thiserror::Error;

/// Extraction errors with helpful messages
#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("Failed to open tarball: {path}\n\n  💡 Check that the file exists and is readable")]
    OpenFailed { path: PathBuf, source: io::Error },

    #[error("Failed to read tarball entries: {0}\n\n  💡 The tarball may be corrupted. Try re-downloading.")]
    ReadEntries(io::Error),

    #[error("Failed to create directory: {path}\n\n  💡 Check directory permissions")]
    CreateDir { path: PathBuf, source: io::Error },

    #[error("Failed to extract file: {path}\n\n  💡 Check disk space and permissions")]
    ExtractFile { path: PathBuf, source: io::Error },

    #[error("Failed to create symlink: {link} -> {target}\n\n  💡 On Windows, symlinks may require admin privileges")]
    CreateSymlink {
        link: PathBuf,
        target: PathBuf,
        source: io::Error,
    },

    #[error("Failed to create hardlink: {link} -> {target}\n\n  💡 Hardlinks must be on the same filesystem")]
    CreateHardlink {
        link: PathBuf,
        target: PathBuf,
        source: io::Error,
    },

    #[error("Failed to set permissions on: {path}")]
    SetPermissions { path: PathBuf, source: io::Error },

    #[error("Invalid tarball format")]
    InvalidFormat,
}

/// Direct extractor - extracts tarball without binary conversion
pub struct DirectExtractor;

impl DirectExtractor {
    /// Extract tarball to node_modules - FAST!
    pub fn extract(tgz_path: &Path, target_dir: &Path) -> io::Result<()> {
        let file = File::open(tgz_path)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        // Extract all entries
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            // Skip "package/" prefix that npm tarballs have
            let path_str = path.to_string_lossy();
            let clean_path = path_str.strip_prefix("package/").unwrap_or(&path_str);

            // Skip empty paths
            if clean_path.is_empty() || clean_path == "." {
                continue;
            }

            let target_path = target_dir.join(clean_path);

            // Create parent directories
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Handle different entry types
            let entry_type = entry.header().entry_type();

            if entry_type.is_file() {
                // Extract regular file
                entry.unpack(&target_path)?;

                // Preserve permissions on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mode) = entry.header().mode() {
                        fs::set_permissions(&target_path, fs::Permissions::from_mode(mode))?;
                    }
                }
            } else if entry_type.is_dir() {
                // Create directory
                fs::create_dir_all(&target_path)?;
            } else if entry_type.is_symlink() {
                // Create symlink
                if let Some(link_name) = entry.link_name()? {
                    Self::create_symlink(&link_name, &target_path)?;
                }
            } else if entry_type.is_hard_link() {
                // Create hardlink
                if let Some(link_name) = entry.link_name()? {
                    let link_target = target_dir.join(
                        link_name
                            .to_string_lossy()
                            .strip_prefix("package/")
                            .unwrap_or(&link_name.to_string_lossy()),
                    );
                    fs::hard_link(&link_target, &target_path)?;
                }
            }
            // Skip other entry types (block devices, etc.)
        }

        Ok(())
    }

    /// Extract from in-memory tarball data
    pub fn extract_from_bytes(tgz_data: &[u8], target_dir: &Path) -> io::Result<()> {
        let gz = GzDecoder::new(tgz_data);
        let mut archive = Archive::new(gz);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            let path_str = path.to_string_lossy();
            let clean_path = path_str.strip_prefix("package/").unwrap_or(&path_str);

            if clean_path.is_empty() || clean_path == "." {
                continue;
            }

            let target_path = target_dir.join(clean_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let entry_type = entry.header().entry_type();

            if entry_type.is_file() {
                entry.unpack(&target_path)?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mode) = entry.header().mode() {
                        fs::set_permissions(&target_path, fs::Permissions::from_mode(mode))?;
                    }
                }
            } else if entry_type.is_dir() {
                fs::create_dir_all(&target_path)?;
            } else if entry_type.is_symlink() {
                if let Some(link_name) = entry.link_name()? {
                    Self::create_symlink(&link_name, &target_path)?;
                }
            }
        }

        Ok(())
    }

    /// Create a symlink (cross-platform)
    fn create_symlink(target: &Path, link: &Path) -> io::Result<()> {
        // Remove existing file/link if present
        if link.exists() || link.symlink_metadata().is_ok() {
            fs::remove_file(link).ok();
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)
        }

        #[cfg(windows)]
        {
            // On Windows, try file symlink first, then directory symlink
            if target.is_dir() {
                std::os::windows::fs::symlink_dir(target, link)
            } else {
                std::os::windows::fs::symlink_file(target, link)
            }
        }
    }

    /// Parallel extraction of multiple packages (single-threaded for now)
    pub fn extract_many(packages: &[(PathBuf, PathBuf)]) -> io::Result<()> {
        for (tgz_path, target_dir) in packages {
            Self::extract(tgz_path, target_dir)?;
        }
        Ok(())
    }

    /// Extract using hardlinks from cache (instant extraction)
    /// This is the fastest method when the cache is on the same filesystem
    pub fn extract_with_hardlinks(cache_path: &Path, dest: &Path) -> io::Result<()> {
        // Walk cache directory and create hardlinks
        for entry in walkdir::WalkDir::new(cache_path) {
            let entry = entry.map_err(io::Error::other)?;
            let relative = entry.path().strip_prefix(cache_path).map_err(io::Error::other)?;

            if relative.as_os_str().is_empty() {
                continue;
            }

            let dest_path = dest.join(relative);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest_path)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                // Try hardlink first, fall back to copy
                if fs::hard_link(entry.path(), &dest_path).is_err() {
                    fs::copy(entry.path(), &dest_path)?;
                }
            } else if entry.file_type().is_symlink() {
                let target = fs::read_link(entry.path())?;
                Self::create_symlink(&target, &dest_path)?;
            }
        }

        Ok(())
    }

    /// Extract using reflinks (copy-on-write) on supported filesystems
    #[cfg(target_os = "macos")]
    pub fn extract_with_reflinks(cache_path: &Path, dest: &Path) -> io::Result<()> {
        use std::process::Command;

        // Use cp -c for CoW copy on macOS (APFS)
        let status = Command::new("cp")
            .args([
                "-cR",
                cache_path
                    .to_str()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?,
                dest.to_str()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?,
            ])
            .status()?;

        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "Reflink copy failed"));
        }

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn extract_with_reflinks(cache_path: &Path, dest: &Path) -> io::Result<()> {
        // Fall back to hardlinks on non-macOS
        Self::extract_with_hardlinks(cache_path, dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_creates_directories() {
        // This test would need a real tarball to work
        // For now, just verify the struct exists
        let _ = DirectExtractor;
    }
}
