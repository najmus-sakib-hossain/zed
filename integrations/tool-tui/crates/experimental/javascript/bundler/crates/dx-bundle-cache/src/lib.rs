//! Persistent warm cache for instant rebuilds
//!
//! Memory-map transformed modules, skip unchanged files

pub mod persistent;
pub mod warm;

pub use persistent::PersistentCache;
pub use warm::WarmCache;

use dx_bundle_core::ContentHash;
use std::path::Path;

/// Cached transform data
#[derive(Clone)]
pub struct CachedTransform {
    /// Content hash for validation
    pub content_hash: ContentHash,
    /// Transformed output
    pub transformed: Vec<u8>,
    /// Import dependencies
    pub imports: Vec<u64>,
    /// Last modified time
    pub mtime: u64,
}

impl CachedTransform {
    /// Check if cache entry is still valid
    pub fn is_valid(&self, source_path: &Path) -> bool {
        // Check if file still exists and has same hash
        match ContentHash::hash_file(source_path) {
            Ok(hash) => hash == self.content_hash,
            Err(_) => false,
        }
    }
}

/// Cache statistics
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: usize,
    /// Total cache misses
    pub misses: usize,
    /// Total bytes saved by cache
    pub bytes_saved: usize,
    /// Cache file size
    pub cache_size: usize,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}
