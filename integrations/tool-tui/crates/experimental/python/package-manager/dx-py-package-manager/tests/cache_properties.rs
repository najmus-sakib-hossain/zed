//! Property tests for content-addressable cache
//!
//! Property 7: Content-Addressable Storage Deduplication
//! For any two packages with identical file content, storing both in the cache
//! SHALL result in only one copy of the shared content on disk.

use dx_py_package_manager::GlobalCache;
use proptest::prelude::*;
use tempfile::TempDir;

/// Generate random binary data
fn arb_data() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..1000)
}

/// Generate a valid BLAKE3 hash from data
fn hash_data(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 7: Content-Addressable Storage Deduplication
    /// Validates: Requirements 6.2, 6.3
    #[test]
    fn prop_cache_deduplication(data in arb_data()) {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let hash = hash_data(&data);

        // Store the same data twice
        let path1 = cache.store(&hash, &data).unwrap();
        let path2 = cache.store(&hash, &data).unwrap();

        // Should return the same path (deduplication)
        prop_assert_eq!(path1, path2);

        // Should only have one file in cache
        let hashes = cache.list().unwrap();
        prop_assert_eq!(hashes.len(), 1);

        // Retrieved data should match original
        let retrieved = cache.get(&hash).unwrap();
        prop_assert_eq!(retrieved, data);
    }

    /// Property: Different content gets different paths
    #[test]
    fn prop_cache_different_content_different_paths(
        data1 in arb_data(),
        data2 in arb_data().prop_filter("different data", |d| !d.is_empty())
    ) {
        // Skip if data happens to be the same
        if data1 == data2 {
            return Ok(());
        }

        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let hash1 = hash_data(&data1);
        let hash2 = hash_data(&data2);

        // Skip if hashes happen to collide (extremely unlikely)
        if hash1 == hash2 {
            return Ok(());
        }

        let path1 = cache.store(&hash1, &data1).unwrap();
        let path2 = cache.store(&hash2, &data2).unwrap();

        // Different content should have different paths
        prop_assert_ne!(path1, path2);

        // Should have two files in cache
        let hashes = cache.list().unwrap();
        prop_assert_eq!(hashes.len(), 2);
    }

    /// Property: Cache contains check is accurate
    #[test]
    fn prop_cache_contains_accurate(data in arb_data()) {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let hash = hash_data(&data);

        // Should not contain before storing
        prop_assert!(!cache.contains(&hash));

        // Store
        cache.store(&hash, &data).unwrap();

        // Should contain after storing
        prop_assert!(cache.contains(&hash));

        // Remove
        cache.remove(&hash).unwrap();

        // Should not contain after removing
        prop_assert!(!cache.contains(&hash));
    }

    /// Property: Verified store rejects wrong hash
    #[test]
    fn prop_cache_verified_rejects_wrong_hash(data in arb_data()) {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let correct_hash = hash_data(&data);
        let wrong_hash = [0u8; 32]; // All zeros is almost certainly wrong

        // Skip if data happens to hash to all zeros (astronomically unlikely)
        if correct_hash == wrong_hash {
            return Ok(());
        }

        // Correct hash should work
        let result = cache.store_verified(&correct_hash, &data);
        prop_assert!(result.is_ok());

        // Wrong hash should fail
        let temp2 = TempDir::new().unwrap();
        let cache2 = GlobalCache::new(temp2.path()).unwrap();
        let result = cache2.store_verified(&wrong_hash, &data);
        prop_assert!(result.is_err());
    }

    /// Property: Ensure only fetches once
    #[test]
    fn prop_cache_ensure_fetches_once(data in arb_data()) {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let hash = hash_data(&data);
        let data_clone = data.clone();

        // Use a counter to track fetch calls
        use std::sync::atomic::{AtomicUsize, Ordering};
        let fetch_count = AtomicUsize::new(0);

        // First ensure should fetch
        let _path1 = cache.ensure(&hash, || {
            fetch_count.fetch_add(1, Ordering::SeqCst);
            Ok(data_clone.clone())
        }).unwrap();

        prop_assert_eq!(fetch_count.load(Ordering::SeqCst), 1);

        // Second ensure should not fetch
        let _path2 = cache.ensure(&hash, || {
            fetch_count.fetch_add(1, Ordering::SeqCst);
            Ok(data_clone.clone())
        }).unwrap();

        prop_assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
    }

    /// Property: Cache path follows expected structure
    #[test]
    fn prop_cache_path_structure(data in arb_data()) {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let hash = hash_data(&data);
        let path = cache.get_path(&hash);

        // Path should contain the hash hex
        let hex_hash = hex::encode(hash);
        let path_str = path.to_string_lossy();

        // Should have two-level directory structure: {hash[0:2]}/{hash[2:4]}/{hash}
        prop_assert!(path_str.contains(&hex_hash[0..2]));
        prop_assert!(path_str.contains(&hex_hash[2..4]));
        prop_assert!(path_str.ends_with(&hex_hash));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_multiple_packages_deduplication() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        // Create 10 packages, some with duplicate content
        let contents = vec![
            b"package A content".to_vec(),
            b"package B content".to_vec(),
            b"package A content".to_vec(), // Duplicate of A
            b"package C content".to_vec(),
            b"package B content".to_vec(), // Duplicate of B
        ];

        for content in &contents {
            let hash = hash_data(content);
            cache.store(&hash, content).unwrap();
        }

        // Should only have 3 unique files (A, B, C)
        let hashes = cache.list().unwrap();
        assert_eq!(hashes.len(), 3);
    }

    #[test]
    fn test_cache_total_size() {
        let temp = TempDir::new().unwrap();
        let cache = GlobalCache::new(temp.path()).unwrap();

        let data1 = vec![0u8; 1000];
        let data2 = vec![1u8; 2000];

        let hash1 = hash_data(&data1);
        let hash2 = hash_data(&data2);

        cache.store(&hash1, &data1).unwrap();
        cache.store(&hash2, &data2).unwrap();

        let total = cache.total_size().unwrap();
        assert_eq!(total, 3000);
    }
}
