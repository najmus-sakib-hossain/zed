//! Build cache implementation with content hashing

use crate::error::{BuildError, Result};
use crate::hash::content_hash;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache key for identifying cached artifacts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Source file path
    pub source_path: PathBuf,
    /// Content hash of the source
    pub content_hash: String,
    /// Processing type
    pub processor: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(source_path: PathBuf, content_hash: String, processor: String) -> Self {
        Self {
            source_path,
            content_hash,
            processor,
        }
    }

    /// Create a cache key from a file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read
    pub fn from_file(path: &Path, processor: String) -> Result<Self> {
        let data = std::fs::read(path).map_err(|e| BuildError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let hash = content_hash(&data);
        Ok(Self::new(path.to_path_buf(), hash, processor))
    }
}

/// Cache entry for a processed artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key
    pub key: CacheKey,
    /// Output file path
    pub output_path: PathBuf,
    /// Output content hash
    pub output_hash: String,
    /// Timestamp when cached
    pub timestamp: u64,
    /// Size in bytes
    pub size: usize,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(key: CacheKey, output_path: PathBuf, output_hash: String, size: usize) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            key,
            output_path,
            output_hash,
            timestamp,
            size,
        }
    }

    /// Check if the cached output still exists and is valid
    pub fn is_valid(&self) -> bool {
        if !self.output_path.exists() {
            return false;
        }

        // Verify output hash matches
        if let Ok(data) = std::fs::read(&self.output_path) {
            let hash = content_hash(&data);
            hash == self.output_hash
        } else {
            false
        }
    }
}

/// Build cache for tracking processed artifacts
pub struct BuildCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// Cache entries as a vector (for JSON serialization)
    entries: Vec<CacheEntry>,
    /// Path to the cache index file
    index_path: PathBuf,
}

impl BuildCache {
    /// Create a new build cache
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or the index cannot be loaded
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir).map_err(|e| BuildError::Io {
            path: cache_dir.to_path_buf(),
            source: e,
        })?;

        let index_path = cache_dir.join("index.json");
        let entries = if index_path.exists() {
            Self::load_index(&index_path)?
        } else {
            Vec::new()
        };

        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
            entries,
            index_path,
        })
    }

    /// Load the cache index from disk
    fn load_index(path: &Path) -> Result<Vec<CacheEntry>> {
        let data = std::fs::read(path).map_err(|e| BuildError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        serde_json::from_slice(&data)
            .map_err(|e| BuildError::Cache(format!("Failed to parse cache index: {}", e)))
    }

    /// Save the cache index to disk
    fn save_index(&self) -> Result<()> {
        let data = serde_json::to_vec_pretty(&self.entries)
            .map_err(|e| BuildError::Cache(format!("Failed to serialize cache index: {}", e)))?;

        std::fs::write(&self.index_path, data).map_err(|e| BuildError::Io {
            path: self.index_path.clone(),
            source: e,
        })?;

        Ok(())
    }

    /// Get a cached entry if it exists and is valid
    pub fn get(&self, key: &CacheKey) -> Option<&CacheEntry> {
        self.entries
            .iter()
            .find(|entry| &entry.key == key)
            .filter(|entry| entry.is_valid())
    }

    /// Insert a new cache entry
    ///
    /// # Errors
    ///
    /// Returns an error if the cache index cannot be saved
    pub fn insert(&mut self, entry: CacheEntry) -> Result<()> {
        // Remove existing entry with same key if present
        self.entries.retain(|e| e.key != entry.key);
        self.entries.push(entry);
        self.save_index()
    }

    /// Remove a cache entry
    ///
    /// # Errors
    ///
    /// Returns an error if the cache index cannot be saved
    pub fn remove(&mut self, key: &CacheKey) -> Result<()> {
        self.entries.retain(|e| &e.key != key);
        self.save_index()
    }

    /// Clear all cache entries
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be cleared
    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        self.save_index()?;

        // Remove all files in cache directory except index
        let entries = std::fs::read_dir(&self.cache_dir).map_err(|e| BuildError::Io {
            path: self.cache_dir.clone(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| BuildError::Io {
                path: self.cache_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path != self.index_path && path.is_file() {
                std::fs::remove_file(&path).map_err(|e| BuildError::Io {
                    path: path.clone(),
                    source: e,
                })?;
            }
        }

        Ok(())
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_key_creation() {
        let key =
            CacheKey::new(PathBuf::from("test.css"), "abc123".to_string(), "style".to_string());
        assert_eq!(key.source_path, PathBuf::from("test.css"));
        assert_eq!(key.content_hash, "abc123");
        assert_eq!(key.processor, "style");
    }

    #[test]
    fn test_cache_entry_creation() {
        let key =
            CacheKey::new(PathBuf::from("test.css"), "abc123".to_string(), "style".to_string());
        let entry =
            CacheEntry::new(key.clone(), PathBuf::from("output.dxbd"), "def456".to_string(), 1024);
        assert_eq!(entry.key, key);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_build_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = BuildCache::new(temp_dir.path());
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let key =
            CacheKey::new(PathBuf::from("test.css"), "abc123".to_string(), "style".to_string());

        // Create a temporary output file
        let output_path = temp_dir.path().join("output.dxbd");
        std::fs::write(&output_path, b"test data").unwrap();
        let output_hash = content_hash(b"test data");

        let entry = CacheEntry::new(key.clone(), output_path, output_hash, 9);
        cache.insert(entry).unwrap();

        assert_eq!(cache.len(), 1);
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn test_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();

        let key =
            CacheKey::new(PathBuf::from("test.css"), "abc123".to_string(), "style".to_string());

        // Create a temporary output file
        let output_path = temp_dir.path().join("output.dxbd");
        std::fs::write(&output_path, b"test data").unwrap();
        let output_hash = content_hash(b"test data");

        // Insert entry and drop cache
        {
            let mut cache = BuildCache::new(temp_dir.path()).unwrap();
            let entry = CacheEntry::new(key.clone(), output_path, output_hash, 9);
            cache.insert(entry).unwrap();
        }

        // Load cache again and verify entry exists
        let cache = BuildCache::new(temp_dir.path()).unwrap();
        assert_eq!(cache.len(), 1);
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        let key =
            CacheKey::new(PathBuf::from("test.css"), "abc123".to_string(), "style".to_string());

        let output_path = temp_dir.path().join("output.dxbd");
        std::fs::write(&output_path, b"test data").unwrap();
        let output_hash = content_hash(b"test data");

        let entry = CacheEntry::new(key, output_path.clone(), output_hash, 9);
        cache.insert(entry).unwrap();

        assert_eq!(cache.len(), 1);
        assert!(output_path.exists());

        cache.clear().unwrap();
        assert_eq!(cache.len(), 0);
        assert!(!output_path.exists());
    }
}
