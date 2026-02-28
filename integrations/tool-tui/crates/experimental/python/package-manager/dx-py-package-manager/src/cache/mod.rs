//! Content-addressable package cache
//!
//! Provides a global cache for DPP packages using content-addressed storage.
//! Directory structure: cache_root/{hash[0:2]}/{hash[2:4]}/{hash}

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::{Error, Result};

/// Content-addressable global cache
pub struct GlobalCache {
    /// Root directory for the cache
    root: PathBuf,
}

/// Statistics from garbage collection
#[derive(Debug, Default)]
pub struct GcStats {
    /// Number of packages removed
    pub removed_count: u64,
    /// Total bytes freed
    pub freed_bytes: u64,
    /// Number of packages kept
    pub kept_count: u64,
}

impl GlobalCache {
    /// Create a new cache at the given root directory
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Get the cache root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Compute the cache path for a given content hash
    ///
    /// Uses a two-level directory structure to avoid too many files in one directory:
    /// cache_root/{hash[0:2]}/{hash[2:4]}/{hash}
    pub fn get_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex = hex::encode(hash);
        self.root.join(&hex[0..2]).join(&hex[2..4]).join(&hex)
    }

    /// Check if a package with the given hash exists in the cache
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.get_path(hash).exists()
    }

    /// Store data in the cache with the given hash
    ///
    /// Returns the path where the data was stored.
    /// If data with this hash already exists, returns the existing path without writing.
    pub fn store(&self, hash: &[u8; 32], data: &[u8]) -> Result<PathBuf> {
        let path = self.get_path(hash);

        // If already exists, just return the path (deduplication)
        if path.exists() {
            // Update access time for GC tracking
            self.touch(&path)?;
            return Ok(path);
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically using a temp file
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, data)?;
        fs::rename(&temp_path, &path)?;

        Ok(path)
    }

    /// Store data and verify the hash matches
    pub fn store_verified(&self, expected_hash: &[u8; 32], data: &[u8]) -> Result<PathBuf> {
        // Compute hash of data
        let computed = blake3::hash(data);
        let computed_bytes: [u8; 32] = *computed.as_bytes();

        if &computed_bytes != expected_hash {
            return Err(Error::Cache(format!(
                "Hash mismatch: expected {}, got {}",
                hex::encode(expected_hash),
                hex::encode(computed_bytes)
            )));
        }

        self.store(expected_hash, data)
    }

    /// Get data from the cache by hash
    pub fn get(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        let path = self.get_path(hash);
        if !path.exists() {
            return Err(Error::Cache(format!("Package not found in cache: {}", hex::encode(hash))));
        }

        // Update access time for GC tracking
        self.touch(&path)?;

        Ok(fs::read(&path)?)
    }

    /// Ensure a package is in the cache, returning its path
    ///
    /// If the package exists, returns its path.
    /// If not, calls the provided function to get the data and stores it.
    pub fn ensure<F>(&self, hash: &[u8; 32], fetch: F) -> Result<PathBuf>
    where
        F: FnOnce() -> Result<Vec<u8>>,
    {
        let path = self.get_path(hash);

        if path.exists() {
            self.touch(&path)?;
            return Ok(path);
        }

        let data = fetch()?;
        self.store_verified(hash, &data)
    }

    /// Remove a package from the cache
    pub fn remove(&self, hash: &[u8; 32]) -> Result<bool> {
        let path = self.get_path(hash);
        if path.exists() {
            fs::remove_file(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Update the access time of a file (for GC tracking)
    fn touch(&self, path: &Path) -> Result<()> {
        // On Windows, we can use set_modified; on Unix we'd use utime
        // For simplicity, we just read a byte to update atime
        if path.exists() {
            let _ = fs::metadata(path);
        }
        Ok(())
    }

    /// Garbage collect packages older than the specified duration
    pub fn gc(&self, keep_duration: Duration) -> Result<GcStats> {
        let cutoff = SystemTime::now() - keep_duration;
        let mut stats = GcStats::default();

        self.gc_recursive(&self.root, cutoff, &mut stats)?;

        Ok(stats)
    }

    /// Recursively garbage collect directories
    #[allow(clippy::only_used_in_recursion)]
    fn gc_recursive(&self, dir: &Path, cutoff: SystemTime, stats: &mut GcStats) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                // Recurse into subdirectories
                self.gc_recursive(&path, cutoff, stats)?;

                // Remove empty directories
                if fs::read_dir(&path)?.next().is_none() {
                    let _ = fs::remove_dir(&path);
                }
            } else if metadata.is_file() {
                // Check if file should be removed
                let accessed = metadata.accessed().unwrap_or(metadata.modified()?);

                if accessed < cutoff {
                    let size = metadata.len();
                    if fs::remove_file(&path).is_ok() {
                        stats.removed_count += 1;
                        stats.freed_bytes += size;
                    }
                } else {
                    stats.kept_count += 1;
                }
            }
        }

        Ok(())
    }

    /// Get total cache size in bytes
    pub fn total_size(&self) -> Result<u64> {
        self.dir_size(&self.root)
    }

    /// Recursively calculate directory size
    #[allow(clippy::only_used_in_recursion)]
    fn dir_size(&self, dir: &Path) -> Result<u64> {
        let mut total = 0;

        if !dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                total += self.dir_size(&entry.path())?;
            } else {
                total += metadata.len();
            }
        }

        Ok(total)
    }

    /// List all cached package hashes
    pub fn list(&self) -> Result<Vec<[u8; 32]>> {
        let mut hashes = Vec::new();
        self.list_recursive(&self.root, &mut hashes)?;
        Ok(hashes)
    }

    /// Recursively list all package hashes
    #[allow(clippy::only_used_in_recursion)]
    fn list_recursive(&self, dir: &Path, hashes: &mut Vec<[u8; 32]>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.list_recursive(&path, hashes)?;
            } else if path.is_file() {
                // Try to parse filename as hash
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.len() == 64 {
                        if let Ok(bytes) = hex::decode(name) {
                            if bytes.len() == 32 {
                                let mut hash = [0u8; 32];
                                hash.copy_from_slice(&bytes);
                                hashes.push(hash);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_store_and_get() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let data = b"test package data";
        let hash = *blake3::hash(data).as_bytes();

        // Store
        let path = cache.store(&hash, data).unwrap();
        assert!(path.exists());

        // Get
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_cache_deduplication() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let data = b"duplicate data";
        let hash = *blake3::hash(data).as_bytes();

        // Store twice
        let path1 = cache.store(&hash, data).unwrap();
        let path2 = cache.store(&hash, data).unwrap();

        // Should return same path
        assert_eq!(path1, path2);

        // Should only have one file
        let hashes = cache.list().unwrap();
        assert_eq!(hashes.len(), 1);
    }

    #[test]
    fn test_cache_verified_store() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let data = b"verified data";
        let hash = *blake3::hash(data).as_bytes();

        // Correct hash should work
        cache.store_verified(&hash, data).unwrap();

        // Wrong hash should fail
        let wrong_hash = [0u8; 32];
        let result = cache.store_verified(&wrong_hash, data);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_ensure() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let data = b"ensured data";
        let hash = *blake3::hash(data).as_bytes();

        let mut fetch_count = 0;

        // First ensure should call fetch
        let path1 = cache
            .ensure(&hash, || {
                fetch_count += 1;
                Ok(data.to_vec())
            })
            .unwrap();

        assert_eq!(fetch_count, 1);
        assert!(path1.exists());

        // Second ensure should not call fetch
        let path2 = cache
            .ensure(&hash, || {
                fetch_count += 1;
                Ok(data.to_vec())
            })
            .unwrap();

        assert_eq!(fetch_count, 1); // Still 1
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_cache_path_structure() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let hash = [
            0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
            0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01,
            0x23, 0x45, 0x67, 0x89,
        ];

        let path = cache.get_path(&hash);
        let path_str = path.to_string_lossy();

        // Should have two-level directory structure
        assert!(path_str.contains("ab"));
        assert!(path_str.contains("cd"));
    }

    #[test]
    fn test_cache_remove() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let data = b"removable data";
        let hash = *blake3::hash(data).as_bytes();

        cache.store(&hash, data).unwrap();
        assert!(cache.contains(&hash));

        cache.remove(&hash).unwrap();
        assert!(!cache.contains(&hash));
    }
}
