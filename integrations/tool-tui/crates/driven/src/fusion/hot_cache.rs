//! Hot Template Cache
//!
//! In-memory cache for frequently used templates with LRU eviction.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Cached template entry
#[derive(Debug)]
pub struct TemplateCacheEntry {
    /// Compiled template bytes
    pub data: Vec<u8>,
    /// Last access time
    last_access: Instant,
    /// Access count
    access_count: u64,
    /// Template hash
    template_hash: u64,
    /// Source hash for staleness check
    source_hash: u64,
}

impl TemplateCacheEntry {
    /// Create a new cache entry
    pub fn new(data: Vec<u8>, template_hash: u64, source_hash: u64) -> Self {
        Self {
            data,
            last_access: Instant::now(),
            access_count: 1,
            template_hash,
            source_hash,
        }
    }

    /// Mark as accessed
    fn touch(&mut self) {
        self.last_access = Instant::now();
        self.access_count += 1;
    }

    /// Check if stale
    pub fn is_stale(&self, current_source_hash: u64) -> bool {
        self.source_hash != current_source_hash
    }

    /// Get age
    pub fn age(&self) -> Duration {
        self.last_access.elapsed()
    }
}

/// Hot template cache with LRU eviction
#[derive(Debug)]
pub struct HotCache {
    /// Cached templates by hash
    entries: HashMap<u64, TemplateCacheEntry>,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current cache size
    current_size: usize,
    /// Hit counter
    hits: AtomicU64,
    /// Miss counter
    misses: AtomicU64,
    /// TTL for cache entries
    ttl: Duration,
}

impl HotCache {
    /// Create a new cache with size limit
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
            current_size: 0,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            ttl: Duration::from_secs(3600), // 1 hour default
        }
    }

    /// Set TTL for cache entries
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Get a cached template
    pub fn get(&mut self, template_hash: u64) -> Option<&[u8]> {
        // Check if entry exists and is not expired
        if let Some(entry) = self.entries.get_mut(&template_hash) {
            if entry.age() < self.ttl {
                entry.touch();
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(&entry.data);
            }
            // Entry expired, will be removed
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert a template into cache
    pub fn insert(&mut self, template_hash: u64, source_hash: u64, data: Vec<u8>) {
        let entry_size = data.len();

        // Evict if necessary
        while self.current_size + entry_size > self.max_size && !self.entries.is_empty() {
            self.evict_one();
        }

        // Don't cache if entry is too large
        if entry_size > self.max_size {
            return;
        }

        // Remove old entry if exists
        if let Some(old) = self.entries.remove(&template_hash) {
            self.current_size -= old.data.len();
        }

        // Insert new entry
        let entry = TemplateCacheEntry::new(data, template_hash, source_hash);
        self.current_size += entry_size;
        self.entries.insert(template_hash, entry);
    }

    /// Remove stale entries
    pub fn invalidate_stale(&mut self, template_hash: u64, current_source_hash: u64) {
        if let Some(entry) = self.entries.get(&template_hash) {
            if entry.is_stale(current_source_hash) {
                if let Some(removed) = self.entries.remove(&template_hash) {
                    self.current_size -= removed.data.len();
                }
            }
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);

        CacheStats {
            entries: self.entries.len(),
            size_bytes: self.current_size,
            max_size_bytes: self.max_size,
            hits,
            misses,
            hit_rate: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }

    // Private helpers

    fn evict_one(&mut self) {
        // LRU eviction: find oldest entry
        let oldest = self.entries.iter().min_by_key(|(_, e)| e.last_access).map(|(k, _)| *k);

        if let Some(key) = oldest {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size -= entry.data.len();
            }
        }
    }
}

impl Default for HotCache {
    fn default() -> Self {
        // 64MB default cache
        Self::new(64 * 1024 * 1024)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached entries
    pub entries: usize,
    /// Current cache size in bytes
    pub size_bytes: usize,
    /// Maximum cache size in bytes
    pub max_size_bytes: usize,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit_miss() {
        let mut cache = HotCache::new(1024);

        cache.insert(1, 100, vec![1, 2, 3, 4]);

        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = HotCache::new(100);

        // Insert 10 entries of 20 bytes each = 200 bytes
        for i in 0..10 {
            cache.insert(i, i * 10, vec![0u8; 20]);
        }

        // Only 5 should fit (100 bytes / 20 bytes)
        assert!(cache.entries.len() <= 5);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = HotCache::new(1024);

        cache.insert(1, 100, vec![1, 2, 3]);

        // Same source hash - should stay
        cache.invalidate_stale(1, 100);
        assert!(cache.entries.contains_key(&1));

        // Different source hash - should be removed
        cache.invalidate_stale(1, 200);
        assert!(!cache.entries.contains_key(&1));
    }
}
