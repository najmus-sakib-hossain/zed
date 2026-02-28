//! # semantic-cache
//!
//! In-memory semantic similarity cache — if a similar query was
//! recently answered, return the cached response without an API call.
//!
//! ## Evidence (TOKEN.md ⚠️ Partly Real per hit)
//! - 100% savings per cache hit (skip entire API call)
//! - BUT hit rates for coding agents are 5-15% (every task is different)
//! - Customer support chatbots: 20-69% hit rates
//! - Vector drift and model updates can break matches
//! - **Honest savings: 100% per hit × 5-15% hit rate = 5-15% overall**
//!
//! STAGE: CallElimination (priority 1)

use dx_core::*;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Configuration for semantic caching.
#[derive(Debug, Clone)]
pub struct SemanticCacheConfig {
    /// Maximum number of cached entries
    pub max_entries: usize,
    /// Similarity threshold (0.0-1.0). Higher = stricter matching.
    /// For agents, use ≥0.95 to avoid false positives.
    pub similarity_threshold: f64,
    /// Cache TTL — entries older than this are evicted
    pub ttl: Duration,
    /// Minimum query length to cache (very short queries are too ambiguous)
    pub min_query_tokens: usize,
    /// Whether system prompt must also match for a cache hit
    pub require_system_match: bool,
}

impl Default for SemanticCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            similarity_threshold: 0.95, // High threshold for agents
            ttl: Duration::from_secs(3600), // 1 hour
            min_query_tokens: 10,
            require_system_match: true,
        }
    }
}

/// A cached entry.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Normalized query text
    query: String,
    /// System prompt hash (for matching context)
    system_hash: u64,
    /// The cached response
    response: String,
    /// Word set for Jaccard similarity
    word_set: Vec<String>,
    /// When this entry was cached
    created: Instant,
    /// Number of times this entry was hit
    hit_count: u64,
}

pub struct SemanticCacheSaver {
    config: SemanticCacheConfig,
    cache: Mutex<Vec<CacheEntry>>,
    stats: Mutex<CacheStats>,
    report: Mutex<TokenSavingsReport>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_queries == 0 { return 0.0; }
        self.cache_hits as f64 / self.total_queries as f64 * 100.0
    }
}

impl SemanticCacheSaver {
    pub fn new() -> Self {
        Self::with_config(SemanticCacheConfig::default())
    }

    pub fn with_config(config: SemanticCacheConfig) -> Self {
        Self {
            config,
            cache: Mutex::new(Vec::new()),
            stats: Mutex::new(CacheStats::default()),
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Store a response in the cache (call after getting API response).
    pub fn store(&self, query: &str, system_prompt: &str, response: &str) {
        let normalized = Self::normalize(query);
        let word_set = Self::word_set(&normalized);
        let system_hash = Self::hash_text(system_prompt);

        let mut cache = self.cache.lock().unwrap();

        // Evict expired entries
        let now = Instant::now();
        cache.retain(|e| now.duration_since(e.created) < self.config.ttl);

        // Evict oldest if at capacity
        if cache.len() >= self.config.max_entries {
            let mut stats = self.stats.lock().unwrap();
            stats.evictions += 1;
            cache.remove(0); // remove oldest
        }

        cache.push(CacheEntry {
            query: normalized,
            system_hash,
            response: response.to_string(),
            word_set,
            created: now,
            hit_count: 0,
        });
    }

    /// Normalize a query for comparison.
    fn normalize(text: &str) -> String {
        text.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Extract word set for Jaccard similarity.
    fn word_set(text: &str) -> Vec<String> {
        let mut words: Vec<String> = text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty() && w.len() > 2)
            .collect();
        words.sort();
        words.dedup();
        words
    }

    /// Jaccard similarity between two word sets.
    fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
        if a.is_empty() && b.is_empty() { return 1.0; }
        if a.is_empty() || b.is_empty() { return 0.0; }

        let mut intersection = 0usize;
        let mut union = b.len();

        for word in a {
            if b.contains(word) {
                intersection += 1;
            } else {
                union += 1;
            }
        }

        intersection as f64 / union as f64
    }

    /// Simple string hash.
    fn hash_text(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Find a cached response for the given query.
    fn find_match(&self, query: &str, system_hash: u64) -> Option<(String, f64)> {
        let normalized = Self::normalize(query);
        let word_set = Self::word_set(&normalized);
        let now = Instant::now();

        let mut cache = self.cache.lock().unwrap();
        let mut best_match: Option<(usize, f64)> = None;
        let mut exact_match_idx: Option<usize> = None;

        for (i, entry) in cache.iter().enumerate() {
            // Skip expired
            if now.duration_since(entry.created) >= self.config.ttl {
                continue;
            }

            // Check system prompt match if required
            if self.config.require_system_match && entry.system_hash != system_hash {
                continue;
            }

            // Check exact match first (fast path)
            if entry.query == normalized {
                exact_match_idx = Some(i);
                break;
            }

            // Jaccard similarity
            let sim = Self::jaccard_similarity(&word_set, &entry.word_set);
            if sim >= self.config.similarity_threshold {
                if best_match.map_or(true, |(_, best_sim)| sim > best_sim) {
                    best_match = Some((i, sim));
                }
            }
        }

        if let Some(idx) = exact_match_idx {
            cache[idx].hit_count += 1;
            let response = cache[idx].response.clone();
            return Some((response, 1.0));
        }

        if let Some((idx, sim)) = best_match {
            cache[idx].hit_count += 1;
            Some((cache[idx].response.clone(), sim))
        } else {
            None
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for SemanticCacheSaver {
    fn name(&self) -> &str { "semantic-cache" }
    fn stage(&self) -> SaverStage { SaverStage::CallElimination }
    fn priority(&self) -> u32 { 1 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();

        // Extract the latest user query
        let user_query = input.messages.iter().rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str());

        let query = match user_query {
            Some(q) if q.len() / 4 >= self.config.min_query_tokens => q,
            _ => {
                // Query too short to cache meaningfully
                let report = TokenSavingsReport {
                    technique: "semantic-cache".into(),
                    tokens_before,
                    tokens_after: tokens_before,
                    tokens_saved: 0,
                    description: "Query too short for semantic caching.".into(),
                };
                *self.report.lock().unwrap() = report;
                return Ok(SaverOutput {
                    messages: input.messages,
                    tools: input.tools,
                    images: input.images,
                    skipped: false,
                    cached_response: None,
                });
            }
        };

        // Get system prompt hash
        let system_prompt = input.messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let system_hash = Self::hash_text(system_prompt);

        let mut stats = self.stats.lock().unwrap();
        stats.total_queries += 1;

        // Try to find a match
        match self.find_match(query, system_hash) {
            Some((response, similarity)) => {
                stats.cache_hits += 1;
                let hit_rate = stats.hit_rate();
                drop(stats);

                let report = TokenSavingsReport {
                    technique: "semantic-cache".into(),
                    tokens_before,
                    tokens_after: 0,
                    tokens_saved: tokens_before,
                    description: format!(
                        "CACHE HIT (similarity: {:.2}). Skipping API call entirely. \
                         Hit rate: {:.1}% ({} hits / {} queries).",
                        similarity, hit_rate,
                        self.stats().cache_hits, self.stats().total_queries
                    ),
                };
                *self.report.lock().unwrap() = report;

                Ok(SaverOutput {
                    messages: input.messages,
                    tools: input.tools,
                    images: input.images,
                    skipped: true,
                    cached_response: Some(response),
                })
            }
            None => {
                stats.cache_misses += 1;
                let hit_rate = stats.hit_rate();
                drop(stats);

                let report = TokenSavingsReport {
                    technique: "semantic-cache".into(),
                    tokens_before,
                    tokens_after: tokens_before,
                    tokens_saved: 0,
                    description: format!(
                        "CACHE MISS. No similar query found (threshold: {:.2}). \
                         Hit rate: {:.1}%.",
                        self.config.similarity_threshold, hit_rate
                    ),
                };
                *self.report.lock().unwrap() = report;

                Ok(SaverOutput {
                    messages: input.messages,
                    tools: input.tools,
                    images: input.images,
                    skipped: false,
                    cached_response: None,
                })
            }
        }
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(content: &str) -> Message {
        Message {
            role: "user".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: None,
            token_count: content.len() / 4,
        }
    }

    fn sys_msg(content: &str) -> Message {
        Message {
            role: "system".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: None,
            token_count: content.len() / 4,
        }
    }

    #[tokio::test]
    async fn test_cache_miss_then_hit() {
        let saver = SemanticCacheSaver::new();
        let ctx = SaverContext::default();

        let query = "how do I implement a binary search tree in rust with all the standard operations";
        let sys = "You are a helpful assistant";

        let input = SaverInput {
            messages: vec![sys_msg(sys), user_msg(query)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };

        // First call: miss
        let out = saver.process(input.clone(), &ctx).await.unwrap();
        assert!(!out.skipped);

        // Store a response
        saver.store(query, sys, "Here's how to implement a BST in Rust...");

        // Second call: hit (exact match)
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(out.skipped);
        assert!(out.cached_response.is_some());

        let stats = saver.stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
    }

    #[test]
    fn test_jaccard_similarity() {
        let a = vec!["hello".into(), "world".into(), "rust".into()];
        let b = vec!["hello".into(), "world".into(), "programming".into()];
        let sim = SemanticCacheSaver::jaccard_similarity(&a, &b);
        assert!(sim > 0.0 && sim < 1.0);
    }
}
