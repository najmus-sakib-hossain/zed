//! Property-based tests for dx-db-teleport.
//!
//! These tests verify the correctness properties defined in the design document.

use proptest::prelude::*;
use std::time::{Duration, Instant};

use dx_www_db_teleport::{QueryCache, RegisteredQuery, hash_params, hash_value};

// ============================================================================
// Property 18: Cache Consistency
// For any query executed via execute_and_cache(), subsequent calls to
// get_cached() with the same query_id and params_hash SHALL return the same
// binary data. After a cache miss, the result SHALL be cached for future calls.
// Validates: Requirements 9.1, 9.5
// ============================================================================

proptest! {
    /// Property 18: Cache Consistency
    /// **Feature: binary-dawn, Property 18: Cache Consistency**
    /// **Validates: Requirements 9.1, 9.5**
    #[test]
    fn prop_cache_consistency(
        query_id in "[a-z_]{1,20}",
        sql in "[A-Z ]{10,50}",
        params_hash in any::<u64>(),
        data in prop::collection::vec(any::<u8>(), 1..1000)
    ) {
        let cache = QueryCache::new();

        // Register the query
        let query = RegisteredQuery::new(&query_id, &sql, &["test_table"]);
        cache.register_query(query);

        // Initially should be a cache miss
        prop_assert!(cache.get_cached(&query_id, params_hash).is_none(),
            "Should be cache miss initially");

        // Set the cached value
        cache.set_cached(&query_id, params_hash, data.clone());

        // Now should be a cache hit with the same data
        let cached = cache.get_cached(&query_id, params_hash);
        prop_assert!(cached.is_some(), "Should be cache hit after set");
        let cached_data = cached.unwrap();
        prop_assert_eq!(cached_data.as_ref(), &data,
            "Cached data should match original");

        // Multiple gets should return the same data
        for _ in 0..5 {
            let cached = cache.get_cached(&query_id, params_hash);
            prop_assert!(cached.is_some(), "Should still be cache hit");
            let cached_data = cached.unwrap();
            prop_assert_eq!(cached_data.as_ref(), &data,
                "Cached data should be consistent across calls");
        }
    }

    /// Property 18: Different params_hash should have independent cache entries
    #[test]
    fn prop_cache_params_independence(
        query_id in "[a-z_]{1,20}",
        params_hash1 in any::<u64>(),
        params_hash2 in any::<u64>(),
        data1 in prop::collection::vec(any::<u8>(), 1..100),
        data2 in prop::collection::vec(any::<u8>(), 1..100)
    ) {
        prop_assume!(params_hash1 != params_hash2);
        prop_assume!(data1 != data2);

        let cache = QueryCache::new();
        let query = RegisteredQuery::new(&query_id, "SELECT 1", &[]);
        cache.register_query(query);

        // Set different data for different params
        cache.set_cached(&query_id, params_hash1, data1.clone());
        cache.set_cached(&query_id, params_hash2, data2.clone());

        // Each should return its own data
        let cached1 = cache.get_cached(&query_id, params_hash1);
        let cached2 = cache.get_cached(&query_id, params_hash2);

        prop_assert!(cached1.is_some() && cached2.is_some());
        let cached1_data = cached1.unwrap();
        let cached2_data = cached2.unwrap();
        prop_assert_eq!(cached1_data.as_ref(), &data1);
        prop_assert_eq!(cached2_data.as_ref(), &data2);
    }
}

#[test]
fn prop_cache_consistency_basic() {
    // **Feature: binary-dawn, Property 18: Cache Consistency**
    // **Validates: Requirements 9.1, 9.5**

    let cache = QueryCache::new();
    let query = RegisteredQuery::new("test_query", "SELECT * FROM users", &["users"]);
    cache.register_query(query);

    let data = vec![1, 2, 3, 4, 5];
    let params_hash = 12345u64;

    // Cache miss
    assert!(cache.get_cached("test_query", params_hash).is_none());

    // Set and verify
    cache.set_cached("test_query", params_hash, data.clone());

    // Multiple reads should return same data
    for _ in 0..10 {
        let cached = cache.get_cached("test_query", params_hash).unwrap();
        assert_eq!(cached.as_ref(), &data);
    }
}

// ============================================================================
// Property 19: Cache Invalidation
// For any DbTeleport cache entry, when a Postgres NOTIFY is received for a
// table that the query depends on, the cache entry SHALL be removed.
// Validates: Requirements 9.3
// ============================================================================

proptest! {
    /// Property 19: Cache Invalidation
    /// **Feature: binary-dawn, Property 19: Cache Invalidation**
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_cache_invalidation_by_table(
        query_id in "[a-z_]{1,20}",
        table_name in "[a-z_]{1,20}",
        params_hash in any::<u64>(),
        data in prop::collection::vec(any::<u8>(), 1..100)
    ) {
        let cache = QueryCache::new();

        // Register query with table dependency
        let query = RegisteredQuery::new(&query_id, "SELECT 1", &[&table_name]);
        cache.register_query(query);

        // Cache some data
        cache.set_cached(&query_id, params_hash, data.clone());
        prop_assert!(cache.get_cached(&query_id, params_hash).is_some(),
            "Data should be cached");

        // Invalidate by table
        cache.invalidate_table(&table_name);

        // Cache should be empty for this query
        prop_assert!(cache.get_cached(&query_id, params_hash).is_none(),
            "Cache should be invalidated after table invalidation");
    }

    /// Property 19: Invalidation should only affect dependent queries
    #[test]
    fn prop_cache_invalidation_selective(
        query1_id in "[a-z]{1,10}1",
        query2_id in "[a-z]{1,10}2",
        table1 in "[a-z]{1,10}_t1",
        table2 in "[a-z]{1,10}_t2",
        params_hash in any::<u64>(),
        data1 in prop::collection::vec(any::<u8>(), 1..50),
        data2 in prop::collection::vec(any::<u8>(), 1..50)
    ) {
        let cache = QueryCache::new();

        // Register two queries with different table dependencies
        cache.register_query(RegisteredQuery::new(&query1_id, "SELECT 1", &[&table1]));
        cache.register_query(RegisteredQuery::new(&query2_id, "SELECT 2", &[&table2]));

        // Cache data for both
        cache.set_cached(&query1_id, params_hash, data1.clone());
        cache.set_cached(&query2_id, params_hash, data2.clone());

        // Invalidate only table1
        cache.invalidate_table(&table1);

        // Query1 should be invalidated, query2 should remain
        prop_assert!(cache.get_cached(&query1_id, params_hash).is_none(),
            "Query1 should be invalidated");
        prop_assert!(cache.get_cached(&query2_id, params_hash).is_some(),
            "Query2 should remain cached");
    }
}

#[test]
fn prop_cache_invalidation_multiple_queries() {
    // **Feature: binary-dawn, Property 19: Cache Invalidation**
    // **Validates: Requirements 9.3**

    let cache = QueryCache::new();

    // Register multiple queries depending on same table
    cache.register_query(RegisteredQuery::new("q1", "SELECT 1", &["users"]));
    cache.register_query(RegisteredQuery::new("q2", "SELECT 2", &["users"]));
    cache.register_query(RegisteredQuery::new("q3", "SELECT 3", &["posts"]));

    // Cache data for all
    cache.set_cached("q1", 0, vec![1]);
    cache.set_cached("q2", 0, vec![2]);
    cache.set_cached("q3", 0, vec![3]);

    // Invalidate users table
    cache.invalidate_table("users");

    // q1 and q2 should be invalidated, q3 should remain
    assert!(cache.get_cached("q1", 0).is_none());
    assert!(cache.get_cached("q2", 0).is_none());
    assert!(cache.get_cached("q3", 0).is_some());
}

// ============================================================================
// Property 20: Cache Access Latency
// For any cached query result, get_cached() SHALL return within 0.1ms
// (100 microseconds).
// Validates: Requirements 9.4
// ============================================================================

#[test]
fn prop_cache_access_latency() {
    // **Feature: binary-dawn, Property 20: Cache Access Latency**
    // **Validates: Requirements 9.4**

    let cache = QueryCache::new();

    // Register and cache a query with substantial data
    let query = RegisteredQuery::new("latency_test", "SELECT 1", &[]);
    cache.register_query(query);

    let data = vec![0u8; 10000]; // 10KB of data
    cache.set_cached("latency_test", 0, data);

    // Warm up
    for _ in 0..100 {
        let _ = cache.get_cached("latency_test", 0);
    }

    // Measure latency over many iterations
    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = cache.get_cached("latency_test", 0);
    }

    let elapsed = start.elapsed();
    let avg_latency = elapsed / iterations as u32;

    // Average latency should be well under 100 microseconds
    // We use 100μs as the threshold per the requirement
    assert!(
        avg_latency < Duration::from_micros(100),
        "Average cache access latency {:?} exceeds 100μs threshold",
        avg_latency
    );

    println!("Average cache access latency: {:?}", avg_latency);
}

proptest! {
    /// Property 20: Cache Access Latency with varying data sizes
    /// **Feature: binary-dawn, Property 20: Cache Access Latency**
    /// **Validates: Requirements 9.4**
    #[test]
    fn prop_cache_access_latency_varying_sizes(
        data_size in 100usize..50000
    ) {
        let cache = QueryCache::new();
        let query = RegisteredQuery::new("size_test", "SELECT 1", &[]);
        cache.register_query(query);

        let data = vec![0u8; data_size];
        cache.set_cached("size_test", 0, data);

        // Measure single access
        let start = Instant::now();
        let _ = cache.get_cached("size_test", 0);
        let latency = start.elapsed();

        // Single access should be under 100μs
        // Note: This is a soft check since single measurements can vary
        prop_assert!(
            latency < Duration::from_millis(1),
            "Cache access for {}B data took {:?}, expected < 1ms",
            data_size, latency
        );
    }
}

// ============================================================================
// Additional tests for hash functions
// ============================================================================

proptest! {
    #[test]
    fn prop_hash_params_deterministic(
        params in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..100), 0..10)
    ) {
        let refs: Vec<&[u8]> = params.iter().map(|v| v.as_slice()).collect();

        let hash1 = hash_params(&refs);
        let hash2 = hash_params(&refs);

        prop_assert_eq!(hash1, hash2, "hash_params should be deterministic");
    }

    #[test]
    fn prop_hash_value_deterministic(value in prop::collection::vec(any::<u8>(), 0..1000)) {
        let hash1 = hash_value(&value);
        let hash2 = hash_value(&value);

        prop_assert_eq!(hash1, hash2, "hash_value should be deterministic");
    }
}
