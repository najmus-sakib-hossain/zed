//! Marker evaluation cache
//!
//! LRU cache for marker evaluation results.

use lru::LruCache;
use std::num::NonZeroUsize;

/// Cache for marker evaluation results
pub struct MarkerCache {
    cache: LruCache<String, bool>,
}

impl MarkerCache {
    /// Create a new cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: LruCache::new(cap),
        }
    }

    /// Get a cached result
    pub fn get(&mut self, marker: &str) -> Option<bool> {
        self.cache.get(marker).copied()
    }

    /// Insert a result into the cache
    pub fn insert(&mut self, marker: String, result: bool) {
        self.cache.put(marker, result);
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let mut cache = MarkerCache::new(10);

        assert!(cache.get("test").is_none());

        cache.insert("test".to_string(), true);
        assert_eq!(cache.get("test"), Some(true));

        cache.insert("test2".to_string(), false);
        assert_eq!(cache.get("test2"), Some(false));
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = MarkerCache::new(2);

        cache.insert("a".to_string(), true);
        cache.insert("b".to_string(), true);
        cache.insert("c".to_string(), true);

        // "a" should be evicted
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }
}
