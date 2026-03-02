//! Multi-layer result caching with request coalescing.
//!
//! Three-layer caching system:
//! 1. Full response cache (query → Arc<SearchResponse>) with TinyLFU eviction
//! 2. Per-engine result cache for partial cache hits
//! 3. Request coalescing — deduplicate identical in-flight queries via broadcast

use std::sync::Arc;
use std::time::Duration;

use ahash::RandomState;
use dashmap::DashMap;
use metasearch_core::result::SearchResponse;
use moka::future::Cache;
use tokio::sync::broadcast;

/// Cache lookup result.
pub enum CacheLookup {
    /// Response was in cache ─ return immediately.
    Hit(Arc<SearchResponse>),
    /// Another task is already fetching this query ─ wait for its result.
    Coalesced(Arc<SearchResponse>),
    /// Cache miss ─ caller must execute the search and call `InflightGuard::complete`.
    Miss(InflightGuard),
}

/// Returned on a cache miss; caller must call `complete` when the search finishes.
pub struct InflightGuard {
    key: String,
    sender: broadcast::Sender<Arc<SearchResponse>>,
    // Weak back-reference so the guard can store results
    inflight: Arc<DashMap<String, broadcast::Sender<Arc<SearchResponse>>>>,
    response_cache: Cache<String, Arc<SearchResponse>, RandomState>,
}

impl InflightGuard {
    /// Store the completed response in the cache and notify any coalesced waiters.
    pub async fn complete(self, response: Arc<SearchResponse>) {
        self.response_cache
            .insert(self.key.clone(), Arc::clone(&response))
            .await;
        self.inflight.remove(&self.key);
        // Ignore errors — no listeners is OK (they may have timed out)
        let _ = self.sender.send(response);
    }
}

/// Three-layer concurrent search cache.
#[derive(Clone)]
pub struct SearchCache {
    /// Layer 1: Full response cache.
    /// TinyLFU eviction + TTL + TTI.   ahash hasher (2–3x faster than SipHash).
    response_cache: Cache<String, Arc<SearchResponse>, RandomState>,

    /// Layer 3: In-flight request registry for coalescing.
    inflight: Arc<DashMap<String, broadcast::Sender<Arc<SearchResponse>>>>,
}

impl SearchCache {
    pub fn new(max_capacity: u64, ttl_secs: u64) -> Self {
        // TinyLFU is the default policy — ideal for search/database workloads.
        let response_cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_secs))
            // Evict idle entries after 60 s to reclaim memory even within TTL
            .time_to_idle(Duration::from_secs(60))
            .build_with_hasher(RandomState::new());

        Self {
            response_cache,
            inflight: Arc::new(DashMap::new()),
        }
    }

    /// Build a cache key from query parameters.
    pub fn cache_key(query: &str, category: &str, page: u32, lang: &str) -> String {
        format!("{query}:{category}:{page}:{lang}")
    }

    /// Try to retrieve a cached response, coalesce with an in-flight request,
    /// or return a `Miss` guard that the caller must complete.
    pub async fn get_or_coalesce(&self, key: &str) -> CacheLookup {
        // 1. Fast path: response already cached (sub-microsecond lookup).
        if let Some(cached) = self.response_cache.get(key).await {
            return CacheLookup::Hit(cached);
        }

        // 2. Someone is already fetching this key — subscribe and wait.
        if let Some(sender) = self.inflight.get(key) {
            let mut rx = sender.subscribe();
            drop(sender); // Release DashMap shard lock before awaiting
            if let Ok(response) = rx.recv().await {
                return CacheLookup::Coalesced(response);
            }
        }

        // 3. Cache miss — register as in-flight so concurrent identical
        //    requests can coalesce onto this one.
        let (tx, _) = broadcast::channel(1);
        self.inflight.insert(key.to_string(), tx.clone());

        CacheLookup::Miss(InflightGuard {
            key: key.to_string(),
            sender: tx,
            inflight: Arc::clone(&self.inflight),
            response_cache: self.response_cache.clone(),
        })
    }

    // ─── Backward-compatible simple API ─────────────────────────────────────

    /// Get a cached response (clones out of Arc for backward compat).
    pub async fn get(&self, key: &str) -> Option<SearchResponse> {
        self.response_cache
            .get(key)
            .await
            .map(|arc| (*arc).clone())
    }

    /// Insert a response into the cache.
    pub async fn insert(&self, key: String, response: SearchResponse) {
        let arc = Arc::new(response);
        self.response_cache.insert(key.clone(), Arc::clone(&arc)).await;
        // Notify any coalesced waiters
        if let Some((_, sender)) = self.inflight.remove(&key) {
            let _ = sender.send(arc);
        }
    }
}
