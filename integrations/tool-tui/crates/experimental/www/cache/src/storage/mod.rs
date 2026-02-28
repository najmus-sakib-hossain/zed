//! # Storage Module
//!
//! Multi-layer storage strategy for eternal caching

pub mod cache_api;
pub mod indexeddb;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub indexeddb_size: u64,
    pub cache_api_size: u64,
    pub total_entries: u32,
    pub hit_rate: f64,
}

/// Initialize IndexedDB
pub async fn init_indexeddb(db_name: &str, version: u32) -> Result<(), JsValue> {
    indexeddb::open_database(db_name, version).await?;
    Ok(())
}

/// Initialize Cache API
pub async fn init_cache_api() -> Result<(), JsValue> {
    cache_api::open_cache("dx-cache-v1").await?;
    Ok(())
}

/// Get storage statistics
pub async fn get_storage_stats() -> Result<StorageStats, JsValue> {
    // Get IndexedDB stats
    let db = indexeddb::open_database("dx-cache", 1).await?;
    let indexeddb_size = indexeddb::get_database_size(&db).await?;
    let indexeddb_entries = indexeddb::get_database_entry_count(&db).await?;

    // Get Cache API stats
    let cache_api_size = cache_api::get_all_caches_size().await?;
    let cache_api_entries = cache_api::get_all_caches_entry_count().await?;

    // Calculate total entries
    let total_entries = indexeddb_entries + cache_api_entries;

    // Hit rate would need to be tracked separately with counters
    // For now, return 0.0 as we don't have hit/miss tracking yet
    let hit_rate = 0.0;

    Ok(StorageStats {
        indexeddb_size,
        cache_api_size,
        total_entries,
        hit_rate,
    })
}

/// Clear IndexedDB
pub async fn clear_indexeddb() -> Result<(), JsValue> {
    indexeddb::delete_database("dx-cache").await?;
    Ok(())
}

/// Clear Cache API
pub async fn clear_cache_api() -> Result<(), JsValue> {
    cache_api::delete_cache("dx-cache-v1").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_stats_struct() {
        let stats = StorageStats {
            indexeddb_size: 1024,
            cache_api_size: 2048,
            total_entries: 10,
            hit_rate: 0.95,
        };

        assert_eq!(stats.indexeddb_size, 1024);
        assert_eq!(stats.cache_api_size, 2048);
        assert_eq!(stats.total_entries, 10);
        assert!((stats.hit_rate - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_storage_stats_serialization() {
        let stats = StorageStats {
            indexeddb_size: 1024,
            cache_api_size: 2048,
            total_entries: 10,
            hit_rate: 0.95,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: StorageStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.indexeddb_size, stats.indexeddb_size);
        assert_eq!(deserialized.cache_api_size, stats.cache_api_size);
        assert_eq!(deserialized.total_entries, stats.total_entries);
        assert!((deserialized.hit_rate - stats.hit_rate).abs() < f64::EPSILON);
    }

    #[test]
    fn test_storage_stats_total_size() {
        let stats = StorageStats {
            indexeddb_size: 1024,
            cache_api_size: 2048,
            total_entries: 10,
            hit_rate: 0.95,
        };

        // Total size should be sum of IndexedDB and Cache API sizes
        let total_size = stats.indexeddb_size + stats.cache_api_size;
        assert_eq!(total_size, 3072);
    }
}
