//! Cache implementation for binary responses.

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::query::{QueryId, QueryParams, RegisteredQuery};

/// A cached query result.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Pre-serialized binary data.
    pub data: Arc<Vec<u8>>,
    /// When this entry was created.
    pub created_at: Instant,
    /// Time-to-live for this entry.
    pub ttl: Option<Duration>,
}

impl CacheEntry {
    /// Create a new cache entry.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(data),
            created_at: Instant::now(),
            ttl: None,
        }
    }

    /// Create a new cache entry with TTL.
    pub fn with_ttl(data: Vec<u8>, ttl: Duration) -> Self {
        Self {
            data: Arc::new(data),
            created_at: Instant::now(),
            ttl: Some(ttl),
        }
    }

    /// Check if this entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            self.created_at.elapsed() > ttl
        } else {
            false
        }
    }

    /// Get the binary data.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the age of this entry.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Thread-safe query cache using DashMap.
pub struct QueryCache {
    /// Registered queries by ID.
    queries: DashMap<QueryId, RegisteredQuery>,
    /// Cached results by cache key.
    cache: DashMap<String, CacheEntry>,
    /// Table to query mappings for invalidation.
    table_queries: DashMap<String, Vec<QueryId>>,
}

impl QueryCache {
    /// Create a new query cache.
    pub fn new() -> Self {
        Self {
            queries: DashMap::new(),
            cache: DashMap::new(),
            table_queries: DashMap::new(),
        }
    }

    /// Register a query with its table dependencies.
    pub fn register_query(&self, query: RegisteredQuery) {
        let query_id = query.id.clone();

        // Register table dependencies for invalidation
        for table in &query.table_dependencies {
            self.table_queries.entry(table.clone()).or_default().push(query_id.clone());
        }

        self.queries.insert(query_id, query);
    }

    /// Get a registered query by ID.
    pub fn get_query(&self, query_id: &str) -> Option<RegisteredQuery> {
        self.queries.get(query_id).map(|r| r.clone())
    }

    /// Get a cached result.
    pub fn get_cached(&self, query_id: &str, params_hash: QueryParams) -> Option<Arc<Vec<u8>>> {
        let query = self.queries.get(query_id)?;
        let cache_key = query.cache_key(params_hash);

        let entry = self.cache.get(&cache_key)?;

        // Check expiration
        if entry.is_expired() {
            drop(entry);
            self.cache.remove(&cache_key);
            return None;
        }

        Some(Arc::clone(&entry.data))
    }

    /// Store a result in the cache.
    pub fn set_cached(&self, query_id: &str, params_hash: QueryParams, data: Vec<u8>) {
        if let Some(query) = self.queries.get(query_id) {
            let cache_key = query.cache_key(params_hash);
            self.cache.insert(cache_key, CacheEntry::new(data));
        }
    }

    /// Store a result in the cache with TTL.
    pub fn set_cached_with_ttl(
        &self,
        query_id: &str,
        params_hash: QueryParams,
        data: Vec<u8>,
        ttl: Duration,
    ) {
        if let Some(query) = self.queries.get(query_id) {
            let cache_key = query.cache_key(params_hash);
            self.cache.insert(cache_key, CacheEntry::with_ttl(data, ttl));
        }
    }

    /// Invalidate all cached results for queries depending on a table.
    pub fn invalidate_table(&self, table: &str) {
        if let Some(query_ids) = self.table_queries.get(table) {
            for query_id in query_ids.iter() {
                self.invalidate_query(query_id);
            }
        }
    }

    /// Invalidate all cached results for a specific query.
    pub fn invalidate_query(&self, query_id: &str) {
        // Remove all cache entries that start with this query ID
        let prefix = format!("{}:", query_id);
        self.cache.retain(|key, _| !key.starts_with(&prefix));
    }

    /// Clear all cached results.
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get the number of registered queries.
    pub fn num_queries(&self) -> usize {
        self.queries.len()
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new(vec![1, 2, 3]);
        assert_eq!(entry.as_bytes(), &[1, 2, 3]);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cache_entry_with_ttl() {
        let entry = CacheEntry::with_ttl(vec![1, 2, 3], Duration::from_millis(1));
        assert!(!entry.is_expired());

        std::thread::sleep(Duration::from_millis(2));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_query_cache_basic() {
        let cache = QueryCache::new();

        let query = RegisteredQuery::new("test", "SELECT 1", &["test_table"]);
        cache.register_query(query);

        // Cache miss
        assert!(cache.get_cached("test", 0).is_none());

        // Set and get
        cache.set_cached("test", 0, vec![1, 2, 3]);
        let result = cache.get_cached("test", 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn test_query_cache_invalidation() {
        let cache = QueryCache::new();

        let query = RegisteredQuery::new("test", "SELECT 1", &["users"]);
        cache.register_query(query);

        cache.set_cached("test", 0, vec![1, 2, 3]);
        assert!(cache.get_cached("test", 0).is_some());

        // Invalidate by table
        cache.invalidate_table("users");
        assert!(cache.get_cached("test", 0).is_none());
    }
}
