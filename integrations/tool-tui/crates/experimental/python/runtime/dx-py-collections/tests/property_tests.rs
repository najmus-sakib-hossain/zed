//! Property-based tests for SIMD-accelerated collections

use dx_py_collections::simd_storage::PyObjectRef;
use dx_py_collections::{SimdList, SimdStorage, SwissDict};
use proptest::prelude::*;

/// Property 5: SIMD Collection Operation Correctness
mod simd_list_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Sum produces correct result
        #[test]
        fn prop_sum_correct(
            values in prop::collection::vec(-1000i64..1000, 0..100)
        ) {
            let list = SimdList::from_ints(values.clone());
            let expected: i64 = values.iter().sum();

            prop_assert_eq!(list.sum(), Some(expected as f64));
        }

        /// Float sum produces correct result
        #[test]
        fn prop_float_sum_correct(
            values in prop::collection::vec(-1000.0f64..1000.0, 0..100)
        ) {
            let list = SimdList::from_floats(values.clone());
            let expected: f64 = values.iter().sum();

            let result = list.sum().unwrap();
            prop_assert!((result - expected).abs() < 1e-10);
        }

        /// Filter produces correct indices
        #[test]
        fn prop_filter_correct(
            values in prop::collection::vec(-100i64..100, 1..50),
            threshold in -100i64..100
        ) {
            let list = SimdList::from_ints(values.clone());
            let result = list.filter_gt_int(threshold);

            let expected: Vec<usize> = values.iter()
                .enumerate()
                .filter(|(_, &v)| v > threshold)
                .map(|(i, _)| i)
                .collect();

            prop_assert_eq!(result, expected);
        }

        /// Map mul2 produces correct result
        #[test]
        fn prop_map_mul2_correct(
            values in prop::collection::vec(-1000i64..1000, 1..50)
        ) {
            let list = SimdList::from_ints(values.clone());
            let result = list.map_mul2_int().unwrap();

            let expected: Vec<i64> = values.iter().map(|&v| v * 2).collect();

            prop_assert_eq!(result.storage().as_ints(), Some(&expected[..]));
        }

        /// Index finds correct position
        #[test]
        fn prop_index_correct(
            values in prop::collection::vec(0i64..100, 1..50),
            target in 0i64..100
        ) {
            let list = SimdList::from_ints(values.clone());
            let result = list.index_int(target);
            let expected = values.iter().position(|&v| v == target);

            prop_assert_eq!(result, expected);
        }

        /// Count produces correct result
        #[test]
        fn prop_count_correct(
            values in prop::collection::vec(0i64..10, 1..50),
            target in 0i64..10
        ) {
            let list = SimdList::from_ints(values.clone());
            let result = list.count_int(target);
            let expected = values.iter().filter(|&&v| v == target).count();

            prop_assert_eq!(result, expected);
        }
    }
}

/// Tests for SimdStorage
mod storage_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Storage correctly identifies type
        #[test]
        fn prop_storage_type_detection(
            ints in prop::collection::vec(any::<i64>(), 1..20),
            floats in prop::collection::vec(any::<f64>(), 1..20)
        ) {
            let int_storage = SimdStorage::from_ints(ints);
            prop_assert!(int_storage.is_ints());
            prop_assert!(!int_storage.is_floats());

            let float_storage = SimdStorage::from_floats(floats);
            prop_assert!(float_storage.is_floats());
            prop_assert!(!float_storage.is_ints());
        }

        /// Storage length is correct
        #[test]
        fn prop_storage_len(
            values in prop::collection::vec(any::<i64>(), 0..100)
        ) {
            let storage = SimdStorage::from_ints(values.clone());
            if values.is_empty() {
                prop_assert!(storage.is_empty());
            } else {
                prop_assert_eq!(storage.len(), values.len());
            }
        }
    }
}

/// Tests for SwissDict
mod swiss_dict_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Insert and get are consistent
        #[test]
        fn prop_insert_get_consistent(
            entries in prop::collection::vec(
                (any::<i32>(), any::<i32>()),
                1..50
            )
        ) {
            let mut dict = SwissDict::new();

            for (k, v) in &entries {
                dict.insert(*k, *v);
            }

            // Last value for each key should be retrievable
            let mut expected = std::collections::HashMap::new();
            for (k, v) in &entries {
                expected.insert(*k, *v);
            }

            for (k, v) in &expected {
                prop_assert_eq!(dict.get(k), Some(v));
            }
        }

        /// Remove actually removes
        #[test]
        fn prop_remove_works(
            entries in prop::collection::vec(
                (any::<i32>(), any::<i32>()),
                1..30
            ),
            remove_idx in any::<usize>()
        ) {
            let mut dict = SwissDict::new();

            for (k, v) in &entries {
                dict.insert(*k, *v);
            }

            if !entries.is_empty() {
                let idx = remove_idx % entries.len();
                let (key, _) = entries[idx];

                dict.remove(&key);
                prop_assert!(!dict.contains_key(&key));
            }
        }

        /// Length is accurate
        #[test]
        fn prop_len_accurate(
            entries in prop::collection::vec(
                (0i32..100, any::<i32>()),
                0..50
            )
        ) {
            let mut dict = SwissDict::new();

            for (k, v) in &entries {
                dict.insert(*k, *v);
            }

            // Count unique keys
            let unique_keys: std::collections::HashSet<_> = entries.iter()
                .map(|(k, _)| k)
                .collect();

            prop_assert_eq!(dict.len(), unique_keys.len());
        }

        /// Clear empties the dict
        #[test]
        fn prop_clear_empties(
            entries in prop::collection::vec(
                (any::<i32>(), any::<i32>()),
                1..20
            )
        ) {
            let mut dict = SwissDict::new();

            for (k, v) in &entries {
                dict.insert(*k, *v);
            }

            dict.clear();
            prop_assert!(dict.is_empty());
            prop_assert_eq!(dict.len(), 0);
        }
    }
}

/// Tests for homogeneous type detection
mod type_detection_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Homogeneous int list is detected
        #[test]
        fn prop_homogeneous_int_detected(
            values in prop::collection::vec(any::<i64>(), 1..20)
        ) {
            let items: Vec<PyObjectRef> = values.iter()
                .map(|&v| PyObjectRef::from_int(v))
                .collect();

            let list = SimdList::from_py_list(items);
            prop_assert!(list.is_int_list());
        }

        /// Homogeneous float list is detected
        #[test]
        fn prop_homogeneous_float_detected(
            values in prop::collection::vec(any::<f64>(), 1..20)
        ) {
            let items: Vec<PyObjectRef> = values.iter()
                .map(|&v| PyObjectRef::from_float(v))
                .collect();

            let list = SimdList::from_py_list(items);
            prop_assert!(list.is_float_list());
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_empty_list() {
        let list = SimdList::new();
        assert!(list.is_empty());
        assert_eq!(list.sum(), Some(0.0));
    }

    #[test]
    fn test_empty_dict() {
        let dict: SwissDict<i32, i32> = SwissDict::new();
        assert!(dict.is_empty());
        assert_eq!(dict.get(&0), None);
    }

    #[test]
    fn test_large_dict() {
        let mut dict = SwissDict::new();

        for i in 0..1000 {
            dict.insert(i, i * 2);
        }

        assert_eq!(dict.len(), 1000);

        for i in 0..1000 {
            assert_eq!(dict.get(&i), Some(&(i * 2)));
        }
    }
}
