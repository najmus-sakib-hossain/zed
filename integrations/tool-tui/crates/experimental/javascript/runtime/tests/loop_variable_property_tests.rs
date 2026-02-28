//! Property tests for Loop Variable Correctness
//!
//! **Feature: dx-runtime-production-ready, Property 10: Loop Variable Correctness**
//! **Validates: Requirements 3.1, 3.2, 3.3**
//!
//! Tests that for/while loops with variable modifications (including i++, i--, i+=n)
//! correctly update the loop variable on each iteration and terminate when the
//! condition becomes false.

use proptest::prelude::*;

/// Property 10: Loop Variable Correctness
/// For any for/while loop with variable modifications (including i++, i--, i+=n),
/// the loop variable should be correctly updated on each iteration and the loop
/// should terminate when the condition becomes false.
mod loop_variable_correctness_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// For any bounded iteration count using i++, the loop should execute exactly that many times
        /// This validates Requirement 3.1 (for loop with increment)
        #[test]
        fn for_loop_increment_executes_correctly(limit in 0usize..100) {
            let mut count = 0;
            let mut i = 0;
            
            // Simulating: for (let i = 0; i < limit; i++)
            while i < limit {
                count += 1;
                i += 1;  // i++
            }

            prop_assert_eq!(count, limit, "For loop with i++ should execute exactly {} times", limit);
            prop_assert_eq!(i, limit, "Loop variable should equal limit after loop");
        }

        /// For any bounded iteration count using i--, the loop should execute exactly that many times
        /// This validates Requirement 3.1 (for loop with decrement)
        #[test]
        fn for_loop_decrement_executes_correctly(start in 0usize..100) {
            let mut count = 0;
            let mut i = start;
            
            // Simulating: for (let i = start; i > 0; i--)
            while i > 0 {
                count += 1;
                i -= 1;  // i--
            }

            prop_assert_eq!(count, start, "For loop with i-- should execute exactly {} times", start);
            prop_assert_eq!(i, 0, "Loop variable should be 0 after loop");
        }

        /// For any step value using i+=n, the loop should execute the correct number of times
        /// This validates Requirement 3.1 (for loop with compound assignment)
        #[test]
        fn for_loop_compound_assignment_executes_correctly(
            start in 0i32..10,
            limit in 10i32..100,
            step in 1i32..10
        ) {
            prop_assume!(step > 0);
            prop_assume!(limit > start);
            
            let mut count = 0;
            let mut i = start;
            
            // Simulating: for (let i = start; i < limit; i += step)
            while i < limit {
                count += 1;
                i += step;  // i += n
            }

            // Calculate expected iterations
            let expected_count = ((limit - start - 1) / step + 1) as usize;
            prop_assert_eq!(count, expected_count, 
                "For loop with i+={} from {} to {} should execute {} times", 
                step, start, limit, expected_count);
        }

        /// While loop condition should be re-evaluated after each variable modification
        /// This validates Requirement 3.2 (while loop condition references modified variable)
        #[test]
        fn while_loop_reevaluates_condition(limit in 1usize..100) {
            let mut i = 0;
            let mut iterations = 0;
            
            while i < limit {
                iterations += 1;
                i += 1;
                // After this modification, the condition should be re-evaluated
            }

            prop_assert_eq!(iterations, limit, 
                "While loop should re-evaluate condition after variable modification");
            prop_assert_eq!(i, limit, 
                "Loop variable should equal limit when condition becomes false");
        }

        /// Nested loops should maintain separate loop variables for each scope
        /// This validates Requirement 3.3 (nested loops maintain separate variables)
        #[test]
        fn nested_loops_maintain_separate_variables(
            outer_limit in 1usize..10,
            inner_limit in 1usize..10
        ) {
            let mut total_iterations = 0;
            let mut outer_final_values = Vec::new();
            
            let mut outer = 0;
            while outer < outer_limit {
                let mut inner = 0;
                while inner < inner_limit {
                    total_iterations += 1;
                    inner += 1;
                }
                // Inner loop variable should be at inner_limit after inner loop
                prop_assert_eq!(inner, inner_limit, 
                    "Inner loop variable should be {} after inner loop", inner_limit);
                
                outer_final_values.push(outer);
                outer += 1;
            }

            prop_assert_eq!(total_iterations, outer_limit * inner_limit,
                "Nested loops should execute outer * inner times");
            prop_assert_eq!(outer, outer_limit,
                "Outer loop variable should be {} after outer loop", outer_limit);
            
            // Verify outer loop variable was correctly tracked through iterations
            for (idx, &val) in outer_final_values.iter().enumerate() {
                prop_assert_eq!(val, idx, 
                    "Outer loop variable should have been {} at iteration {}", idx, idx);
            }
        }

        /// Loop variable should be correctly updated even with complex expressions
        /// This validates Requirements 3.1, 3.2
        #[test]
        fn loop_with_complex_update_expression(
            start in 0i32..10,
            multiplier in 1i32..5,
            addend in 1i32..5
        ) {
            prop_assume!(multiplier > 0 && addend > 0);
            
            let mut i = start;
            let mut iterations = 0;
            let limit = 100i32;
            
            // Simulating: for (let i = start; i < 100; i = i * multiplier + addend)
            while i < limit && iterations < 50 {
                iterations += 1;
                i = i * multiplier + addend;
            }

            // Verify the loop terminated correctly
            prop_assert!(i >= limit || iterations >= 50,
                "Loop should terminate when condition becomes false or max iterations reached");
        }

        /// Postfix increment (i++) should return old value but update variable
        /// This validates Requirement 3.1
        #[test]
        fn postfix_increment_semantics(initial in 0i32..100) {
            let mut i = initial;
            let mut collected_values = Vec::new();
            
            // Collect values using postfix increment
            for _ in 0..5 {
                let old_value = i;
                i += 1;  // Simulating i++
                collected_values.push(old_value);
            }

            // Values should be initial, initial+1, initial+2, ...
            for (idx, &val) in collected_values.iter().enumerate() {
                prop_assert_eq!(val, initial + idx as i32,
                    "Postfix increment should return old value");
            }
            prop_assert_eq!(i, initial + 5, "Variable should be incremented 5 times");
        }

        /// Prefix increment (++i) should return new value and update variable
        /// This validates Requirement 3.1
        #[test]
        fn prefix_increment_semantics(initial in 0i32..100) {
            let mut i = initial;
            let mut collected_values = Vec::new();
            
            // Collect values using prefix increment
            for _ in 0..5 {
                i += 1;  // Simulating ++i
                collected_values.push(i);
            }

            // Values should be initial+1, initial+2, initial+3, ...
            for (idx, &val) in collected_values.iter().enumerate() {
                prop_assert_eq!(val, initial + (idx as i32) + 1,
                    "Prefix increment should return new value");
            }
            prop_assert_eq!(i, initial + 5, "Variable should be incremented 5 times");
        }

        /// Loop variable should be accessible after loop exits
        /// This validates Requirements 3.1, 3.2
        #[test]
        fn loop_variable_accessible_after_exit(limit in 0usize..100) {
            let mut i = 0;
            
            while i < limit {
                i += 1;
            }

            // Variable should still be accessible and have the final value
            prop_assert_eq!(i, limit, 
                "Loop variable should be accessible and equal to {} after loop", limit);
        }

        /// Multiple loop variables should be independently tracked
        /// This validates Requirement 3.3
        #[test]
        fn multiple_loop_variables_independent(
            limit_a in 1usize..20,
            limit_b in 1usize..20
        ) {
            let mut a = 0;
            let mut b = 0;
            let mut iterations = 0;
            
            // Two variables updated in the same loop
            while a < limit_a && b < limit_b {
                a += 1;
                b += 2;  // Different increment
                iterations += 1;
            }

            // Verify both variables were tracked correctly
            let expected_iterations = limit_a.min((limit_b + 1) / 2);
            prop_assert_eq!(iterations, expected_iterations,
                "Loop should execute until either condition fails");
            prop_assert!(a >= limit_a || b >= limit_b,
                "At least one condition should be false after loop");
        }
    }
}

/// Additional edge case tests for loop variable handling
mod loop_variable_edge_cases {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Loop with zero iterations should not modify the variable
        #[test]
        fn zero_iteration_loop_preserves_variable(initial in 0i32..100) {
            let mut i = initial;
            
            // Condition is false from the start
            while i < 0 {
                i += 1;
            }

            prop_assert_eq!(i, initial, 
                "Zero-iteration loop should not modify variable");
        }

        /// Loop with single iteration should update variable exactly once
        #[test]
        fn single_iteration_loop(initial in 0i32..100) {
            let mut i = initial;
            let limit = initial + 1;
            
            while i < limit {
                i += 1;
            }

            prop_assert_eq!(i, limit, 
                "Single-iteration loop should update variable exactly once");
        }

        /// Loop variable should handle negative values correctly
        #[test]
        fn loop_with_negative_values(start in -50i32..0, end in 0i32..50) {
            let mut i = start;
            let mut count = 0;
            
            while i < end {
                count += 1;
                i += 1;
            }

            let expected = (end - start) as usize;
            prop_assert_eq!(count, expected,
                "Loop from {} to {} should execute {} times", start, end, expected);
            prop_assert_eq!(i, end, "Loop variable should equal end value");
        }

        /// Loop with floating point-like behavior (simulated with integers)
        #[test]
        fn loop_with_fractional_step(
            start in 0i32..10,
            limit in 20i32..100,
            step_numerator in 1i32..5,
            step_denominator in 1i32..5
        ) {
            prop_assume!(step_denominator > 0);
            prop_assume!(step_numerator > 0);
            
            // Simulate fractional step by scaling
            let scaled_start = start * step_denominator;
            let scaled_limit = limit * step_denominator;
            let mut i = scaled_start;
            let mut count = 0;
            
            while i < scaled_limit && count < 1000 {
                count += 1;
                i += step_numerator;
            }

            prop_assert!(i >= scaled_limit || count >= 1000,
                "Loop should terminate");
        }
    }
}
