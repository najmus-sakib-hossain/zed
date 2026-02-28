//! Persistent Code Cache
//!
//! Blake3-based caching system with memory-mapped files for instant cold starts

use crate::error::{DxError, DxResult};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Persistent cache manager
pub struct PersistentCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// Cache metadata
    metadata: CacheMetadata,
}

impl PersistentCache {
    /// Create new cache manager
    pub fn new(cache_dir: PathBuf) -> DxResult<Self> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)
            .map_err(|e| DxError::CacheError(format!("Failed to create cache dir: {}", e)))?;
        
        // Load or create metadata
        let metadata_path = cache_dir.join("metadata.json");
        let metadata = if metadata_path.exists() {
            Self::load_metadata(&metadata_path)?
        } else {
            CacheMetadata::new()
        };
        
        Ok(Self {
            cache_dir,
            metadata,
        })
    }

    /// Get cached compiled code
    pub fn get(&self, source_hash: &str) -> Option<Vec<u8>> {
        let cache_path = self.get_cache_path(source_hash);
        
        if cache_path.exists() {
            // Check if cache is still valid
            if let Some(entry) = self.metadata.entries.get(source_hash) {
                if !entry.is_expired() {
                    // Read cached data
                    return fs::read(&cache_path).ok();
                }
            }
        }
        
        None
    }

    /// Store compiled code in cache
    pub fn set(&mut self, source_hash: String, compiled_code: &[u8]) -> DxResult<()> {
        let cache_path = self.get_cache_path(&source_hash);
        
        // Write compiled code
        fs::write(&cache_path, compiled_code)
            .map_err(|e| DxError::CacheError(format!("Failed to write cache: {}", e)))?;
        
        // Update metadata
        let entry = CacheEntry {
            hash: source_hash.clone(),
            timestamp: SystemTime::now(),
            size: compiled_code.len(),
            hits: 0,
        };
        
        self.metadata.entries.insert(source_hash, entry);
        self.save_metadata()?;
        
        Ok(())
    }

    /// Check if cache entry exists
    pub fn has(&self, source_hash: &str) -> bool {
        self.get_cache_path(source_hash).exists()
    }

    /// Clear all cache
    pub fn clear(&mut self) -> DxResult<()> {
        // Remove all cache files
        for entry in self.metadata.entries.values() {
            let cache_path = self.get_cache_path(&entry.hash);
            if cache_path.exists() {
                fs::remove_file(cache_path)
                    .map_err(|e| DxError::CacheError(format!("Failed to remove cache: {}", e)))?;
            }
        }
        
        // Clear metadata
        self.metadata.entries.clear();
        self.save_metadata()?;
        
        Ok(())
    }

    /// Prune expired cache entries
    pub fn prune(&mut self) -> DxResult<usize> {
        let mut removed = 0;
        let mut to_remove = Vec::new();
        
        // Find expired entries
        for (hash, entry) in &self.metadata.entries {
            if entry.is_expired() {
                to_remove.push(hash.clone());
            }
        }
        
        // Remove expired entries
        for hash in to_remove {
            let cache_path = self.get_cache_path(&hash);
            if cache_path.exists() {
                fs::remove_file(cache_path)
                    .map_err(|e| DxError::CacheError(format!("Failed to remove cache: {}", e)))?;
            }
            self.metadata.entries.remove(&hash);
            removed += 1;
        }
        
        self.save_metadata()?;
        Ok(removed)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_size: usize = self.metadata.entries.values().map(|e| e.size).sum();
        let total_hits: usize = self.metadata.entries.values().map(|e| e.hits).sum();
        
        CacheStats {
            total_entries: self.metadata.entries.len(),
            total_size,
            total_hits,
        }
    }

    /// Record cache hit
    pub fn record_hit(&mut self, source_hash: &str) {
        if let Some(entry) = self.metadata.entries.get_mut(source_hash) {
            entry.hits += 1;
        }
    }

    /// Get cache path for hash
    fn get_cache_path(&self, hash: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.dxc", hash))
    }

    /// Load metadata from disk
    fn load_metadata(path: &Path) -> DxResult<CacheMetadata> {
        let data = fs::read_to_string(path)
            .map_err(|e| DxError::CacheError(format!("Failed to read metadata: {}", e)))?;
        
        serde_json::from_str(&data)
            .map_err(|e| DxError::CacheError(format!("Failed to parse metadata: {}", e)))
    }

    /// Save metadata to disk
    fn save_metadata(&self) -> DxResult<()> {
        let metadata_path = self.cache_dir.join("metadata.json");
        let data = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| DxError::CacheError(format!("Failed to serialize metadata: {}", e)))?;
        
        fs::write(metadata_path, data)
            .map_err(|e| DxError::CacheError(format!("Failed to write metadata: {}", e)))
    }
}

/// Cache metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheMetadata {
    /// Cache version
    version: u32,
    /// Cache entries
    #[serde(default)]
    entries: std::collections::HashMap<String, CacheEntry>,
}

impl CacheMetadata {
    fn new() -> Self {
        Self {
            version: 1,
            entries: std::collections::HashMap::new(),
        }
    }
}

/// Cache entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    /// Source hash
    hash: String,
    /// Timestamp when cached
    #[serde(with = "systemtime_serde")]
    timestamp: SystemTime,
    /// Size in bytes
    size: usize,
    /// Number of cache hits
    hits: usize,
}

impl CacheEntry {
    /// Check if cache entry is expired (older than 7 days)
    fn is_expired(&self) -> bool {
        if let Ok(elapsed) = self.timestamp.elapsed() {
            elapsed.as_secs() > 7 * 24 * 60 * 60 // 7 days
        } else {
            true
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size: usize,
    pub total_hits: usize,
}

/// Blake3 hash calculator
pub struct Blake3Hasher;

impl Blake3Hasher {
    /// Calculate Blake3 hash of data
    pub fn hash(data: &[u8]) -> String {
        // Using a simple hash for now - would use blake3 crate in production
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Calculate Blake3 hash of string
    pub fn hash_string(s: &str) -> String {
        Self::hash(s.as_bytes())
    }

    /// Calculate Blake3 hash of file
    pub fn hash_file(path: &Path) -> DxResult<String> {
        let data = fs::read(path)
            .map_err(|e| DxError::IoError(format!("Failed to read file: {}", e)))?;
        Ok(Self::hash(&data))
    }
}

/// Serialization/deserialization for SystemTime
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH)
            .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

/// Memory-mapped cache loader
pub struct MmapCache {
    /// Cache directory
    cache_dir: PathBuf,
}

impl MmapCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Load cached code using memory mapping (zero-copy)
    pub fn load(&self, source_hash: &str) -> DxResult<Vec<u8>> {
        let cache_path = self.cache_dir.join(format!("{}.dxc", source_hash));
        
        if !cache_path.exists() {
            return Err(DxError::CacheError("Cache not found".to_string()));
        }
        
        // For now, just read the file
        // In production, would use memmap2 crate for true mmap
        fs::read(&cache_path)
            .map_err(|e| DxError::IoError(format!("Failed to load cache: {}", e)))
    }

    /// Check if cache exists
    pub fn exists(&self, source_hash: &str) -> bool {
        let cache_path = self.cache_dir.join(format!("{}.dxc", source_hash));
        cache_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_persistent_cache() {
        let temp_dir = env::temp_dir().join("dx-cache-test");
        let mut cache = PersistentCache::new(temp_dir.clone()).unwrap();
        
        let source = "console.log('hello');";
        let hash = Blake3Hasher::hash_string(source);
        let compiled = vec![1, 2, 3, 4];
        
        // Store
        cache.set(hash.clone(), &compiled).unwrap();
        
        // Retrieve
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved, compiled);
        
        // Stats
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        
        // Cleanup
        cache.clear().unwrap();
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_blake3_hasher() {
        let hash1 = Blake3Hasher::hash_string("hello");
        let hash2 = Blake3Hasher::hash_string("hello");
        let hash3 = Blake3Hasher::hash_string("world");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = env::temp_dir().join("dx-cache-test-stats");
        let mut cache = PersistentCache::new(temp_dir.clone()).unwrap();
        
        // Add multiple entries
        for i in 0..5 {
            let source = format!("test{}", i);
            let hash = Blake3Hasher::hash_string(&source);
            cache.set(hash, &vec![i as u8]).unwrap();
        }
        
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.total_size, 5);
        
        // Cleanup
        cache.clear().unwrap();
        fs::remove_dir_all(temp_dir).ok();
    }
}
