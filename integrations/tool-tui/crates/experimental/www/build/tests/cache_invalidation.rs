//! Property tests for build artifact caching
//!
//! Feature: dx-www-production-ready, Property 1: Cache invalidation correctness
//!
//! This property test verifies that the build cache correctly invalidates entries when source
//! files change. The property tests that:
//! 1. For any valid source file, caching it then retrieving it returns the same artifact
//! 2. If the source file changes (different content hash), the cache should not return the old entry
//! 3. Cache entries with invalid output files should not be returned

use build::{BuildCache, CacheEntry, CacheKey, content_hash};
use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Strategy for generating arbitrary file content
fn arbitrary_content() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1024)
}

/// Strategy for generating arbitrary processor names
fn arbitrary_processor() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]{3,10}").unwrap()
}

/// Strategy for generating arbitrary file paths
fn arbitrary_path() -> impl Strategy<Value = PathBuf> {
    prop::string::string_regex("[a-z0-9_]{1,20}\\.(css|js|png|woff2)")
        .unwrap()
        .prop_map(PathBuf::from)
}

proptest! {
    /// Property 1a: Cache round-trip consistency
    ///
    /// For any valid source file, caching it then retrieving it returns the same artifact.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn cache_round_trip_consistency(
        content in arbitrary_content(),
        processor in arbitrary_processor(),
        source_name in arbitrary_path(),
        output_name in arbitrary_path(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Create source file
        let source_path = temp_dir.path().join(&source_name);
        fs::write(&source_path, &content).unwrap();

        // Create output file
        let output_path = temp_dir.path().join(&output_name);
        let output_content = b"processed output";
        fs::write(&output_path, output_content).unwrap();

        // Create cache key and entry
        let key = CacheKey::from_file(&source_path, processor).unwrap();
        let output_hash = content_hash(output_content);
        let entry = CacheEntry::new(
            key.clone(),
            output_path.clone(),
            output_hash,
            output_content.len(),
        );

        // Insert into cache
        cache.insert(entry.clone()).unwrap();

        // Retrieve from cache
        let retrieved = cache.get(&key);

        // Verify we got the same entry back
        prop_assert!(retrieved.is_some(), "Cache entry should be retrievable");
        let retrieved = retrieved.unwrap();
        prop_assert_eq!(&retrieved.key, &key, "Cache key should match");
        prop_assert_eq!(&retrieved.output_path, &output_path, "Output path should match");
        prop_assert_eq!(retrieved.size, output_content.len(), "Size should match");
    }

    /// Property 1b: Cache invalidation on content change
    ///
    /// If the source file changes (different content hash), the cache should not return
    /// the old entry.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn cache_invalidation_on_content_change(
        initial_content in arbitrary_content(),
        changed_content in arbitrary_content(),
        processor in arbitrary_processor(),
        source_name in arbitrary_path(),
        output_name in arbitrary_path(),
    ) {
        // Skip if contents are the same (would have same hash)
        prop_assume!(initial_content != changed_content);

        let temp_dir = TempDir::new().unwrap();
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Create source file with initial content
        let source_path = temp_dir.path().join(&source_name);
        fs::write(&source_path, &initial_content).unwrap();

        // Create output file
        let output_path = temp_dir.path().join(&output_name);
        let output_content = b"processed output";
        fs::write(&output_path, output_content).unwrap();

        // Create cache key and entry for initial content
        let initial_key = CacheKey::from_file(&source_path, processor.clone()).unwrap();
        let output_hash = content_hash(output_content);
        let entry = CacheEntry::new(
            initial_key.clone(),
            output_path.clone(),
            output_hash,
            output_content.len(),
        );

        // Insert into cache
        cache.insert(entry).unwrap();

        // Verify initial entry is cached
        prop_assert!(cache.get(&initial_key).is_some(), "Initial entry should be cached");

        // Change the source file content
        fs::write(&source_path, &changed_content).unwrap();

        // Create new cache key with changed content
        let changed_key = CacheKey::from_file(&source_path, processor).unwrap();

        // Verify the cache does NOT return the old entry for the new key
        prop_assert!(
            cache.get(&changed_key).is_none(),
            "Cache should not return old entry for changed content"
        );

        // Verify the keys are different
        prop_assert_ne!(
            initial_key.content_hash,
            changed_key.content_hash,
            "Content hashes should differ for different content"
        );
    }

    /// Property 1c: Cache invalidation for missing output files
    ///
    /// Cache entries with invalid output files (missing or modified) should not be returned.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn cache_invalidation_for_missing_output(
        content in arbitrary_content(),
        processor in arbitrary_processor(),
        source_name in arbitrary_path(),
        output_name in arbitrary_path(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Create source file
        let source_path = temp_dir.path().join(&source_name);
        fs::write(&source_path, &content).unwrap();

        // Create output file
        let output_path = temp_dir.path().join(&output_name);
        let output_content = b"processed output";
        fs::write(&output_path, output_content).unwrap();

        // Create cache key and entry
        let key = CacheKey::from_file(&source_path, processor).unwrap();
        let output_hash = content_hash(output_content);
        let entry = CacheEntry::new(
            key.clone(),
            output_path.clone(),
            output_hash,
            output_content.len(),
        );

        // Insert into cache
        cache.insert(entry).unwrap();

        // Verify entry is cached
        prop_assert!(cache.get(&key).is_some(), "Entry should be cached initially");

        // Delete the output file
        fs::remove_file(&output_path).unwrap();

        // Verify cache does NOT return the entry (because output is missing)
        prop_assert!(
            cache.get(&key).is_none(),
            "Cache should not return entry when output file is missing"
        );
    }

    /// Property 1d: Cache invalidation for modified output files
    ///
    /// Cache entries with modified output files (different hash) should not be returned.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn cache_invalidation_for_modified_output(
        content in arbitrary_content(),
        processor in arbitrary_processor(),
        source_name in arbitrary_path(),
        output_name in arbitrary_path(),
        modified_output in arbitrary_content(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Create source file
        let source_path = temp_dir.path().join(&source_name);
        fs::write(&source_path, &content).unwrap();

        // Create output file
        let output_path = temp_dir.path().join(&output_name);
        let output_content = b"processed output";
        fs::write(&output_path, output_content).unwrap();

        // Create cache key and entry
        let key = CacheKey::from_file(&source_path, processor).unwrap();
        let output_hash = content_hash(output_content);
        let entry = CacheEntry::new(
            key.clone(),
            output_path.clone(),
            output_hash,
            output_content.len(),
        );

        // Insert into cache
        cache.insert(entry).unwrap();

        // Verify entry is cached
        prop_assert!(cache.get(&key).is_some(), "Entry should be cached initially");

        // Modify the output file (only if different from original)
        if modified_output != output_content {
            fs::write(&output_path, &modified_output).unwrap();

            // Verify cache does NOT return the entry (because output hash changed)
            prop_assert!(
                cache.get(&key).is_none(),
                "Cache should not return entry when output file is modified"
            );
        }
    }
}
