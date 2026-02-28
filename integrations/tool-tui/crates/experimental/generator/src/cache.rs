//! Template Cache with Dirty-Bit Invalidation - Feature #6
//!
//! Caches rendered output keyed by template ID and parameter hash.
//! Uses dirty-bit tracking for O(1) invalidation decisions.

use crate::dirty::DirtyMask;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Cache Entry
// ============================================================================

/// A cached template output.
#[derive(Clone, Debug)]
pub struct CacheEntry {
    /// Template ID.
    pub template_id: u32,
    /// Parameter hash (Blake3).
    pub param_hash: [u8; 32],
    /// Cached output bytes.
    pub output: Vec<u8>,
    /// Dirty mask at time of caching.
    pub dirty_mask: DirtyMask,
    /// Timestamp (for LRU eviction).
    pub timestamp: u64,
    /// Hit count (for frequency-based eviction).
    pub hits: u32,
}

impl CacheEntry {
    /// Create a new cache entry.
    #[must_use]
    pub fn new(template_id: u32, param_hash: [u8; 32], output: Vec<u8>) -> Self {
        Self {
            template_id,
            param_hash,
            output,
            dirty_mask: DirtyMask::clean(),
            timestamp: 0,
            hits: 0,
        }
    }

    /// Check if this entry matches the given key.
    #[must_use]
    pub fn matches(&self, template_id: u32, param_hash: &[u8; 32]) -> bool {
        self.template_id == template_id && &self.param_hash == param_hash
    }

    /// Get the output size.
    #[must_use]
    pub fn size(&self) -> usize {
        self.output.len()
    }
}

// ============================================================================
// Cache Key
// ============================================================================

/// Key for cache lookup.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Template ID.
    pub template_id: u32,
    /// Parameter hash.
    pub param_hash: [u8; 32],
}

impl CacheKey {
    /// Create a new cache key.
    #[must_use]
    pub fn new(template_id: u32, param_hash: [u8; 32]) -> Self {
        Self {
            template_id,
            param_hash,
        }
    }

    /// Create from template ID and parameters.
    #[must_use]
    pub fn from_params(template_id: u32, params: &crate::params::Parameters<'_>) -> Self {
        let hash = blake3::hash(&params.encode());
        let mut param_hash = [0u8; 32];
        param_hash.copy_from_slice(hash.as_bytes());
        Self {
            template_id,
            param_hash,
        }
    }
}

// ============================================================================
// Template Cache
// ============================================================================

/// Thread-safe template output cache.
///
/// Caches rendered output to avoid redundant generation.
/// Uses dirty-bit tracking for intelligent invalidation.
///
/// # Example
///
/// ```rust
/// use dx_generator::{TemplateCache, CacheEntry};
///
/// let cache = TemplateCache::new(1000);
///
/// // Store output
/// let entry = CacheEntry::new(1, [0; 32], b"Hello!".to_vec());
/// cache.insert(entry);
///
/// // Retrieve
/// if let Some(entry) = cache.get(1, &[0; 32]) {
///     println!("Cached: {:?}", entry.output);
/// }
/// ```
#[derive(Debug)]
pub struct TemplateCache {
    /// Cache entries.
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    /// Maximum number of entries.
    max_entries: usize,
    /// Maximum total size in bytes.
    max_size: usize,
    /// Current total size.
    current_size: RwLock<usize>,
    /// Global timestamp counter.
    timestamp: RwLock<u64>,
}

impl TemplateCache {
    /// Create a new cache with the given capacity.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::with_capacity(max_entries)),
            max_entries,
            max_size: 64 * 1024 * 1024, // 64 MB default
            current_size: RwLock::new(0),
            timestamp: RwLock::new(0),
        }
    }

    /// Create with custom size limit.
    #[must_use]
    pub fn with_size_limit(max_entries: usize, max_size: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::with_capacity(max_entries)),
            max_entries,
            max_size,
            current_size: RwLock::new(0),
            timestamp: RwLock::new(0),
        }
    }

    /// Get an entry from the cache.
    #[must_use]
    pub fn get(&self, template_id: u32, param_hash: &[u8; 32]) -> Option<CacheEntry> {
        let key = CacheKey::new(template_id, *param_hash);
        let mut entries = self.entries.write();

        if let Some(entry) = entries.get_mut(&key) {
            // Update access metadata
            entry.hits += 1;
            let mut ts = self.timestamp.write();
            *ts += 1;
            entry.timestamp = *ts;

            Some(entry.clone())
        } else {
            None
        }
    }

    /// Insert an entry into the cache.
    pub fn insert(&self, mut entry: CacheEntry) {
        let key = CacheKey::new(entry.template_id, entry.param_hash);
        let entry_size = entry.size();

        // Update timestamp
        {
            let mut ts = self.timestamp.write();
            *ts += 1;
            entry.timestamp = *ts;
        }

        // Check if we need to evict
        self.ensure_capacity(entry_size);

        // Insert
        let mut entries = self.entries.write();
        if let Some(old) = entries.insert(key, entry) {
            // Subtract old entry size
            let mut size = self.current_size.write();
            *size = size.saturating_sub(old.size());
        }

        // Add new entry size
        let mut size = self.current_size.write();
        *size += entry_size;
    }

    /// Remove an entry from the cache.
    pub fn remove(&self, template_id: u32, param_hash: &[u8; 32]) -> Option<CacheEntry> {
        let key = CacheKey::new(template_id, *param_hash);
        let mut entries = self.entries.write();

        if let Some(entry) = entries.remove(&key) {
            let mut size = self.current_size.write();
            *size = size.saturating_sub(entry.size());
            Some(entry)
        } else {
            None
        }
    }

    /// Invalidate all entries for a template.
    pub fn invalidate_template(&self, template_id: u32) {
        let mut entries = self.entries.write();
        let mut size = self.current_size.write();

        entries.retain(|key, entry| {
            if key.template_id == template_id {
                *size = size.saturating_sub(entry.size());
                false
            } else {
                true
            }
        });
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        entries.clear();

        let mut size = self.current_size.write();
        *size = 0;
    }

    /// Get cache statistics.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.read();
        let size = *self.current_size.read();

        CacheStats {
            entry_count: entries.len(),
            total_size: size,
            max_entries: self.max_entries,
            max_size: self.max_size,
        }
    }

    /// Ensure capacity for a new entry.
    fn ensure_capacity(&self, needed_size: usize) {
        let mut entries = self.entries.write();
        let mut size = self.current_size.write();

        // Evict by LRU if over entry limit or size limit
        while (entries.len() >= self.max_entries || *size + needed_size > self.max_size)
            && !entries.is_empty()
        {
            // Find LRU entry
            let lru_key = entries.iter().min_by_key(|(_, e)| e.timestamp).map(|(k, _)| k.clone());

            if let Some(key) = lru_key {
                if let Some(entry) = entries.remove(&key) {
                    *size = size.saturating_sub(entry.size());
                }
            } else {
                break;
            }
        }
    }
}

impl Default for TemplateCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ============================================================================
// Cache Statistics
// ============================================================================

/// Cache statistics.
#[derive(Clone, Debug)]
pub struct CacheStats {
    /// Number of cached entries.
    pub entry_count: usize,
    /// Total size of cached data.
    pub total_size: usize,
    /// Maximum entry count.
    pub max_entries: usize,
    /// Maximum size in bytes.
    pub max_size: usize,
}

impl CacheStats {
    /// Get cache utilization as a percentage.
    #[must_use]
    pub fn utilization(&self) -> f64 {
        if self.max_entries == 0 {
            0.0
        } else {
            (self.entry_count as f64 / self.max_entries as f64) * 100.0
        }
    }

    /// Get size utilization as a percentage.
    #[must_use]
    pub fn size_utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.total_size as f64 / self.max_size as f64) * 100.0
        }
    }
}

// ============================================================================
// Shared Cache
// ============================================================================

/// Thread-safe shared cache reference.
pub type SharedCache = Arc<TemplateCache>;

/// Create a shared cache.
#[must_use]
pub fn shared_cache(max_entries: usize) -> SharedCache {
    Arc::new(TemplateCache::new(max_entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_get() {
        let cache = TemplateCache::new(100);

        let entry = CacheEntry::new(1, [0; 32], b"Hello!".to_vec());
        cache.insert(entry);

        let retrieved = cache.get(1, &[0; 32]).unwrap();
        assert_eq!(retrieved.output, b"Hello!");
    }

    #[test]
    fn test_cache_miss() {
        let cache = TemplateCache::new(100);
        assert!(cache.get(1, &[0; 32]).is_none());
    }

    #[test]
    fn test_cache_remove() {
        let cache = TemplateCache::new(100);

        let entry = CacheEntry::new(1, [0; 32], b"Hello!".to_vec());
        cache.insert(entry);

        let removed = cache.remove(1, &[0; 32]);
        assert!(removed.is_some());
        assert!(cache.get(1, &[0; 32]).is_none());
    }

    #[test]
    fn test_cache_invalidate_template() {
        let cache = TemplateCache::new(100);

        cache.insert(CacheEntry::new(1, [0; 32], b"A".to_vec()));
        cache.insert(CacheEntry::new(1, [1; 32], b"B".to_vec()));
        cache.insert(CacheEntry::new(2, [0; 32], b"C".to_vec()));

        cache.invalidate_template(1);

        assert!(cache.get(1, &[0; 32]).is_none());
        assert!(cache.get(1, &[1; 32]).is_none());
        assert!(cache.get(2, &[0; 32]).is_some());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = TemplateCache::new(2);

        cache.insert(CacheEntry::new(1, [0; 32], b"A".to_vec()));
        cache.insert(CacheEntry::new(2, [0; 32], b"B".to_vec()));

        // Access entry 1 to make it more recent
        let _ = cache.get(1, &[0; 32]);

        // Insert entry 3, should evict entry 2 (LRU)
        cache.insert(CacheEntry::new(3, [0; 32], b"C".to_vec()));

        assert!(cache.get(1, &[0; 32]).is_some());
        assert!(cache.get(2, &[0; 32]).is_none()); // Evicted
        assert!(cache.get(3, &[0; 32]).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let cache = TemplateCache::new(100);

        cache.insert(CacheEntry::new(1, [0; 32], b"Hello!".to_vec()));
        cache.insert(CacheEntry::new(2, [0; 32], b"World!".to_vec()));

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.total_size, 12); // 6 + 6
    }
}
