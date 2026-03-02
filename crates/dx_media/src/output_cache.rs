//! Output cache — caches media generation results to avoid re-generating identical outputs.
//!
//! Identical prompt + settings → cached result. Saves cost and time.

use dx_core::{MediaGenerationRequest, MediaOutput, MicroCost};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache key derived from a generation request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub model: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub duration_seconds: Option<u64>, // Rounded to avoid float hashing issues
    pub seed: Option<u64>,
    pub style: Option<String>,
}

impl CacheKey {
    /// Create a cache key from a media generation request.
    pub fn from_request(request: &MediaGenerationRequest) -> Self {
        Self {
            model: request.model.clone(),
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            dimensions: request.dimensions,
            duration_seconds: request.duration_seconds.map(|d| d as u64),
            seed: request.seed,
            style: request.style.clone(),
        }
    }
}

/// A cached media output entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// MIME type of the cached output.
    pub mime_type: String,
    /// File extension.
    pub extension: String,
    /// Cost of the original generation.
    pub original_cost: MicroCost,
    /// On-disk file path (we don't cache bytes in memory to save RAM).
    pub file_path: String,
    /// Unix timestamp of when this was cached.
    pub cached_at: u64,
    /// Number of times this cached result has been served.
    pub hit_count: u32,
}

/// Media output cache — stores and retrieves cached generation results.
pub struct OutputCache {
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    max_entries: usize,
    total_cost_saved: RwLock<MicroCost>,
}

impl OutputCache {
    /// Create a new output cache with maximum entry count.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            total_cost_saved: RwLock::new(MicroCost::ZERO),
        }
    }

    /// Look up a cached result for a request.
    pub fn get(&self, request: &MediaGenerationRequest) -> Option<CacheEntry> {
        let key = CacheKey::from_request(request);
        let entries = self.entries.read();
        if let Some(entry) = entries.get(&key) {
            let mut result = entry.clone();
            result.hit_count += 1;
            // Track cost savings
            *self.total_cost_saved.write() += entry.original_cost;
            Some(result)
        } else {
            None
        }
    }

    /// Store a generation result in the cache.
    pub fn put(&self, request: &MediaGenerationRequest, output: &MediaOutput, file_path: String) {
        let key = CacheKey::from_request(request);
        let entry = CacheEntry {
            mime_type: output.mime_type.clone(),
            extension: output.extension.clone(),
            original_cost: output.cost,
            file_path,
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            hit_count: 0,
        };

        let mut entries = self.entries.write();

        // Evict least-recently-used if at capacity
        if entries.len() >= self.max_entries {
            if let Some(lru_key) = entries
                .iter()
                .min_by_key(|(_, e)| e.cached_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&lru_key);
            }
        }

        entries.insert(key, entry);
    }

    /// Total cost saved by cache hits.
    pub fn total_cost_saved(&self) -> MicroCost {
        *self.total_cost_saved.read()
    }

    /// Number of entries in the cache.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.entries.write().clear();
    }
}

impl Default for OutputCache {
    fn default() -> Self {
        Self::new(1000)
    }
}
