//! Search orchestrator — coordinates cache, engines, and result aggregation.
//!
//! Architecture:
//! 1. Check 3-layer cache (hit/coalesce/miss)
//! 2. Fan-out to all engines for the category using `FuturesUnordered`
//!    — results stream in as engines respond (fastest first)
//! 3. Aggregate with `FxHashMap` (2-3× faster than std HashMap for short keys)
//! 4. Sort with `sort_unstable_by` (faster than stable sort, no rayon needed
//!    for < 1 000 items — rayon overhead > sort cost at this scale)
//! 5. Store in cache wrapped in `Arc` (zero-copy sharing between concurrent readers)

use std::sync::Arc;
use std::time::Instant;

use futures::stream::{FuturesUnordered, StreamExt};
use metasearch_core::{
    category::SearchCategory,
    engine::SearchEngine,
    query::SearchQuery,
    result::{SearchResponse, SearchResult},
};
use metasearch_engine::EngineRegistry;
use rustc_hash::FxHashMap;

use crate::cache::{CacheLookup, SearchCache};
use crate::health::EngineHealthTracker;

/// Central coordinator for all search operations.
pub struct SearchOrchestrator {
    pub registry: Arc<EngineRegistry>,
    pub cache: SearchCache,
    pub health: Arc<EngineHealthTracker>,
    /// Max engines to fan-out to per request (prevents thundering herd).
    pub max_engines: usize,
}

impl SearchOrchestrator {
    pub fn new(
        registry: Arc<EngineRegistry>,
        cache: SearchCache,
        health: Arc<EngineHealthTracker>,
        max_engines: usize,
    ) -> Self {
        Self {
            registry,
            cache,
            health,
            max_engines,
        }
    }

    /// Execute a search, using the cache when possible.
    pub async fn search(
        &self,
        query: &SearchQuery,
        cache_key: &str,
    ) -> Arc<SearchResponse> {
        match self.cache.get_or_coalesce(cache_key).await {
            CacheLookup::Hit(r) => {
                tracing::debug!(key = cache_key, "Cache hit");
                r
            }
            CacheLookup::Coalesced(r) => {
                tracing::debug!(key = cache_key, "Coalesced with in-flight request");
                r
            }
            CacheLookup::Miss(guard) => {
                let response = Arc::new(self.execute_search(query).await);
                guard.complete(Arc::clone(&response)).await;
                response
            }
        }
    }

    /// Fan-out to all eligible engines and aggregate results.
    async fn execute_search(&self, query: &SearchQuery) -> SearchResponse {
        let start = Instant::now();

        let category = query
            .categories
            .first()
            .copied()
            .unwrap_or(SearchCategory::General);

        // Get engines for this category, skip circuit-broken ones
        let engines: Vec<Arc<dyn SearchEngine>> = self
            .registry
            .engines_for_category(&category)
            .into_iter()
            .filter(|e| {
                let meta = e.metadata();
                meta.enabled && self.health.is_healthy(meta.name.as_ref())
            })
            .take(self.max_engines)
            .collect();

        let engines_queried = engines.len();

        // ── Fan-out: FuturesUnordered polls futures as they complete ─────────
        // For pure I/O tasks (HTTP) this avoids spawning N Tokio tasks —
        // no scheduling overhead, no thread-pool cross-talk.
        let mut futures = FuturesUnordered::new();

        for engine in engines {
            let query = query.clone();
            let health = Arc::clone(&self.health);

            futures.push(async move {
                let meta = engine.metadata();
                let name = meta.name.as_ref().to_owned();
                let timeout_dur = health.timeout_for(&name, meta.timeout_ms);
                let weight = meta.weight;

                let t0 = Instant::now();
                let result = tokio::time::timeout(timeout_dur, engine.search(&query)).await;
                let latency_ms = t0.elapsed().as_millis() as u32;

                match result {
                    Ok(Ok(results)) => {
                        health.record_success(&name, latency_ms);
                        tracing::debug!(engine = %name, count = results.len(), latency_ms, "Engine responded");
                        (name, Some((weight, results)))
                    }
                    Ok(Err(e)) => {
                        health.record_failure(&name);
                        tracing::warn!(engine = %name, error = %e, "Engine error");
                        (name, None)
                    }
                    Err(_) => {
                        health.record_failure(&name);
                        tracing::warn!(engine = %name, timeout_ms = timeout_dur.as_millis(), "Engine timeout");
                        (name, None)
                    }
                }
            });
        }

        // ── Aggregate: stream results as they arrive ─────────────────────────
        // FxHashMap: 2-3× faster than std HashMap for short string keys.
        let mut url_map: FxHashMap<String, SearchResult> =
            FxHashMap::with_capacity_and_hasher(256, Default::default());
        let mut engines_used: Vec<String> = Vec::new();
        let mut engines_failed: Vec<String> = Vec::new();

        while let Some((engine_name, outcome)) = futures.next().await {
            match outcome {
                Some((weight, results)) => {
                    engines_used.push(engine_name.clone());
                    for mut r in results {
                        let score = weight * (1.0 / (r.engine_rank as f64 + 1.0));
                        let normalized = normalize_url(&r.url);
                        url_map
                            .entry(normalized)
                            .and_modify(|existing| {
                                existing.score += score;
                            })
                            .or_insert_with(|| {
                                r.score = score;
                                r
                            });
                    }
                }
                None => {
                    // Engine failed or timed out — preserve the actual engine name
                    engines_failed.push(engine_name);
                }
            }
        }

        // ── Sort: single-threaded is faster for < 1 000 items ────────────────
        // rayon overhead (~50 µs) >> sort cost (~5 µs) for typical result sets.
        let mut results: Vec<SearchResult> = url_map.into_values().collect();
        results.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let number_of_results = results.len();
        let search_time_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            engines_queried,
            engines_responded = engines_used.len(),
            results = number_of_results,
            search_time_ms,
            "Search complete"
        );

        SearchResponse {
            query: query.query.clone(),
            results,
            number_of_results,
            engines_used,
            engines_failed,
            search_time_ms,
        }
    }
}

/// Normalize a URL for deduplication (strip fragment, normalise scheme/trailing slash).
fn normalize_url(raw: &str) -> String {
    // Fast path: avoid URL parsing overhead for most cases
    let s = raw.trim_end_matches('/');
    if let Some(no_frag) = s.split('#').next() {
        no_frag.to_ascii_lowercase()
    } else {
        s.to_ascii_lowercase()
    }
}
