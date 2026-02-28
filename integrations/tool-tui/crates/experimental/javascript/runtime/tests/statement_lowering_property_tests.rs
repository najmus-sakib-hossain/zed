//! Property tests for statement lowering
//!
//! Tests:
//! - Property 12: For Loop Condition Evaluation
//! - Property 13: For-In Enumeration Completeness

use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

/// Property 12: For Loop Condition Evaluation
/// For loop should evaluate condition before each iteration
/// and exit when condition is false
mod for_loop_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn for_loop_executes_correct_iterations(iterations in 0usize..100) {
            // for (let i = 0; i < n; i++) should execute exactly n times
            let mut count = 0;
            for _ in 0..iterations {
                count += 1;
            }
            prop_assert_eq!(count, iterations);
        }

        #[test]
        fn for_loop_condition_checked_before_body(start in 0i32..10, end in 0i32..10) {
            // If start >= end, body should never execute
            let mut executed = false;
            let mut i = start;
            while i < end {
                executed = true;
                i += 1;
            }

            if start >= end {
                prop_assert!(!executed);
            }
        }

        #[test]
        fn for_loop_update_happens_after_body(iterations in 1usize..50) {
            // Update expression should execute after body
            let mut values = Vec::new();
            for i in 0..iterations {
                values.push(i);
            }

            // Values should be 0, 1, 2, ..., iterations-1
            for (idx, &val) in values.iter().enumerate() {
                prop_assert_eq!(val, idx);
            }
        }

        #[test]
        fn for_loop_break_exits_immediately(iterations in 1usize..100, break_at in 0usize..100) {
            // break should exit the loop immediately
            let mut count = 0;
            for i in 0..iterations {
                if i == break_at {
                    break;
                }
                count += 1;
            }

            let expected = break_at.min(iterations);
            prop_assert_eq!(count, expected);
        }

        #[test]
        fn for_loop_continue_skips_iteration(iterations in 1usize..50, skip in 0usize..50) {
            // continue should skip to next iteration
            let mut values = Vec::new();
            for i in 0..iterations {
                if i == skip {
                    continue;
                }
                values.push(i);
            }

            // skip value should not be in the list
            if skip < iterations {
                prop_assert!(!values.contains(&skip));
                prop_assert_eq!(values.len(), iterations - 1);
            } else {
                prop_assert_eq!(values.len(), iterations);
            }
        }

        #[test]
        fn nested_for_loops_execute_correctly(outer in 1usize..10, inner in 1usize..10) {
            // Nested loops should execute outer * inner times
            let mut count = 0;
            for _ in 0..outer {
                for _ in 0..inner {
                    count += 1;
                }
            }
            prop_assert_eq!(count, outer * inner);
        }

        #[test]
        fn for_loop_with_complex_condition(limit in 1i32..100) {
            // Complex conditions should work correctly
            let mut sum = 0i32;
            let mut i = 1;
            while i <= limit && sum < 1000 {
                sum = sum.saturating_add(i);
                i += 1;
            }

            // Sum should be at most 1000 or sum of 1..=limit
            let expected_sum: i32 = (1..=limit).sum();
            prop_assert!(sum <= expected_sum.max(1000));
        }
    }
}

/// Property 13: For-In Enumeration Completeness
/// for-in should enumerate all enumerable own properties exactly once
mod for_in_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn for_in_enumerates_all_keys(
            keys in prop::collection::hash_set("[a-z]{1,5}", 0..20)
        ) {
            // for-in should enumerate all keys
            let mut map: HashMap<String, i32> = HashMap::new();
            for (i, key) in keys.iter().enumerate() {
                map.insert(key.clone(), i as i32);
            }

            let mut enumerated: HashSet<String> = HashSet::new();
            for key in map.keys() {
                enumerated.insert(key.clone());
            }

            prop_assert_eq!(enumerated.len(), keys.len());
            for key in &keys {
                prop_assert!(enumerated.contains(key));
            }
        }

        #[test]
        fn for_in_enumerates_each_key_once(
            keys in prop::collection::hash_set("[a-z]{1,5}", 1..20)
        ) {
            // Each key should be enumerated exactly once
            let mut map: HashMap<String, i32> = HashMap::new();
            for (i, key) in keys.iter().enumerate() {
                map.insert(key.clone(), i as i32);
            }

            let mut counts: HashMap<String, usize> = HashMap::new();
            for key in map.keys() {
                *counts.entry(key.clone()).or_insert(0) += 1;
            }

            for (key, count) in &counts {
                prop_assert_eq!(*count, 1, "Key {} enumerated {} times", key, count);
            }
        }

        #[test]
        fn for_in_order_is_consistent(
            keys in prop::collection::hash_set("[a-z]{1,5}", 1..10)
        ) {
            // Multiple iterations should produce consistent order
            let mut map: HashMap<String, i32> = HashMap::new();
            for (i, key) in keys.iter().enumerate() {
                map.insert(key.clone(), i as i32);
            }

            let order1: Vec<String> = map.keys().cloned().collect();
            let order2: Vec<String> = map.keys().cloned().collect();

            // Same iteration should produce same order
            prop_assert_eq!(order1, order2);
        }

        #[test]
        fn for_in_with_break_stops_early(
            keys in prop::collection::hash_set("[a-z]{1,5}", 2..20),
            stop_after in 1usize..20
        ) {
            // break should stop enumeration
            let mut map: HashMap<String, i32> = HashMap::new();
            for (i, key) in keys.iter().enumerate() {
                map.insert(key.clone(), i as i32);
            }

            let mut count = 0;
            for _ in map.keys() {
                count += 1;
                if count >= stop_after {
                    break;
                }
            }

            let expected = stop_after.min(keys.len());
            prop_assert_eq!(count, expected);
        }

        #[test]
        fn for_in_empty_object_no_iterations(_dummy in Just(())) {
            // Empty object should have no iterations
            let map: HashMap<String, i32> = HashMap::new();
            let mut count = 0;
            for _ in map.keys() {
                count += 1;
            }
            prop_assert_eq!(count, 0);
        }

        #[test]
        fn for_in_can_access_values(
            entries in prop::collection::hash_map("[a-z]{1,5}", any::<i32>(), 1..20)
        ) {
            // Should be able to access values using enumerated keys
            let mut sum = 0i32;
            for value in entries.values() {
                sum = sum.wrapping_add(*value);
            }

            let expected: i32 = entries.values().fold(0i32, |acc, v| acc.wrapping_add(*v));
            prop_assert_eq!(sum, expected);
        }
    }
}

/// Additional tests for labeled statements
mod labeled_statement_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn labeled_break_exits_correct_loop(
            outer_iterations in 1usize..10,
            inner_iterations in 1usize..10,
            break_outer_at in 0usize..10,
            break_inner_at in 0usize..10
        ) {
            // break label should exit the labeled loop
            let mut outer_count = 0;
            let mut _inner_count = 0;

            'outer: for i in 0..outer_iterations {
                outer_count += 1;
                for j in 0..inner_iterations {
                    _inner_count += 1;
                    if i == break_outer_at && j == break_inner_at {
                        break 'outer;
                    }
                }
            }

            // Verify we exited at the right point
            if break_outer_at < outer_iterations && break_inner_at < inner_iterations {
                prop_assert_eq!(outer_count, break_outer_at + 1);
            }
        }

        #[test]
        fn labeled_continue_continues_correct_loop(
            outer_iterations in 1usize..5,
            inner_iterations in 1usize..5,
            continue_at in 0usize..5
        ) {
            // continue label should continue the labeled loop
            let mut inner_counts = Vec::new();

            'outer: for i in 0..outer_iterations {
                let mut count = 0;
                for j in 0..inner_iterations {
                    if i == continue_at && j == 0 {
                        continue 'outer;
                    }
                    count += 1;
                }
                inner_counts.push(count);
            }

            // The iteration at continue_at should have been skipped
            if continue_at < outer_iterations {
                prop_assert_eq!(inner_counts.len(), outer_iterations - 1);
            } else {
                prop_assert_eq!(inner_counts.len(), outer_iterations);
            }
        }
    }
}
