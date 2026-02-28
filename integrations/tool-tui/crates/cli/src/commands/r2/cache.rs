//! R2 Local Cache
//!
//! Caches R2 objects locally for faster access

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache directory
    pub cache_dir: PathBuf,

    /// Maximum cache size in bytes
    pub max_size: u64,

    /// TTL for cached items
    pub ttl: Duration,

    /// Enable cache
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx-r2-cache");

        Self {
            cache_dir,
            max_size: 1024 * 1024 * 1024,   // 1GB
            ttl: Duration::from_secs(3600), // 1 hour
            enabled: true,
        }
    }
}

/// Cache entry metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub local_path: PathBuf,
    pub size: u64,
    pub etag: String,
    pub cached_at: SystemTime,
    pub last_accessed: SystemTime,
}

/// R2 cache manager
pub struct CacheManager {
    config: CacheConfig,
    entries: HashMap<String, CacheEntry>,
    current_size: u64,
}

impl CacheManager {
    /// Create new cache manager
    pub fn new(config: CacheConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.cache_dir)?;

        let mut manager = Self {
            config,
            entries: HashMap::new(),
            current_size: 0,
        };

        // Load existing cache entries
        manager.load_index()?;

        Ok(manager)
    }

    /// Get cached file if available and valid
    pub fn get(&mut self, key: &str) -> Option<PathBuf> {
        if !self.config.enabled {
            return None;
        }

        if let Some(entry) = self.entries.get_mut(key) {
            // Check TTL
            let now = SystemTime::now();
            if let Ok(age) = now.duration_since(entry.cached_at) {
                if age > self.config.ttl {
                    // Expired
                    return None;
                }
            }

            // Check file exists
            if entry.local_path.exists() {
                entry.last_accessed = now;
                return Some(entry.local_path.clone());
            }
        }

        None
    }

    /// Put file in cache
    pub fn put(&mut self, key: &str, data: &[u8], etag: &str) -> Result<PathBuf> {
        if !self.config.enabled {
            return Err(anyhow::anyhow!("Cache disabled"));
        }

        // Evict if necessary
        let data_size = data.len() as u64;
        while self.current_size + data_size > self.config.max_size {
            if !self.evict_one() {
                return Err(anyhow::anyhow!("Cannot make space in cache"));
            }
        }

        // Generate cache path
        let hash = hash_key(key);
        let cache_path = self.config.cache_dir.join(&hash);

        // Write to cache
        std::fs::write(&cache_path, data)?;

        let now = SystemTime::now();
        let entry = CacheEntry {
            key: key.to_string(),
            local_path: cache_path.clone(),
            size: data_size,
            etag: etag.to_string(),
            cached_at: now,
            last_accessed: now,
        };

        // Remove old entry if exists
        if let Some(old) = self.entries.remove(key) {
            self.current_size -= old.size;
            let _ = std::fs::remove_file(&old.local_path);
        }

        self.current_size += data_size;
        self.entries.insert(key.to_string(), entry);

        // Save index
        self.save_index()?;

        Ok(cache_path)
    }

    /// Invalidate cache entry
    pub fn invalidate(&mut self, key: &str) -> Result<()> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size -= entry.size;
            let _ = std::fs::remove_file(&entry.local_path);
            self.save_index()?;
        }
        Ok(())
    }

    /// Clear entire cache
    pub fn clear(&mut self) -> Result<()> {
        for entry in self.entries.values() {
            let _ = std::fs::remove_file(&entry.local_path);
        }
        self.entries.clear();
        self.current_size = 0;
        self.save_index()?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            size: self.current_size,
            max_size: self.config.max_size,
            ttl_secs: self.config.ttl.as_secs(),
        }
    }

    /// Check if key has valid cache entry
    pub fn has(&self, key: &str, etag: Option<&str>) -> bool {
        if let Some(entry) = self.entries.get(key) {
            // Check TTL
            if let Ok(age) = SystemTime::now().duration_since(entry.cached_at) {
                if age > self.config.ttl {
                    return false;
                }
            }

            // Check etag if provided
            if let Some(expected) = etag {
                if entry.etag != expected {
                    return false;
                }
            }

            // Check file exists
            entry.local_path.exists()
        } else {
            false
        }
    }

    /// Evict one entry (LRU)
    fn evict_one(&mut self) -> bool {
        let oldest =
            self.entries.iter().min_by_key(|(_, e)| e.last_accessed).map(|(k, _)| k.clone());

        if let Some(key) = oldest {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size -= entry.size;
                let _ = std::fs::remove_file(&entry.local_path);
                return true;
            }
        }

        false
    }

    /// Load cache index from disk
    fn load_index(&mut self) -> Result<()> {
        let index_path = self.config.cache_dir.join("index.json");

        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            let entries: Vec<CacheEntryJson> = serde_json::from_str(&content)?;

            for entry in entries {
                let local_path = self.config.cache_dir.join(&entry.hash);
                if local_path.exists() {
                    self.current_size += entry.size;
                    self.entries.insert(
                        entry.key.clone(),
                        CacheEntry {
                            key: entry.key,
                            local_path,
                            size: entry.size,
                            etag: entry.etag,
                            cached_at: SystemTime::UNIX_EPOCH
                                + Duration::from_secs(entry.cached_at),
                            last_accessed: SystemTime::UNIX_EPOCH
                                + Duration::from_secs(entry.last_accessed),
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Save cache index to disk
    fn save_index(&self) -> Result<()> {
        let index_path = self.config.cache_dir.join("index.json");

        let entries: Vec<CacheEntryJson> = self
            .entries
            .values()
            .map(|e| CacheEntryJson {
                key: e.key.clone(),
                hash: hash_key(&e.key),
                size: e.size,
                etag: e.etag.clone(),
                cached_at: e
                    .cached_at
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                last_accessed: e
                    .last_accessed
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            })
            .collect();

        let content = serde_json::to_string_pretty(&entries)?;
        std::fs::write(&index_path, content)?;

        Ok(())
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub entries: usize,
    pub size: u64,
    pub max_size: u64,
    pub ttl_secs: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheEntryJson {
    key: String,
    hash: String,
    size: u64,
    etag: String,
    cached_at: u64,
    last_accessed: u64,
}

/// Hash key to filename
fn hash_key(key: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}
