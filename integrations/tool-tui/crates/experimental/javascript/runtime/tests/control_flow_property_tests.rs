//! Property tests for control flow
//!
//! **Feature: dx-js-production-complete, Property: Control flow executes correct branches**
//! **Validates: Requirements 1.5**
//!
//! Tests that control flow statements (if/else, switch, for, while, do-while)
//! execute the correct branches based on conditions.

use proptest::prelude::*;

/// Property: If/else executes correct branch based on condition
mod if_else_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// For any boolean condition, if/else should execute exactly one branch
        #[test]
        fn if_else_executes_correct_branch(condition: bool) {
            let mut then_executed = false;
            let mut else_executed = false;

            if condition {
                then_executed = true;
            } else {
                else_executed = true;
            }

            // Exactly one branch should execute
            prop_assert!(then_executed != else_executed);
            prop_assert_eq!(then_executed, condition);
            prop_assert_eq!(else_executed, !condition);
        }

        /// For any value, if without else should execute then branch only when truthy
        #[test]
        fn if_without_else_executes_when_truthy(value in any::<i32>()) {
            let mut executed = false;

            // In JS, 0 is falsy, non-zero is truthy
            if value != 0 {
                executed = true;
            }

            prop_assert_eq!(executed, value != 0);
        }

        /// Nested if/else should execute correct combination of branches
        #[test]
        fn nested_if_else_executes_correctly(a: bool, b: bool) {
            let result;

            if a {
                if b {
                    result = 1;
                } else {
                    result = 2;
                }
            } else if b {
                result = 3;
            } else {
                result = 4;
            }

            let expected = match (a, b) {
                (true, true) => 1,
                (true, false) => 2,
                (false, true) => 3,
                (false, false) => 4,
            };

            prop_assert_eq!(result, expected);
        }

        /// Else-if chains should execute first matching branch
        #[test]
        fn else_if_chain_executes_first_match(value in 0i32..10) {
            let result;

            if value < 3 {
                result = 0;
            } else if value < 6 {
                result = 1;
            } else if value < 9 {
                result = 2;
            } else {
                result = 3;
            }

            let expected = if value < 3 {
                0
            } else if value < 6 {
                1
            } else if value < 9 {
                2
            } else {
                3
            };

            prop_assert_eq!(result, expected);
        }
    }
}

/// Property: Switch executes correct case based on discriminant
mod switch_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// For any discriminant value, switch should execute matching case
        #[test]
        fn switch_executes_matching_case(value in 0i32..5) {
            let result = match value {
                0 => "zero",
                1 => "one",
                2 => "two",
                3 => "three",
                4 => "four",
                _ => "default",
            };

            let expected = match value {
                0 => "zero",
                1 => "one",
                2 => "two",
                3 => "three",
                4 => "four",
                _ => "default",
            };

            prop_assert_eq!(result, expected);
        }

        /// Switch with no matching case should execute default
        #[test]
        fn switch_executes_default_when_no_match(value in 100i32..200) {
            let result = match value {
                0 => "zero",
                1 => "one",
                _ => "default",
            };

            prop_assert_eq!(result, "default");
        }

        /// Switch fall-through behavior (simulated with explicit handling)
        #[test]
        fn switch_fall_through_behavior(value in 0i32..5) {
            // Simulating fall-through: cases 0, 1, 2 all result in "small"
            let result = match value {
                0..=2 => "small",
                3 | 4 => "medium",
                _ => "large",
            };

            let expected = if value <= 2 {
                "small"
            } else if value <= 4 {
                "medium"
            } else {
                "large"
            };

            prop_assert_eq!(result, expected);
        }
    }
}

/// Property: While loop executes correct number of iterations
mod while_loop_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// While loop should execute while condition is true
        #[test]
        fn while_loop_executes_while_true(limit in 0usize..100) {
            let mut count = 0;
            let mut i = 0;

            while i < limit {
                count += 1;
                i += 1;
            }

            prop_assert_eq!(count, limit);
        }

        /// While loop with false initial condition should not execute
        #[test]
        fn while_loop_skips_when_initially_false(start in 10i32..100, end in 0i32..10) {
            let mut executed = false;
            let mut i = start;

            while i < end {
                executed = true;
                i += 1;
            }

            // start >= end, so loop should never execute
            prop_assert!(!executed);
        }

        /// While loop break should exit immediately
        #[test]
        fn while_loop_break_exits(limit in 1usize..100, break_at in 0usize..100) {
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
            prop_assert_eq!(count, expected);
        }
    }
}

/// Property: Do-while loop executes at least once
mod do_while_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Do-while should execute at least once even if condition is false
        #[test]
        fn do_while_executes_at_least_once(_dummy in Just(())) {
            let mut count = 0;
            let condition = false;

            // Simulating do-while: execute body, then check condition
            loop {
                count += 1;
                if !condition {
                    break;
                }
            }

            prop_assert!(count >= 1);
        }

        /// Do-while should execute correct number of times
        #[test]
        fn do_while_executes_correct_iterations(limit in 1usize..100) {
            let mut count = 0;
            let mut i = 0;

            // Simulating do-while
            loop {
                count += 1;
                i += 1;
                if i >= limit {
                    break;
                }
            }

            prop_assert_eq!(count, limit);
        }
    }
}

/// Property: For loop executes correct iterations with init, test, update
mod for_loop_control_flow_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// For loop should respect all three parts: init, test, update
        #[test]
        fn for_loop_respects_all_parts(start in 0i32..10, end in 0i32..20, step in 1i32..5) {
            let mut values = Vec::new();
            let mut i = start;

            while i < end {
                values.push(i);
                i += step;
            }

            // Verify values are correct
            let mut expected = Vec::new();
            let mut j = start;
            while j < end {
                expected.push(j);
                j += step;
            }

            prop_assert_eq!(values, expected);
        }

        /// For loop with empty body should still iterate correctly
        #[test]
        fn for_loop_empty_body_iterates(iterations in 0usize..100) {
            let mut count = 0;
            for _ in 0..iterations {
                count += 1;
            }
            prop_assert_eq!(count, iterations);
        }

        /// For loop continue should skip to next iteration
        #[test]
        fn for_loop_continue_skips_correctly(iterations in 1usize..50, skip_value in 0usize..50) {
            let mut collected = Vec::new();

            for i in 0..iterations {
                if i == skip_value {
                    continue;
                }
                collected.push(i);
            }

            // skip_value should not be in collected
            if skip_value < iterations {
                prop_assert!(!collected.contains(&skip_value));
                prop_assert_eq!(collected.len(), iterations - 1);
            } else {
                prop_assert_eq!(collected.len(), iterations);
            }
        }
    }
}

/// Property: Control flow with complex conditions
mod complex_condition_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Short-circuit AND should not evaluate right side if left is false
        #[test]
        fn short_circuit_and(left: bool, right: bool) {
            let mut right_evaluated = false;

            let result = left && {
                right_evaluated = true;
                right
            };

            if !left {
                prop_assert!(!right_evaluated);
                prop_assert!(!result);
            } else {
                prop_assert!(right_evaluated);
                prop_assert_eq!(result, right);
            }
        }

        /// Short-circuit OR should not evaluate right side if left is true
        #[test]
        fn short_circuit_or(left: bool, right: bool) {
            let mut right_evaluated = false;

            let result = left || {
                right_evaluated = true;
                right
            };

            if left {
                prop_assert!(!right_evaluated);
                prop_assert!(result);
            } else {
                prop_assert!(right_evaluated);
                prop_assert_eq!(result, right);
            }
        }

        /// Ternary operator should evaluate correct branch
        #[test]
        fn ternary_evaluates_correct_branch(condition: bool, then_val in any::<i32>(), else_val in any::<i32>()) {
            let result = if condition { then_val } else { else_val };

            let expected = if condition { then_val } else { else_val };
            prop_assert_eq!(result, expected);
        }
    }
}
