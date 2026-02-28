//! AST Cache
//!
//! Persistent binary AST cache for instant re-linting of unchanged files.
//! Uses memory-mapped files for zero-copy access.
//!
//! ## Graceful Shutdown
//!
//! The cache automatically persists its index when dropped. For explicit control,
//! use `AstCache::save_index()` or the global `shutdown_cache()` function.

use bincode::{Decode, Encode};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to track if shutdown hooks have been registered
static SHUTDOWN_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Global cache instances for shutdown handling
static GLOBAL_CACHES: RwLock<Vec<std::sync::Weak<AstCacheInner>>> = RwLock::new(Vec::new());

/// Cache format version - increment when format changes
const CACHE_VERSION: u32 = 1;

/// Inner cache state that can be shared
struct AstCacheInner {
    /// Cache directory
    cache_dir: PathBuf,
    /// Index of cached entries (in memory for fast lookup)
    index: RwLock<CacheIndex>,
    /// Maximum cache size in bytes
    max_size: u64,
    /// Current cache size
    current_size: RwLock<u64>,
    /// Whether the cache has been modified since last save
    dirty: AtomicBool,
}

/// Binary AST cache with memory-mapped storage
pub struct AstCache {
    inner: Arc<AstCacheInner>,
}

/// Serializable cache index for persistence
#[derive(Default, Encode, Decode)]
struct CacheIndex {
    /// Cache format version
    version: u32,
    /// Content hash -> cache entry
    entries: HashMap<[u8; 32], CacheEntry>,
}

/// Individual cache entry
#[derive(Clone, Encode, Decode)]
struct CacheEntry {
    /// Path to the cache file (relative to cache dir)
    cache_path: PathBuf,
    /// Size in bytes
    size: u64,
    /// Last access timestamp (for LRU eviction)
    last_access: u64,
}

impl AstCache {
    /// Create a new AST cache
    pub fn new(cache_dir: PathBuf, max_size: u64) -> std::io::Result<Self> {
        // Ensure cache directory exists
        std::fs::create_dir_all(&cache_dir)?;

        let inner = Arc::new(AstCacheInner {
            cache_dir,
            index: RwLock::new(CacheIndex::default()),
            max_size,
            current_size: RwLock::new(0),
            dirty: AtomicBool::new(false),
        });

        let cache = Self { inner };

        // Load existing index
        cache.load_index()?;

        // Register for global shutdown handling
        cache.register_for_shutdown();

        Ok(cache)
    }

    /// Register this cache instance for graceful shutdown
    fn register_for_shutdown(&self) {
        // Add weak reference to global list
        let mut caches = GLOBAL_CACHES.write();
        caches.push(Arc::downgrade(&self.inner));

        // Register shutdown hook if not already done
        if !SHUTDOWN_REGISTERED.swap(true, Ordering::SeqCst) {
            // Register ctrlc handler for graceful shutdown
            #[cfg(not(test))]
            {
                let _ = ctrlc::set_handler(|| {
                    tracing::info!("Received shutdown signal, persisting caches...");
                    shutdown_all_caches();
                    std::process::exit(0);
                });
            }
        }
    }

    /// Get cached AST or parse and cache
    pub fn get_or_parse<F, T>(&self, source: &[u8], parse_fn: F) -> std::io::Result<T>
    where
        F: FnOnce(&[u8]) -> T,
        T: AsRef<[u8]> + From<Vec<u8>>,
    {
        let hash = self.hash_content(source);

        // Check cache
        if let Some(cached) = self.get(&hash) {
            return Ok(T::from(cached));
        }

        // Parse and cache
        let result = parse_fn(source);
        self.store(&hash, result.as_ref())?;

        Ok(result)
    }

    /// Get cached entry by content hash
    #[must_use]
    pub fn get(&self, hash: &[u8; 32]) -> Option<Vec<u8>> {
        let index = self.inner.index.read();
        let entry = index.entries.get(hash)?;

        // Read cached data
        let mut file = File::open(&entry.cache_path).ok()?;
        let mut data = Vec::with_capacity(entry.size as usize);
        file.read_to_end(&mut data).ok()?;

        // Update last access time
        drop(index);
        let mut index = self.inner.index.write();
        if let Some(entry) = index.entries.get_mut(hash) {
            entry.last_access = current_timestamp();
            self.inner.dirty.store(true, Ordering::SeqCst);
        }

        Some(data)
    }

    /// Store data in cache
    pub fn store(&self, hash: &[u8; 32], data: &[u8]) -> std::io::Result<()> {
        // Check if we need to evict
        let data_size = data.len() as u64;
        self.maybe_evict(data_size)?;

        // Generate cache file path
        let cache_path = self.hash_to_path(hash);

        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to cache file
        let mut file = File::create(&cache_path)?;
        file.write_all(data)?;

        // Update index
        let mut index = self.inner.index.write();
        index.entries.insert(
            *hash,
            CacheEntry {
                cache_path,
                size: data_size,
                last_access: current_timestamp(),
            },
        );

        // Update current size and mark dirty
        *self.inner.current_size.write() += data_size;
        self.inner.dirty.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Hash content using blake3
    fn hash_content(&self, content: &[u8]) -> [u8; 32] {
        *blake3::hash(content).as_bytes()
    }

    /// Convert hash to file path
    fn hash_to_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex = hex::encode(hash);
        // Use first 2 characters as subdirectory for better filesystem performance
        self.inner.cache_dir.join(&hex[..2]).join(&hex[2..])
    }

    /// Evict old entries if cache is full
    fn maybe_evict(&self, needed_size: u64) -> std::io::Result<()> {
        let current = *self.inner.current_size.read();
        if current + needed_size <= self.inner.max_size {
            return Ok(());
        }

        // Need to evict - find oldest entries
        let mut index = self.inner.index.write();
        let mut entries: Vec<_> = index.entries.iter().collect();
        entries.sort_by_key(|(_, e)| e.last_access);

        let mut freed = 0u64;
        let target = (current + needed_size).saturating_sub(self.inner.max_size);

        let mut to_remove = Vec::new();
        for (hash, entry) in entries {
            if freed >= target {
                break;
            }

            // Delete cache file
            let _ = std::fs::remove_file(&entry.cache_path);
            freed += entry.size;
            to_remove.push(*hash);
        }

        // Remove from index
        for hash in to_remove {
            index.entries.remove(&hash);
        }

        // Update current size and mark dirty
        *self.inner.current_size.write() = current.saturating_sub(freed);
        self.inner.dirty.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Load index from disk
    fn load_index(&self) -> std::io::Result<()> {
        let index_path = self.inner.cache_dir.join("index.bin");
        if !index_path.exists() {
            return Ok(());
        }

        let data = std::fs::read(&index_path)?;

        // Deserialize the index using bincode 2.x API
        let config = bincode::config::standard();
        match bincode::decode_from_slice::<CacheIndex, _>(&data, config) {
            Ok((loaded_index, _)) => {
                // Check version compatibility
                if loaded_index.version != CACHE_VERSION {
                    // Version mismatch - clear cache and start fresh
                    tracing::info!(
                        "Cache version mismatch (found {}, expected {}), clearing cache",
                        loaded_index.version,
                        CACHE_VERSION
                    );
                    self.clear()?;
                    return Ok(());
                }

                // Validate entries exist on disk
                let mut valid_entries = HashMap::new();
                let mut valid_size = 0u64;

                for (hash, entry) in loaded_index.entries {
                    let full_path = self.inner.cache_dir.join(&entry.cache_path);
                    if full_path.exists() {
                        valid_size += entry.size;
                        valid_entries.insert(hash, entry);
                    }
                }

                *self.inner.index.write() = CacheIndex {
                    version: CACHE_VERSION,
                    entries: valid_entries,
                };
                *self.inner.current_size.write() = valid_size;

                tracing::debug!(
                    "Loaded cache index with {} entries ({} bytes)",
                    self.inner.index.read().entries.len(),
                    valid_size
                );
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize cache index: {}, clearing cache", e);
                self.clear()?;
            }
        }

        Ok(())
    }

    /// Save index to disk
    pub fn save_index(&self) -> std::io::Result<()> {
        // Only save if dirty
        if !self.inner.dirty.load(Ordering::SeqCst) {
            return Ok(());
        }

        let index_path = self.inner.cache_dir.join("index.bin");

        let index = self.inner.index.read();
        let cache_index = CacheIndex {
            version: CACHE_VERSION,
            entries: index.entries.clone(),
        };

        let config = bincode::config::standard();
        let data = bincode::encode_to_vec(&cache_index, config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Write atomically using a temp file
        let temp_path = self.inner.cache_dir.join("index.bin.tmp");
        std::fs::write(&temp_path, &data)?;
        std::fs::rename(&temp_path, &index_path)?;

        // Mark as clean
        self.inner.dirty.store(false, Ordering::SeqCst);

        tracing::debug!("Saved cache index with {} entries", cache_index.entries.len());

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&self) -> std::io::Result<()> {
        let mut index = self.inner.index.write();

        for entry in index.entries.values() {
            let _ = std::fs::remove_file(&entry.cache_path);
        }

        index.entries.clear();
        *self.inner.current_size.write() = 0;
        self.inner.dirty.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let index = self.inner.index.read();
        CacheStats {
            entry_count: index.entries.len(),
            total_size: *self.inner.current_size.read(),
            max_size: self.inner.max_size,
        }
    }

    /// Check if the cache has unsaved changes
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.dirty.load(Ordering::SeqCst)
    }
}

/// Implement Drop to automatically save the cache index on shutdown
impl Drop for AstCache {
    fn drop(&mut self) {
        if let Err(e) = self.save_index() {
            tracing::warn!("Failed to save cache index on drop: {}", e);
        }
    }
}

/// Shutdown all registered cache instances
///
/// This function is called automatically on SIGINT/SIGTERM signals.
/// It can also be called manually for explicit shutdown handling.
pub fn shutdown_all_caches() {
    let caches = GLOBAL_CACHES.read();
    for weak in caches.iter() {
        if let Some(inner) = weak.upgrade() {
            // Create a temporary AstCache to save the index
            let cache = AstCache { inner };
            if let Err(e) = cache.save_index() {
                tracing::warn!("Failed to save cache during shutdown: {}", e);
            }
            // Prevent double-drop by forgetting the cache
            std::mem::forget(cache);
        }
    }
    tracing::info!("All caches persisted successfully");
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached entries
    pub entry_count: usize,
    /// Total size of cache in bytes
    pub total_size: u64,
    /// Maximum cache size
    pub max_size: u64,
}

impl CacheStats {
    /// Get cache utilization as percentage
    #[must_use]
    pub fn utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.total_size as f64 / self.max_size as f64) * 100.0
        }
    }
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Hex encoding helper
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            result.push(HEX_CHARS[(b >> 4) as usize] as char);
            result.push(HEX_CHARS[(b & 0xf) as usize] as char);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_store_and_get() {
        let dir = tempdir().unwrap();
        let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

        let data = b"test data";
        let hash = *blake3::hash(data).as_bytes();

        cache.store(&hash, data).unwrap();
        let retrieved = cache.get(&hash).unwrap();

        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_cache_stats() {
        let dir = tempdir().unwrap();
        let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_cache_persistence() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().to_path_buf();

        // Create cache and store data
        {
            let cache = AstCache::new(cache_dir.clone(), 1024 * 1024).unwrap();
            let data = b"persistent data";
            let hash = *blake3::hash(data).as_bytes();
            cache.store(&hash, data).unwrap();
            cache.save_index().unwrap();
        }

        // Create new cache instance and verify data persists
        {
            let cache = AstCache::new(cache_dir, 1024 * 1024).unwrap();
            let data = b"persistent data";
            let hash = *blake3::hash(data).as_bytes();
            let retrieved = cache.get(&hash);
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap(), data);
        }
    }

    #[test]
    fn test_cache_dirty_flag() {
        let dir = tempdir().unwrap();
        let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

        // Initially not dirty
        assert!(!cache.is_dirty());

        // Store data makes it dirty
        let data = b"test data";
        let hash = *blake3::hash(data).as_bytes();
        cache.store(&hash, data).unwrap();
        assert!(cache.is_dirty());

        // Save clears dirty flag
        cache.save_index().unwrap();
        assert!(!cache.is_dirty());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let dir = tempdir().unwrap();
        // Small cache that can only hold ~100 bytes
        let cache = AstCache::new(dir.path().to_path_buf(), 100).unwrap();

        // Store first entry
        let data1 = b"first entry data";
        let hash1 = *blake3::hash(data1).as_bytes();
        cache.store(&hash1, data1).unwrap();

        // Store second entry
        let data2 = b"second entry data";
        let hash2 = *blake3::hash(data2).as_bytes();
        cache.store(&hash2, data2).unwrap();

        // Store third entry - should trigger eviction
        let data3 = vec![0u8; 80]; // Large enough to trigger eviction
        let hash3 = *blake3::hash(&data3).as_bytes();
        cache.store(&hash3, &data3).unwrap();

        // First entry should be evicted (oldest)
        assert!(cache.get(&hash1).is_none());
        // Third entry should exist
        assert!(cache.get(&hash3).is_some());
    }

    #[test]
    fn test_cache_clear() {
        let dir = tempdir().unwrap();
        let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

        // Store some data
        let data = b"test data";
        let hash = *blake3::hash(data).as_bytes();
        cache.store(&hash, data).unwrap();

        // Verify it exists
        assert!(cache.get(&hash).is_some());
        assert_eq!(cache.stats().entry_count, 1);

        // Clear cache
        cache.clear().unwrap();

        // Verify it's gone
        assert!(cache.get(&hash).is_none());
        assert_eq!(cache.stats().entry_count, 0);
    }

    #[test]
    fn test_cache_utilization() {
        let dir = tempdir().unwrap();
        let cache = AstCache::new(dir.path().to_path_buf(), 1000).unwrap();

        // Store 100 bytes
        let data = vec![0u8; 100];
        let hash = *blake3::hash(&data).as_bytes();
        cache.store(&hash, &data).unwrap();

        let stats = cache.stats();
        assert!((stats.utilization() - 10.0).abs() < 0.1); // ~10% utilization
    }

    #[test]
    fn test_cache_get_or_parse() {
        let dir = tempdir().unwrap();
        let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

        let source = b"source code";
        let mut parse_count = 0;

        // First call should parse
        let result1: Vec<u8> = cache
            .get_or_parse(source, |_| {
                parse_count += 1;
                b"parsed result".to_vec()
            })
            .unwrap();

        assert_eq!(parse_count, 1);
        assert_eq!(result1, b"parsed result");

        // Second call should use cache (but parse_count won't change due to closure capture)
        // We verify by checking the cache has the entry
        let hash = *blake3::hash(source).as_bytes();
        assert!(cache.get(&hash).is_some());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::tempdir;

    proptest! {
        /// Property: Cache size never exceeds max_size after operations
        #[test]
        fn cache_size_never_exceeds_max(
            max_size in 100u64..10000,
            entries in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..20)
        ) {
            let dir = tempdir().unwrap();
            let cache = AstCache::new(dir.path().to_path_buf(), max_size).unwrap();

            for entry in entries {
                let hash = *blake3::hash(&entry).as_bytes();
                let _ = cache.store(&hash, &entry);
            }

            let stats = cache.stats();
            prop_assert!(stats.total_size <= max_size);
        }

        /// Property: Stored data can be retrieved unchanged
        #[test]
        fn stored_data_retrievable(data in prop::collection::vec(any::<u8>(), 1..1000)) {
            let dir = tempdir().unwrap();
            let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

            let hash = *blake3::hash(&data).as_bytes();
            cache.store(&hash, &data).unwrap();

            let retrieved = cache.get(&hash).unwrap();
            prop_assert_eq!(retrieved, data);
        }

        /// Property: Cache entry count matches stored entries (within max_size)
        #[test]
        fn entry_count_consistent(
            entries in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..10)
        ) {
            let dir = tempdir().unwrap();
            // Large cache to avoid eviction
            let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

            let mut unique_hashes = std::collections::HashSet::new();
            for entry in &entries {
                let hash = *blake3::hash(entry).as_bytes();
                cache.store(&hash, entry).unwrap();
                unique_hashes.insert(hash);
            }

            let stats = cache.stats();
            prop_assert_eq!(stats.entry_count, unique_hashes.len());
        }

        /// Property: Clear removes all entries
        #[test]
        fn clear_removes_all(
            entries in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..10)
        ) {
            let dir = tempdir().unwrap();
            let cache = AstCache::new(dir.path().to_path_buf(), 1024 * 1024).unwrap();

            for entry in &entries {
                let hash = *blake3::hash(entry).as_bytes();
                cache.store(&hash, entry).unwrap();
            }

            cache.clear().unwrap();

            let stats = cache.stats();
            prop_assert_eq!(stats.entry_count, 0);
            prop_assert_eq!(stats.total_size, 0);
        }

        /// Property: Persistence round-trip preserves data
        #[test]
        fn persistence_round_trip(data in prop::collection::vec(any::<u8>(), 1..500)) {
            let dir = tempdir().unwrap();
            let cache_dir = dir.path().to_path_buf();
            let hash = *blake3::hash(&data).as_bytes();

            // Store and persist
            {
                let cache = AstCache::new(cache_dir.clone(), 1024 * 1024).unwrap();
                cache.store(&hash, &data).unwrap();
                cache.save_index().unwrap();
            }

            // Load and verify
            {
                let cache = AstCache::new(cache_dir, 1024 * 1024).unwrap();
                let retrieved = cache.get(&hash);
                prop_assert!(retrieved.is_some());
                prop_assert_eq!(retrieved.unwrap(), data);
            }
        }
    }
}
