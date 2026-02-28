//! Inline LRU cache implementation for @lru_cache decorator

use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A cache key that can be hashed
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CacheKey {
    /// Serialized arguments
    pub args: Vec<u8>,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.args.hash(state);
    }
}

impl CacheKey {
    /// Create a cache key from arguments
    pub fn from_args(args: &[u64]) -> Self {
        let mut bytes = Vec::with_capacity(args.len() * 8);
        for arg in args {
            bytes.extend_from_slice(&arg.to_le_bytes());
        }
        Self { args: bytes }
    }
}

/// Entry in the LRU cache
struct CacheEntry {
    /// Cached value
    value: u64,
    /// Access order (higher = more recent)
    order: u64,
}

/// Inline LRU cache for decorated functions
pub struct InlineLruCache {
    /// Maximum cache size
    maxsize: usize,
    /// Cache entries
    entries: Mutex<HashMap<CacheKey, CacheEntry>>,
    /// Current access counter
    counter: Mutex<u64>,
    /// Cache statistics
    stats: Mutex<CacheStats>,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
}

impl CacheStats {
    /// Get the hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl InlineLruCache {
    /// Create a new LRU cache
    pub fn new(maxsize: usize) -> Self {
        Self {
            maxsize,
            entries: Mutex::new(HashMap::new()),
            counter: Mutex::new(0),
            stats: Mutex::new(CacheStats::default()),
        }
    }

    /// Look up a value in the cache
    pub fn get(&self, key: &CacheKey) -> Option<u64> {
        let mut entries = self.entries.lock();

        if let Some(entry) = entries.get_mut(key) {
            // Update access order
            let mut counter = self.counter.lock();
            *counter += 1;
            entry.order = *counter;

            // Update stats
            self.stats.lock().hits += 1;

            Some(entry.value)
        } else {
            self.stats.lock().misses += 1;
            None
        }
    }

    /// Store a value in the cache
    pub fn put(&self, key: CacheKey, value: u64) {
        let mut entries = self.entries.lock();

        // Evict if necessary
        if entries.len() >= self.maxsize && !entries.contains_key(&key) {
            self.evict_lru(&mut entries);
        }

        // Update counter
        let mut counter = self.counter.lock();
        *counter += 1;
        let order = *counter;

        entries.insert(key, CacheEntry { value, order });
    }

    /// Evict the least recently used entry
    fn evict_lru(&self, entries: &mut HashMap<CacheKey, CacheEntry>) {
        if let Some(lru_key) = entries.iter().min_by_key(|(_, e)| e.order).map(|(k, _)| k.clone()) {
            entries.remove(&lru_key);
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.entries.lock().clear();
        *self.stats.lock() = CacheStats::default();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().clone()
    }

    /// Get the current cache size
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// Get the maximum cache size
    pub fn maxsize(&self) -> usize {
        self.maxsize
    }
}

impl std::fmt::Debug for InlineLruCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InlineLruCache")
            .field("maxsize", &self.maxsize)
            .field("len", &self.len())
            .field("stats", &self.stats())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let cache = InlineLruCache::new(10);
        let key = CacheKey::from_args(&[1, 2, 3]);

        cache.put(key.clone(), 42);
        assert_eq!(cache.get(&key), Some(42));

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let cache = InlineLruCache::new(10);
        let key = CacheKey::from_args(&[1, 2, 3]);

        assert_eq!(cache.get(&key), None);

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = InlineLruCache::new(3);

        // Fill the cache
        for i in 0..3 {
            let key = CacheKey::from_args(&[i]);
            cache.put(key, i);
        }

        assert_eq!(cache.len(), 3);

        // Add one more, should evict the oldest
        let key = CacheKey::from_args(&[100]);
        cache.put(key, 100);

        assert_eq!(cache.len(), 3);

        // First entry should be evicted
        let key0 = CacheKey::from_args(&[0]);
        assert_eq!(cache.get(&key0), None);
    }

    #[test]
    fn test_lru_order() {
        let cache = InlineLruCache::new(3);

        // Fill the cache
        for i in 0..3 {
            let key = CacheKey::from_args(&[i]);
            cache.put(key, i);
        }

        // Access the first entry to make it recent
        let key0 = CacheKey::from_args(&[0]);
        cache.get(&key0);

        // Add a new entry, should evict key 1 (least recently used)
        let key_new = CacheKey::from_args(&[100]);
        cache.put(key_new, 100);

        // Key 0 should still be there
        assert_eq!(cache.get(&key0), Some(0));

        // Key 1 should be evicted
        let key1 = CacheKey::from_args(&[1]);
        assert_eq!(cache.get(&key1), None);
    }

    #[test]
    fn test_clear() {
        let cache = InlineLruCache::new(10);

        for i in 0..5 {
            let key = CacheKey::from_args(&[i]);
            cache.put(key, i);
        }

        assert_eq!(cache.len(), 5);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_hit_rate() {
        let cache = InlineLruCache::new(10);
        let key = CacheKey::from_args(&[1]);

        cache.put(key.clone(), 42);

        // 1 miss, 3 hits
        cache.get(&CacheKey::from_args(&[999])); // miss
        cache.get(&key); // hit
        cache.get(&key); // hit
        cache.get(&key); // hit

        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.75).abs() < 0.001);
    }
}
