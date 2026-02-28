//! Multi-layer Caching System
//!
//! Provides LRU caching for operations, blob content, and query results.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

/// LRU Cache entry
struct CacheEntry<V> {
    value: V,
    access_count: u64,
    last_accessed: std::time::Instant,
}

/// LRU Cache implementation
pub struct LruCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    cache: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    capacity: usize,
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new LRU cache with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            capacity,
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();

        if let Some(entry) = cache.get_mut(key) {
            entry.access_count += 1;
            entry.last_accessed = std::time::Instant::now();
            *self.hits.write() += 1;
            Some(entry.value.clone())
        } else {
            *self.misses.write() += 1;
            None
        }
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.write();

        // Check if we need to evict
        if cache.len() >= self.capacity && !cache.contains_key(&key) {
            self.evict_lru(&mut cache);
        }

        cache.insert(
            key,
            CacheEntry {
                value,
                access_count: 1,
                last_accessed: std::time::Instant::now(),
            },
        );
    }

    /// Evict least recently used entry
    fn evict_lru(&self, cache: &mut HashMap<K, CacheEntry<V>>) {
        if let Some((key_to_remove, _)) = cache.iter().min_by_key(|(_, entry)| entry.last_accessed)
        {
            let key = key_to_remove.clone();
            cache.remove(&key);
        }
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) {
        let mut cache = self.cache.write();
        cache.remove(key);
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        *self.hits.write() = 0;
        *self.misses.write() = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = *self.hits.read();
        let misses = *self.misses.read();
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            hits,
            misses,
            hit_rate,
            size: self.cache.read().len(),
            capacity: self.capacity,
        }
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
    pub capacity: usize,
}

/// Multi-layer cache manager
pub struct CacheManager {
    /// Cache for CRDT operations
    operation_cache: LruCache<String, Vec<u8>>,

    /// Cache for blob content
    blob_cache: LruCache<String, Vec<u8>>,

    /// Cache for query results
    query_cache: LruCache<String, String>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            operation_cache: LruCache::new(1000),
            blob_cache: LruCache::new(100),
            query_cache: LruCache::new(500),
        }
    }

    /// Create with custom capacities
    pub fn with_capacities(
        op_capacity: usize,
        blob_capacity: usize,
        query_capacity: usize,
    ) -> Self {
        Self {
            operation_cache: LruCache::new(op_capacity),
            blob_cache: LruCache::new(blob_capacity),
            query_cache: LruCache::new(query_capacity),
        }
    }

    /// Get operation from cache
    pub fn get_operation(&self, key: &str) -> Option<Vec<u8>> {
        self.operation_cache.get(&key.to_string())
    }

    /// Cache an operation
    pub fn cache_operation(&self, key: String, data: Vec<u8>) {
        self.operation_cache.insert(key, data);
    }

    /// Get blob from cache
    pub fn get_blob(&self, hash: &str) -> Option<Vec<u8>> {
        self.blob_cache.get(&hash.to_string())
    }

    /// Cache a blob
    pub fn cache_blob(&self, hash: String, content: Vec<u8>) {
        self.blob_cache.insert(hash, content);
    }

    /// Get query result from cache
    pub fn get_query(&self, query: &str) -> Option<String> {
        self.query_cache.get(&query.to_string())
    }

    /// Cache a query result
    pub fn cache_query(&self, query: String, result: String) {
        self.query_cache.insert(query, result);
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.operation_cache.clear();
        self.blob_cache.clear();
        self.query_cache.clear();
    }

    /// Get combined statistics
    pub fn get_stats(&self) -> CombinedStats {
        CombinedStats {
            operation_cache: self.operation_cache.stats(),
            blob_cache: self.blob_cache.stats(),
            query_cache: self.query_cache.stats(),
        }
    }

    /// Print cache statistics
    pub fn print_stats(&self) {
        let stats = self.get_stats();

        println!("\n=== Cache Statistics ===\n");

        println!("Operation Cache:");
        print_cache_stats(&stats.operation_cache);

        println!("\nBlob Cache:");
        print_cache_stats(&stats.blob_cache);

        println!("\nQuery Cache:");
        print_cache_stats(&stats.query_cache);

        let total_hits =
            stats.operation_cache.hits + stats.blob_cache.hits + stats.query_cache.hits;
        let total_misses =
            stats.operation_cache.misses + stats.blob_cache.misses + stats.query_cache.misses;
        let overall_hit_rate = if total_hits + total_misses > 0 {
            total_hits as f64 / (total_hits + total_misses) as f64
        } else {
            0.0
        };

        println!("\nOverall Hit Rate: {:.2}%", overall_hit_rate * 100.0);
        println!();
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

fn print_cache_stats(stats: &CacheStats) {
    println!("  Size: {}/{}", stats.size, stats.capacity);
    println!("  Hits: {}", stats.hits);
    println!("  Misses: {}", stats.misses);
    println!("  Hit Rate: {:.2}%", stats.hit_rate * 100.0);
}

/// Combined cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedStats {
    pub operation_cache: CacheStats,
    pub blob_cache: CacheStats,
    pub query_cache: CacheStats,
}

/// Global cache manager instance
static GLOBAL_CACHE: once_cell::sync::Lazy<CacheManager> =
    once_cell::sync::Lazy::new(CacheManager::new);

/// Get the global cache manager
pub fn global_cache() -> &'static CacheManager {
    &GLOBAL_CACHE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let cache = LruCache::new(3);

        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));

        // This should evict the least recently used (which is "a" again after get)
        cache.insert("d", 4);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_cache_stats() {
        let cache = LruCache::new(10);

        cache.insert("a", 1);
        cache.insert("b", 2);

        let _ = cache.get(&"a"); // hit
        let _ = cache.get(&"a"); // hit
        let _ = cache.get(&"c"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 2);
    }

    #[test]
    fn test_cache_manager() {
        let manager = CacheManager::new();

        manager.cache_operation("op1".to_string(), vec![1, 2, 3]);
        manager.cache_blob("hash1".to_string(), vec![4, 5, 6]);
        manager.cache_query("query1".to_string(), "result1".to_string());

        assert_eq!(manager.get_operation("op1"), Some(vec![1, 2, 3]));
        assert_eq!(manager.get_blob("hash1"), Some(vec![4, 5, 6]));
        assert_eq!(manager.get_query("query1"), Some("result1".to_string()));
    }
}
