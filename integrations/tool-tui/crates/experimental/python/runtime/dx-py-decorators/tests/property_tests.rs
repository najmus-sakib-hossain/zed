//! Property-based tests for compiler-inlined decorators

use dx_py_decorators::{
    dataclass::DataclassField, inlineable::FunctionFlags, lru_cache::CacheKey, DataclassInfo,
    DecoratorInliner, InlineLruCache, InlineableDecorator,
};
use proptest::prelude::*;

/// Property 8: Decorator Inlining Compatibility
mod inlining_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Inlined decorators produce correct flags
        #[test]
        fn prop_flags_correct(
            decorator_type in 0u8..7
        ) {
            let decorator = match decorator_type {
                0 => InlineableDecorator::StaticMethod,
                1 => InlineableDecorator::ClassMethod,
                2 => InlineableDecorator::Property,
                3 => InlineableDecorator::LruCache { maxsize: Some(128) },
                4 => InlineableDecorator::Jit,
                5 => InlineableDecorator::Parallel,
                _ => InlineableDecorator::Custom("test".to_string()),
            };

            let inliner = DecoratorInliner::new();
            let result = inliner.inline(&decorator, &[], None);

            let expected_flags = decorator.get_flags();
            prop_assert_eq!(result.flags, expected_flags);
        }

        /// Inlineable decorators are correctly identified
        #[test]
        fn prop_inlineable_detection(
            name in "[a-z_]+",
            is_builtin in any::<bool>()
        ) {
            let builtin_names = ["staticmethod", "classmethod", "property", "jit", "parallel"];

            let test_name = if is_builtin {
                builtin_names[name.len() % builtin_names.len()]
            } else {
                &name
            };

            let decorator = InlineableDecorator::parse(test_name, &[]).unwrap();

            if builtin_names.contains(&test_name) {
                prop_assert!(decorator.is_inlineable());
            }
        }
    }
}

/// Tests for LRU cache
mod lru_cache_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Cache respects maxsize
        #[test]
        fn prop_cache_maxsize(
            maxsize in 1usize..100,
            num_entries in 1usize..200
        ) {
            let cache = InlineLruCache::new(maxsize);

            for i in 0..num_entries {
                let key = CacheKey::from_args(&[i as u64]);
                cache.put(key, i as u64);
            }

            prop_assert!(cache.len() <= maxsize);
        }

        /// Cache hit rate is reasonable for repeated access
        #[test]
        fn prop_cache_hit_rate(
            maxsize in 10usize..50,
            num_unique in 1usize..20,
            num_accesses in 10usize..100
        ) {
            let cache = InlineLruCache::new(maxsize);

            // Populate cache
            for i in 0..num_unique.min(maxsize) {
                let key = CacheKey::from_args(&[i as u64]);
                cache.put(key, i as u64);
            }

            // Access repeatedly
            for i in 0..num_accesses {
                let idx = i % num_unique.min(maxsize);
                let key = CacheKey::from_args(&[idx as u64]);
                cache.get(&key);
            }

            let stats = cache.stats();

            // If we're accessing within cache size, hit rate should be high
            if num_unique <= maxsize {
                prop_assert!(stats.hit_rate() >= 0.5);
            }
        }

        /// Cache preserves values correctly
        #[test]
        fn prop_cache_value_preservation(
            entries in prop::collection::vec((any::<u64>(), any::<u64>()), 1..50)
        ) {
            let cache = InlineLruCache::new(100);

            // Store all entries
            for (k, v) in &entries {
                let key = CacheKey::from_args(&[*k]);
                cache.put(key, *v);
            }

            // Verify last value for each key
            let mut expected = std::collections::HashMap::new();
            for (k, v) in &entries {
                expected.insert(*k, *v);
            }

            for (k, v) in &expected {
                let key = CacheKey::from_args(&[*k]);
                prop_assert_eq!(cache.get(&key), Some(*v));
            }
        }
    }
}

/// Tests for dataclass generation
mod dataclass_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Generated __init__ bytecode is non-empty
        #[test]
        fn prop_init_generated(
            num_fields in 1usize..10
        ) {
            let mut info = DataclassInfo::new("TestClass");

            for i in 0..num_fields {
                info.add_field(DataclassField::new(&format!("field{}", i)));
            }

            let bytecode = info.generate_init();
            prop_assert!(!bytecode.is_empty());

            // Should end with RETURN_VALUE (0x56)
            prop_assert_eq!(*bytecode.last().unwrap(), 0x56);
        }

        /// Slots match fields
        #[test]
        fn prop_slots_match_fields(
            field_names in prop::collection::vec("[a-z][a-z0-9_]{0,10}", 1..10)
        ) {
            let mut info = DataclassInfo::new("TestClass");

            for name in &field_names {
                info.add_field(DataclassField::new(name));
            }

            let slots = info.get_slots();
            prop_assert_eq!(slots.len(), field_names.len());

            for (slot, name) in slots.iter().zip(field_names.iter()) {
                prop_assert_eq!(slot, name);
            }
        }

        /// All comparison methods are generated
        #[test]
        fn prop_comparison_methods_generated(
            num_fields in 1usize..5
        ) {
            let mut info = DataclassInfo::new("TestClass");

            for i in 0..num_fields {
                info.add_field(DataclassField::new(&format!("field{}", i)));
            }

            for method in ["__lt__", "__le__", "__gt__", "__ge__"] {
                let bytecode = info.generate_comparison(method);
                prop_assert!(!bytecode.is_empty());
                prop_assert_eq!(*bytecode.last().unwrap(), 0x56);
            }
        }
    }
}

/// Tests for decorator parsing
mod parsing_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// lru_cache maxsize is parsed correctly
        #[test]
        fn prop_lru_cache_maxsize_parsed(
            maxsize in 1u32..10000
        ) {
            let args = [("maxsize", maxsize.to_string())];
            let args_ref: Vec<(&str, &str)> = args.iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect();

            let decorator = InlineableDecorator::parse("lru_cache", &args_ref).unwrap();

            match decorator {
                InlineableDecorator::LruCache { maxsize: Some(m) } => {
                    prop_assert_eq!(m, maxsize as usize);
                }
                _ => prop_assert!(false, "Expected LruCache"),
            }
        }

        /// dataclass options are parsed correctly
        #[test]
        fn prop_dataclass_options_parsed(
            frozen in any::<bool>(),
            slots in any::<bool>()
        ) {
            let frozen_str = if frozen { "True" } else { "False" };
            let slots_str = if slots { "True" } else { "False" };

            let args = [("frozen", frozen_str), ("slots", slots_str)];
            let decorator = InlineableDecorator::parse("dataclass", &args).unwrap();

            match decorator {
                InlineableDecorator::Dataclass { frozen: f, slots: s, .. } => {
                    prop_assert_eq!(f, frozen);
                    prop_assert_eq!(s, slots);
                }
                _ => prop_assert!(false, "Expected Dataclass"),
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parse_all_builtins() {
        let builtins = [
            ("staticmethod", InlineableDecorator::StaticMethod),
            ("classmethod", InlineableDecorator::ClassMethod),
            ("property", InlineableDecorator::Property),
            ("jit", InlineableDecorator::Jit),
            ("parallel", InlineableDecorator::Parallel),
        ];

        for (name, expected) in builtins {
            let parsed = InlineableDecorator::parse(name, &[]).unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn test_cache_key_equality() {
        let key1 = CacheKey::from_args(&[1, 2, 3]);
        let key2 = CacheKey::from_args(&[1, 2, 3]);
        let key3 = CacheKey::from_args(&[1, 2, 4]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_inliner_custom_decorator() {
        let mut inliner = DecoratorInliner::new();

        inliner.register_custom("trace", |_| dx_py_decorators::inliner::InlineResult {
            bytecode: Some(vec![0xFF]),
            flags: FunctionFlags::HAS_TYPES,
            generated_methods: Vec::new(),
            cache: None,
        });

        let result = inliner.inline(&InlineableDecorator::Custom("trace".to_string()), &[], None);

        assert_eq!(result.bytecode, Some(vec![0xFF]));
    }
}
