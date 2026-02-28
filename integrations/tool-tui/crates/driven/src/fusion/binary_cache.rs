//! Persistent Binary Cache
//!
//! Disk-backed cache for compiled templates.

use crate::Result;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Cache key (based on template and source hashes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Template hash
    pub template_hash: u64,
    /// Source hash
    pub source_hash: u64,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(template_hash: u64, source_hash: u64) -> Self {
        Self {
            template_hash,
            source_hash,
        }
    }

    /// Generate filename for cache entry
    pub fn filename(&self) -> String {
        format!("{:016x}_{:016x}.dtm", self.template_hash, self.source_hash)
    }

    /// Parse from filename
    pub fn from_filename(name: &str) -> Option<Self> {
        let name = name.strip_suffix(".dtm")?;
        let mut parts = name.split('_');

        let template_hash = u64::from_str_radix(parts.next()?, 16).ok()?;
        let source_hash = u64::from_str_radix(parts.next()?, 16).ok()?;

        Some(Self {
            template_hash,
            source_hash,
        })
    }
}

/// Cache entry metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cache key
    pub key: CacheKey,
    /// File path
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Creation timestamp
    pub created: std::time::SystemTime,
}

/// Persistent binary cache
#[derive(Debug)]
pub struct BinaryCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// Index of cached entries
    index: HashMap<u64, CacheEntry>,
    /// Maximum cache size in bytes
    max_size: u64,
    /// Current cache size
    current_size: u64,
}

impl BinaryCache {
    /// Open or create a cache directory
    pub fn open(cache_dir: &Path) -> Result<Self> {
        fs::create_dir_all(cache_dir)?;

        let mut cache = Self {
            cache_dir: cache_dir.to_path_buf(),
            index: HashMap::new(),
            max_size: 512 * 1024 * 1024, // 512MB default
            current_size: 0,
        };

        cache.scan()?;

        Ok(cache)
    }

    /// Set maximum cache size
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = max_size;
        self
    }

    /// Scan cache directory for existing entries
    fn scan(&mut self) -> Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(key) = CacheKey::from_filename(name) {
                    if let Ok(metadata) = entry.metadata() {
                        let cache_entry = CacheEntry {
                            key,
                            path: path.clone(),
                            size: metadata.len(),
                            created: metadata
                                .created()
                                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                        };
                        self.current_size += cache_entry.size;
                        self.index.insert(key.template_hash, cache_entry);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get cached template
    pub fn get(&self, template_hash: u64, source_hash: u64) -> Result<Option<Vec<u8>>> {
        if let Some(entry) = self.index.get(&template_hash) {
            // Check if source hash matches (not stale)
            if entry.key.source_hash == source_hash {
                let mut file = File::open(&entry.path)?;
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                return Ok(Some(data));
            }
            // Stale entry - caller should recompile
        }

        Ok(None)
    }

    /// Store compiled template
    pub fn put(&mut self, template_hash: u64, source_hash: u64, data: &[u8]) -> Result<()> {
        let key = CacheKey::new(template_hash, source_hash);

        // Evict if necessary
        while self.current_size + data.len() as u64 > self.max_size {
            self.evict_oldest()?;
        }

        // Remove old version if exists
        if let Some(old) = self.index.remove(&template_hash) {
            self.current_size -= old.size;
            let _ = fs::remove_file(&old.path);
        }

        // Write new entry
        let path = self.cache_dir.join(key.filename());
        let mut file = File::create(&path)?;
        file.write_all(data)?;

        let entry = CacheEntry {
            key,
            path,
            size: data.len() as u64,
            created: std::time::SystemTime::now(),
        };

        self.current_size += entry.size;
        self.index.insert(template_hash, entry);

        Ok(())
    }

    /// Remove a cached entry
    pub fn remove(&mut self, template_hash: u64) -> Result<bool> {
        if let Some(entry) = self.index.remove(&template_hash) {
            self.current_size -= entry.size;
            fs::remove_file(&entry.path)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Clear entire cache
    pub fn clear(&mut self) -> Result<()> {
        for entry in self.index.values() {
            let _ = fs::remove_file(&entry.path);
        }
        self.index.clear();
        self.current_size = 0;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> BinaryCacheStats {
        BinaryCacheStats {
            entries: self.index.len(),
            size_bytes: self.current_size,
            max_size_bytes: self.max_size,
        }
    }

    /// Evict oldest entry
    fn evict_oldest(&mut self) -> Result<()> {
        let oldest = self.index.iter().min_by_key(|(_, e)| e.created).map(|(k, _)| *k);

        if let Some(key) = oldest {
            self.remove(key)?;
        }

        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct BinaryCacheStats {
    /// Number of cached entries
    pub entries: usize,
    /// Current cache size in bytes
    pub size_bytes: u64,
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_filename() {
        let key = CacheKey::new(0x1234567890ABCDEF, 0xFEDCBA0987654321);
        let filename = key.filename();

        assert_eq!(filename, "1234567890abcdef_fedcba0987654321.dtm");

        let parsed = CacheKey::from_filename(&filename).unwrap();
        assert_eq!(parsed, key);
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = std::env::temp_dir().join("driven_cache_test");
        let _ = fs::remove_dir_all(&temp_dir);

        let mut cache = BinaryCache::open(&temp_dir).unwrap();

        // Put and get
        cache.put(1, 100, b"test data").unwrap();
        let data = cache.get(1, 100).unwrap();
        assert_eq!(data.as_deref(), Some(b"test data".as_slice()));

        // Stale check
        let stale = cache.get(1, 200).unwrap();
        assert!(stale.is_none());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
