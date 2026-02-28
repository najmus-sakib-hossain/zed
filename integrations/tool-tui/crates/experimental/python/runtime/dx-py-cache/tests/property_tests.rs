//! Property-based tests for reactive bytecode cache

use dx_py_cache::{entry::CompilationTier, CacheEntry, ReactiveCache};
use proptest::prelude::*;
use tempfile::NamedTempFile;

/// Property 17: Cache Invalidation Correctness
mod invalidation_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Invalidated entries are not retrievable
        #[test]
        fn prop_invalidation_removes_entry(
            path in "[a-z]{1,20}\\.py",
            source in prop::collection::vec(any::<u8>(), 1..100),
            data in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            let temp = NamedTempFile::new().unwrap();
            let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

            cache.store(&path, &source, &data, CompilationTier::Interpreter, 0).unwrap();
            prop_assert!(cache.get(&path).is_some());

            cache.invalidate(&path);
            prop_assert!(cache.get(&path).is_none());
        }

        /// Modified source invalidates cache
        #[test]
        fn prop_modified_source_invalid(
            path in "[a-z]{1,20}\\.py",
            source1 in prop::collection::vec(any::<u8>(), 1..100),
            source2 in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            let temp = NamedTempFile::new().unwrap();
            let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

            cache.store(&path, &source1, b"data", CompilationTier::Interpreter, 100).unwrap();

            // Quick validation with same mtime should pass
            prop_assert!(cache.is_valid_quick(&path, 100));

            // Quick validation with different mtime should fail
            prop_assert!(!cache.is_valid_quick(&path, 101));

            // Full validation with original source should pass
            prop_assert!(cache.validate_full(&path, &source1));

            // Full validation with different source should fail (unless sources are equal)
            if source1 != source2 {
                prop_assert!(!cache.validate_full(&path, &source2));
            }
        }

        /// Multiple entries can be stored and retrieved
        #[test]
        fn prop_multiple_entries(
            entries in prop::collection::vec(
                ("[a-z]{1,10}\\.py", prop::collection::vec(any::<u8>(), 1..50)),
                1..10
            )
        ) {
            let temp = NamedTempFile::new().unwrap();
            let cache = ReactiveCache::open(temp.path(), 10 * 1024 * 1024).unwrap();

            // Deduplicate entries by path (keep last occurrence)
            let mut unique_entries: std::collections::HashMap<&str, (usize, &Vec<u8>)> = std::collections::HashMap::new();
            for (i, (path, source)) in entries.iter().enumerate() {
                unique_entries.insert(path.as_str(), (i, source));
            }

            // Store all entries
            for (i, (path, source)) in entries.iter().enumerate() {
                let data = format!("compiled_{}", i).into_bytes();
                cache.store(path, source, &data, CompilationTier::Interpreter, i as u64).unwrap();
            }

            // Verify unique entries are retrievable with their last stored mtime
            for (path, (expected_mtime, _)) in unique_entries.iter() {
                let entry = cache.get(path);
                prop_assert!(entry.is_some(), "Entry {} not found", path);
                prop_assert_eq!(entry.unwrap().source_mtime, *expected_mtime as u64);
            }
        }
    }
}

/// Tests for cache entry serialization
mod entry_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Entry serialization is reversible
        #[test]
        fn prop_entry_roundtrip(
            source_hash in prop::collection::vec(any::<u8>(), 32..=32),
            data_offset in any::<u64>(),
            data_size in any::<u32>(),
            tier in 0u8..4,
            source_mtime in any::<u64>()
        ) {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&source_hash);

            let tier = CompilationTier::from_u8(tier).unwrap();
            let entry = CacheEntry::new(hash, data_offset, data_size, tier, source_mtime);

            let bytes = entry.to_bytes();
            let restored = CacheEntry::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.source_hash, hash);
            prop_assert_eq!(restored.data_offset, data_offset);
            prop_assert_eq!(restored.data_size, data_size);
            prop_assert_eq!(restored.tier, tier);
            prop_assert_eq!(restored.source_mtime, source_mtime);
        }

        /// Quick validation is consistent
        #[test]
        fn prop_quick_validation_consistent(
            mtime in any::<u64>(),
            check_mtime in any::<u64>()
        ) {
            let entry = CacheEntry::new([0u8; 32], 0, 0, CompilationTier::Interpreter, mtime);

            let result1 = entry.is_valid_quick(check_mtime);
            let result2 = entry.is_valid_quick(check_mtime);

            prop_assert_eq!(result1, result2);
            prop_assert_eq!(result1, mtime == check_mtime);
        }
    }
}

/// Tests for cache data storage
mod storage_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        /// Stored data is retrievable
        #[test]
        fn prop_data_roundtrip(
            path in "[a-z]{1,20}\\.py",
            source in prop::collection::vec(any::<u8>(), 1..100),
            data in prop::collection::vec(any::<u8>(), 1..1000)
        ) {
            let temp = NamedTempFile::new().unwrap();
            let cache = ReactiveCache::open(temp.path(), 10 * 1024 * 1024).unwrap();

            cache.store(&path, &source, &data, CompilationTier::BaselineJit, 0).unwrap();

            let retrieved = cache.get_data(&path).unwrap();
            prop_assert_eq!(retrieved, data);
        }

        /// Cache respects tier information
        #[test]
        fn prop_tier_preserved(
            path in "[a-z]{1,20}\\.py",
            tier in 0u8..4
        ) {
            let temp = NamedTempFile::new().unwrap();
            let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

            let tier = CompilationTier::from_u8(tier).unwrap();
            cache.store(&path, b"source", b"data", tier, 0).unwrap();

            let entry = cache.get(&path).unwrap();
            prop_assert_eq!(entry.tier, tier);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_cache_clear() {
        let temp = NamedTempFile::new().unwrap();
        let cache = ReactiveCache::open(temp.path(), 1024 * 1024).unwrap();

        cache.store("a.py", b"a", b"a", CompilationTier::Interpreter, 0).unwrap();
        cache.store("b.py", b"b", b"b", CompilationTier::Interpreter, 0).unwrap();

        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_compilation_tiers() {
        assert_eq!(CompilationTier::from_u8(0), Some(CompilationTier::Interpreter));
        assert_eq!(CompilationTier::from_u8(1), Some(CompilationTier::BaselineJit));
        assert_eq!(CompilationTier::from_u8(2), Some(CompilationTier::OptimizingJit));
        assert_eq!(CompilationTier::from_u8(3), Some(CompilationTier::AotOptimized));
        assert_eq!(CompilationTier::from_u8(4), None);
    }

    #[test]
    fn test_entry_serialized_size() {
        assert_eq!(CacheEntry::serialized_size(), 64);
    }
}
