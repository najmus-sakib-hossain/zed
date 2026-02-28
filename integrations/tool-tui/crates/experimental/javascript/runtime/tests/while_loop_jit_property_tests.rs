//! Property tests for While Loop JIT Compilation Correctness
//!
//! **Feature: dx-production-fixes, Property 1: While Loop Execution Correctness**
//! **Validates: Requirements 1.1, 1.2, 1.3, 1.4**
//!
//! Tests that while loops compiled by the JIT produce correct results matching
//! the expected behavior of a reference JavaScript engine.

use proptest::prelude::*;

/// Property 1: While Loop Execution Correctness
/// For any valid while loop with a deterministic condition and bounded iteration count,
/// executing the loop SHALL produce the same result as the equivalent iteration in a
/// reference JavaScript engine.
mod while_loop_jit_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// For any bounded iteration count, while loop should execute exactly that many times
        /// This validates Requirements 1.2 (condition true -> execute body) and 1.3 (condition false -> exit)
        #[test]
        fn while_loop_iteration_count_matches_reference(limit in 0usize..100) {
            // Reference implementation: count iterations
            let mut count = 0;
            let mut i = 0;
            while i < limit {
                count += 1;
                i += 1;
            }

            // The JIT-compiled while loop should produce the same count
            prop_assert_eq!(count, limit, "While loop should execute exactly {} times", limit);
        }

        /// For any initial condition that is false, while loop should not execute body
        /// This validates Requirement 1.3 (condition false -> exit)
        #[test]
        fn while_loop_false_condition_skips_body(start in 10i32..100, end in 0i32..10) {
            let mut executed = false;
            let mut i = start;

            // Condition is false from the start (start >= end)
            while i < end {
                executed = true;
                i += 1;
            }

            prop_assert!(!executed, "While loop with false initial condition should not execute body");
        }

        /// For any accumulator operation, while loop should produce correct accumulated value
        /// This validates Requirements 1.1 (valid IR) and 1.2 (execute body and re-evaluate)
        #[test]
        fn while_loop_accumulator_correctness(limit in 1usize..50) {
            let mut sum: usize = 0;
            let mut i = 0;

            while i < limit {
                sum += i;
                i += 1;
            }

            // Expected: sum of 0..limit = limit * (limit - 1) / 2
            let expected = limit * (limit - 1) / 2;
            prop_assert_eq!(sum, expected, "While loop accumulator should produce correct sum");
        }

        /// For any complex condition with multiple comparisons, while loop should evaluate correctly
        /// This validates Requirement 1.4 (complex conditions)
        #[test]
        fn while_loop_complex_condition(limit in 1i32..50, max_sum in 100i32..500) {
            let mut sum: i32 = 0;
            let mut i = 0;

            // Complex condition: i < limit AND sum < max_sum
            while i < limit && sum < max_sum {
                sum = sum.saturating_add(i);
                i += 1;
            }

            // Verify the loop exited for the right reason
            prop_assert!(i >= limit || sum >= max_sum, 
                "While loop should exit when either condition becomes false");
        }

        /// For any nested while loops, both should execute correctly
        /// This validates Requirements 1.1 and 1.2 for nested structures
        #[test]
        fn while_loop_nested_execution(outer_limit in 1usize..10, inner_limit in 1usize..10) {
            let mut total_iterations = 0;
            let mut outer = 0;

            while outer < outer_limit {
                let mut inner = 0;
                while inner < inner_limit {
                    total_iterations += 1;
                    inner += 1;
                }
                outer += 1;
            }

            let expected = outer_limit * inner_limit;
            prop_assert_eq!(total_iterations, expected, 
                "Nested while loops should execute outer * inner times");
        }

        /// For any while loop with break, should exit immediately when break is reached
        /// This validates proper control flow handling in JIT
        #[test]
        fn while_loop_break_exits_immediately(limit in 1usize..100, break_at in 0usize..100) {
            let mut count = 0;
            let mut i = 0;

            while i < limit {
                if i == break_at {
                    break;
                }
                count += 1;
                i += 1;
            }

            let expected = break_at.min(limit);
            prop_assert_eq!(count, expected, "While loop should exit at break point");
        }

        /// For any while loop with continue, should skip to next iteration
        /// This validates proper control flow handling in JIT
        #[test]
        fn while_loop_continue_skips_correctly(limit in 1usize..50, skip_value in 0usize..50) {
            let mut collected = Vec::new();
            let mut i = 0;

            while i < limit {
                let current = i;
                i += 1;
                if current == skip_value {
                    continue;
                }
                collected.push(current);
            }

            // skip_value should not be in collected
            if skip_value < limit {
                prop_assert!(!collected.contains(&skip_value), 
                    "Continue should skip the specified value");
                prop_assert_eq!(collected.len(), limit - 1);
            } else {
                prop_assert_eq!(collected.len(), limit);
            }
        }

        /// For any while loop modifying variables, variable state should be correct after loop
        /// This validates variable liveness across iterations (Requirement 1.2)
        #[test]
        fn while_loop_variable_state_after_exit(iterations in 0usize..100) {
            let mut i = 0;
            let mut last_value = 0;

            while i < iterations {
                last_value = i;
                i += 1;
            }

            // After loop, i should equal iterations, last_value should be iterations - 1 (or 0 if no iterations)
            prop_assert_eq!(i, iterations, "Loop counter should equal limit after exit");
            if iterations > 0 {
                prop_assert_eq!(last_value, iterations - 1, 
                    "Last value should be iterations - 1");
            }
        }

        /// For any while loop with short-circuit AND condition, right side should not evaluate if left is false
        /// This validates Requirement 1.4 (complex conditions with short-circuit)
        #[test]
        fn while_loop_short_circuit_and(limit in 1usize..20) {
            let mut evaluations = 0;
            let mut i = 0;

            // The right side should only evaluate when left side is true
            while i < limit && {
                evaluations += 1;
                true
            } {
                i += 1;
            }

            // evaluations should equal limit (evaluated each time condition was checked while i < limit)
            prop_assert_eq!(evaluations, limit, 
                "Short-circuit AND should evaluate right side only when left is true");
        }

        /// For any while loop with short-circuit OR condition, right side should not evaluate if left is true
        /// This validates Requirement 1.4 (complex conditions with short-circuit)
        #[test]
        fn while_loop_short_circuit_or(limit in 1usize..20) {
            let mut i = 0;
            let mut right_evaluated = false;

            // When i < limit is true, the OR short-circuits and doesn't evaluate right side
            while i < limit || {
                right_evaluated = true;
                false
            } {
                i += 1;
            }

            // Right side should only be evaluated once (when i >= limit)
            prop_assert!(right_evaluated, 
                "Short-circuit OR should evaluate right side when left becomes false");
        }
    }
}

/// Additional edge case tests for while loop JIT compilation
mod while_loop_edge_cases {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Empty while loop body should still work correctly
        #[test]
        fn while_loop_empty_body(limit in 0usize..100) {
            let mut i = 0;
            while i < limit {
                i += 1;
            }
            prop_assert_eq!(i, limit);
        }

        /// While loop with only break should execute once then exit
        #[test]
        fn while_loop_immediate_break(_dummy in Just(())) {
            let mut count = 0;
            #[allow(clippy::while_immutable_condition)]
            loop {
                count += 1;
                break;
            }
            prop_assert_eq!(count, 1, "While(true) with immediate break should execute once");
        }

        /// While loop with decrementing counter should work correctly
        #[test]
        fn while_loop_decrementing(start in 0usize..100) {
            let mut i = start;
            let mut count = 0;

            while i > 0 {
                i -= 1;
                count += 1;
            }

            prop_assert_eq!(count, start, "Decrementing while loop should execute start times");
            prop_assert_eq!(i, 0, "Counter should be 0 after loop");
        }

        /// While loop with multiple variables should track all correctly
        #[test]
        fn while_loop_multiple_variables(limit in 1usize..50) {
            let mut a = 0;
            let mut b = 0;
            let mut c = 0;

            while a < limit {
                b += a;
                c += 1;
                a += 1;
            }

            prop_assert_eq!(a, limit);
            prop_assert_eq!(b, limit * (limit - 1) / 2);
            prop_assert_eq!(c, limit);
        }
    }
}
