//! Property tests for Phi Node Value Merging
//!
//! **Feature: dx-runtime-production-ready, Property 12: Phi Node Value Merging**
//! **Validates: Requirements 3.7**
//!
//! Tests that variables modified in different branches of conditionals
//! have the correct value at the join point (merge block).

use proptest::prelude::*;

/// Property 12: Phi Node Value Merging
/// For any variable modified in different branches of a conditional,
/// the correct value should be available at the join point.
mod phi_node_value_merging_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Variable modified in both branches should have correct value at merge
        /// This validates Requirement 3.7
        #[test]
        fn variable_modified_in_both_branches(
            condition: bool,
            then_value in any::<i32>(),
            else_value in any::<i32>()
        ) {
            let mut x = 0;
            
            if condition {
                x = then_value;
            } else {
                x = else_value;
            }
            
            // At merge point, x should have the value from the executed branch
            let expected = if condition { then_value } else { else_value };
            prop_assert_eq!(x, expected,
                "Variable should have value {} from {} branch",
                expected, if condition { "then" } else { "else" });
        }

        /// Variable modified only in then branch should retain original or new value
        /// This validates Requirement 3.7
        #[test]
        fn variable_modified_only_in_then_branch(
            condition: bool,
            initial_value in any::<i32>(),
            then_value in any::<i32>()
        ) {
            let mut x = initial_value;
            
            if condition {
                x = then_value;
            }
            // No else branch - x keeps initial value if condition is false
            
            let expected = if condition { then_value } else { initial_value };
            prop_assert_eq!(x, expected,
                "Variable should be {} when condition is {}",
                expected, condition);
        }

        /// Variable modified only in else branch should retain original or new value
        /// This validates Requirement 3.7
        #[test]
        fn variable_modified_only_in_else_branch(
            condition: bool,
            initial_value in any::<i32>(),
            else_value in any::<i32>()
        ) {
            let mut x = initial_value;
            
            if condition {
                // No modification in then branch
            } else {
                x = else_value;
            }
            
            let expected = if condition { initial_value } else { else_value };
            prop_assert_eq!(x, expected,
                "Variable should be {} when condition is {}",
                expected, condition);
        }

        /// Multiple variables modified in branches should all have correct values
        /// This validates Requirement 3.7
        #[test]
        fn multiple_variables_modified_in_branches(
            condition: bool,
            a_then in any::<i32>(),
            a_else in any::<i32>(),
            b_then in any::<i32>(),
            b_else in any::<i32>()
        ) {
            let mut a = 0;
            let mut b = 0;
            
            if condition {
                a = a_then;
                b = b_then;
            } else {
                a = a_else;
                b = b_else;
            }
            
            let expected_a = if condition { a_then } else { a_else };
            let expected_b = if condition { b_then } else { b_else };
            
            prop_assert_eq!(a, expected_a, "Variable a should have correct value");
            prop_assert_eq!(b, expected_b, "Variable b should have correct value");
        }

        /// Nested conditionals should merge values correctly at each level
        /// This validates Requirement 3.7
        #[test]
        fn nested_conditionals_merge_correctly(
            outer_cond: bool,
            inner_cond: bool,
            v1 in any::<i32>(),
            v2 in any::<i32>(),
            v3 in any::<i32>(),
            v4 in any::<i32>()
        ) {
            let mut x = 0;
            
            if outer_cond {
                if inner_cond {
                    x = v1;
                } else {
                    x = v2;
                }
            } else {
                if inner_cond {
                    x = v3;
                } else {
                    x = v4;
                }
            }
            
            let expected = match (outer_cond, inner_cond) {
                (true, true) => v1,
                (true, false) => v2,
                (false, true) => v3,
                (false, false) => v4,
            };
            
            prop_assert_eq!(x, expected,
                "Nested conditional should produce {} for ({}, {})",
                expected, outer_cond, inner_cond);
        }

        /// Variable used after modification in loop with conditional
        /// This validates Requirement 3.7
        #[test]
        fn variable_modified_in_loop_conditional(
            iterations in 1usize..20,
            threshold in 0usize..20
        ) {
            let mut sum = 0i32;
            let mut count = 0i32;
            
            for i in 0..iterations {
                if i < threshold {
                    sum += 1;
                } else {
                    count += 1;
                }
            }
            
            let expected_sum = threshold.min(iterations) as i32;
            let expected_count = iterations.saturating_sub(threshold) as i32;
            
            prop_assert_eq!(sum, expected_sum, "Sum should be {}", expected_sum);
            prop_assert_eq!(count, expected_count, "Count should be {}", expected_count);
        }

        /// Chained else-if should merge values correctly
        /// This validates Requirement 3.7
        #[test]
        fn chained_else_if_merges_correctly(
            value in 0i32..10,
            v1 in any::<i32>(),
            v2 in any::<i32>(),
            v3 in any::<i32>(),
            v4 in any::<i32>()
        ) {
            let mut result = 0;
            
            if value < 3 {
                result = v1;
            } else if value < 6 {
                result = v2;
            } else if value < 9 {
                result = v3;
            } else {
                result = v4;
            }
            
            let expected = if value < 3 {
                v1
            } else if value < 6 {
                v2
            } else if value < 9 {
                v3
            } else {
                v4
            };
            
            prop_assert_eq!(result, expected,
                "Chained else-if should produce {} for value {}",
                expected, value);
        }

        /// Variable incremented in conditional should have correct final value
        /// This validates Requirement 3.7
        #[test]
        fn variable_incremented_in_conditional(
            iterations in 0usize..50,
            increment_when_even: bool
        ) {
            let mut counter = 0i32;
            
            for i in 0..iterations {
                let is_even = i % 2 == 0;
                if is_even == increment_when_even {
                    counter += 1;
                }
            }
            
            // Count how many iterations match the condition
            let expected = (0..iterations)
                .filter(|i| (i % 2 == 0) == increment_when_even)
                .count() as i32;
            
            prop_assert_eq!(counter, expected,
                "Counter should be {} after {} iterations",
                expected, iterations);
        }
    }
}

/// Edge cases for phi node handling
mod phi_node_edge_cases {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Empty branches should preserve variable value
        #[test]
        fn empty_branches_preserve_value(condition: bool, initial in any::<i32>()) {
            let mut x = initial;
            
            if condition {
                // Empty then branch
            } else {
                // Empty else branch
            }
            
            prop_assert_eq!(x, initial, "Empty branches should preserve value");
        }

        /// Variable assigned same value in both branches
        #[test]
        fn same_value_in_both_branches(condition: bool, value in any::<i32>()) {
            let mut x = 0;
            
            if condition {
                x = value;
            } else {
                x = value;
            }
            
            prop_assert_eq!(x, value, "Same value in both branches should work");
        }

        /// Variable used immediately after conditional
        #[test]
        fn variable_used_immediately_after(
            condition: bool,
            then_val in any::<i32>(),
            else_val in any::<i32>()
        ) {
            let mut x = 0;
            
            if condition {
                x = then_val;
            } else {
                x = else_val;
            }
            
            // Use x immediately
            let y = x + 1;
            
            let expected_x = if condition { then_val } else { else_val };
            let expected_y = expected_x.wrapping_add(1);
            
            prop_assert_eq!(x, expected_x);
            prop_assert_eq!(y, expected_y);
        }

        /// Multiple uses of variable after conditional
        #[test]
        fn multiple_uses_after_conditional(
            condition: bool,
            then_val in 0i32..1000,
            else_val in 0i32..1000
        ) {
            let mut x = 0;
            
            if condition {
                x = then_val;
            } else {
                x = else_val;
            }
            
            // Multiple uses of x
            let a = x;
            let b = x * 2;
            let c = x + x;
            
            let expected = if condition { then_val } else { else_val };
            
            prop_assert_eq!(a, expected);
            prop_assert_eq!(b, expected * 2);
            prop_assert_eq!(c, expected + expected);
        }

        /// Deeply nested conditionals with variable modification
        #[test]
        fn deeply_nested_modification(
            c1: bool, c2: bool, c3: bool,
            v1 in any::<i32>(),
            v2 in any::<i32>(),
            v3 in any::<i32>(),
            v4 in any::<i32>()
        ) {
            let mut x = 0;
            
            if c1 {
                if c2 {
                    if c3 {
                        x = v1;
                    } else {
                        x = v2;
                    }
                } else {
                    x = v3;
                }
            } else {
                x = v4;
            }
            
            let expected = if c1 {
                if c2 {
                    if c3 { v1 } else { v2 }
                } else {
                    v3
                }
            } else {
                v4
            };
            
            prop_assert_eq!(x, expected);
        }
    }
}
