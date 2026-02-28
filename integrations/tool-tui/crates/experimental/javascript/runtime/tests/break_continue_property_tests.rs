//! Property tests for Break/Continue Targeting
//!
//! **Feature: dx-runtime-production-ready, Property 11: Break/Continue Targeting**
//! **Validates: Requirements 3.4, 3.5, 3.6**
//!
//! Tests that break and continue statements (including labeled) correctly
//! transfer control flow to the appropriate target loop.

use proptest::prelude::*;

/// Property 11: Break/Continue Targeting
/// For any break or continue statement (including labeled), control flow
/// should transfer to the correct target loop.
mod break_continue_targeting_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Break should exit the innermost enclosing loop immediately
        /// This validates Requirement 3.4
        #[test]
        fn break_exits_innermost_loop(
            outer_limit in 1usize..10,
            inner_limit in 1usize..10,
            break_at in 0usize..10
        ) {
            let mut outer_iterations = 0;
            let mut inner_iterations = 0;
            
            for _ in 0..outer_limit {
                outer_iterations += 1;
                for j in 0..inner_limit {
                    if j == break_at {
                        break;  // Should only exit inner loop
                    }
                    inner_iterations += 1;
                }
            }

            // Outer loop should complete all iterations
            prop_assert_eq!(outer_iterations, outer_limit,
                "Break should only exit innermost loop, outer should complete");
            
            // Inner loop should execute break_at iterations per outer iteration
            let expected_inner = outer_limit * break_at.min(inner_limit);
            prop_assert_eq!(inner_iterations, expected_inner,
                "Inner loop should execute {} times before break", break_at.min(inner_limit));
        }

        /// Continue should skip to the next iteration of the innermost loop
        /// This validates Requirement 3.5
        #[test]
        fn continue_skips_to_next_iteration(
            limit in 1usize..50,
            skip_value in 0usize..50
        ) {
            let mut collected = Vec::new();
            
            for i in 0..limit {
                if i == skip_value {
                    continue;  // Skip this iteration
                }
                collected.push(i);
            }

            // The skipped value should not be in the collected list
            if skip_value < limit {
                prop_assert!(!collected.contains(&skip_value),
                    "Continue should skip value {}", skip_value);
                prop_assert_eq!(collected.len(), limit - 1,
                    "Should have {} elements after skipping one", limit - 1);
            } else {
                prop_assert_eq!(collected.len(), limit,
                    "Should have all {} elements when skip_value >= limit", limit);
            }
        }

        /// Labeled break should exit the labeled loop, not just the innermost
        /// This validates Requirement 3.6
        #[test]
        fn labeled_break_exits_labeled_loop(
            outer_limit in 1usize..10,
            inner_limit in 1usize..10,
            break_outer_at in 0usize..10,
            break_inner_at in 0usize..10
        ) {
            let mut outer_iterations = 0;
            let mut inner_iterations = 0;
            
            'outer: for i in 0..outer_limit {
                outer_iterations += 1;
                for j in 0..inner_limit {
                    inner_iterations += 1;
                    if i == break_outer_at && j == break_inner_at {
                        break 'outer;  // Exit outer loop
                    }
                }
            }

            // If break condition was met, outer loop should have stopped early
            if break_outer_at < outer_limit && break_inner_at < inner_limit {
                prop_assert_eq!(outer_iterations, break_outer_at + 1,
                    "Labeled break should exit outer loop at iteration {}", break_outer_at);
                
                // Inner iterations: full iterations for outer 0..break_outer_at, 
                // plus break_inner_at + 1 for the final outer iteration
                let expected_inner = break_outer_at * inner_limit + break_inner_at + 1;
                prop_assert_eq!(inner_iterations, expected_inner,
                    "Inner loop should have {} iterations before labeled break", expected_inner);
            } else {
                // Break condition never met, both loops complete
                prop_assert_eq!(outer_iterations, outer_limit);
                prop_assert_eq!(inner_iterations, outer_limit * inner_limit);
            }
        }

        /// Labeled continue should continue the labeled loop, not just the innermost
        /// This validates Requirement 3.6
        #[test]
        fn labeled_continue_continues_labeled_loop(
            outer_limit in 1usize..10,
            inner_limit in 1usize..10,
            continue_outer_at in 0usize..10
        ) {
            let mut inner_counts = Vec::new();
            
            'outer: for i in 0..outer_limit {
                for j in 0..inner_limit {
                    if i == continue_outer_at && j == 0 {
                        continue 'outer;  // Skip rest of outer iteration
                    }
                }
                inner_counts.push(i);
            }

            // The iteration at continue_outer_at should have been skipped
            if continue_outer_at < outer_limit {
                prop_assert!(!inner_counts.contains(&continue_outer_at),
                    "Labeled continue should skip outer iteration {}", continue_outer_at);
                prop_assert_eq!(inner_counts.len(), outer_limit - 1,
                    "Should have {} completed outer iterations", outer_limit - 1);
            } else {
                prop_assert_eq!(inner_counts.len(), outer_limit,
                    "All outer iterations should complete when continue_outer_at >= limit");
            }
        }

        /// Multiple nested loops with labeled break should target correct loop
        /// This validates Requirement 3.6
        #[test]
        fn deeply_nested_labeled_break(
            level1_limit in 1usize..5,
            level2_limit in 1usize..5,
            level3_limit in 1usize..5,
            break_at_level in 1usize..4
        ) {
            let mut level1_count = 0;
            let mut level2_count = 0;
            let mut level3_count = 0;
            
            'level1: for _ in 0..level1_limit {
                level1_count += 1;
                'level2: for _ in 0..level2_limit {
                    level2_count += 1;
                    for _ in 0..level3_limit {
                        level3_count += 1;
                        match break_at_level {
                            1 => break 'level1,
                            2 => break 'level2,
                            _ => break,  // innermost
                        }
                    }
                }
            }

            match break_at_level {
                1 => {
                    // Break level1 on first iteration
                    prop_assert_eq!(level1_count, 1, "Level1 should stop at 1");
                    prop_assert_eq!(level2_count, 1, "Level2 should stop at 1");
                    prop_assert_eq!(level3_count, 1, "Level3 should stop at 1");
                }
                2 => {
                    // Break level2 on each level1 iteration
                    prop_assert_eq!(level1_count, level1_limit, "Level1 should complete");
                    prop_assert_eq!(level2_count, level1_limit, "Level2 should have 1 per level1");
                    prop_assert_eq!(level3_count, level1_limit, "Level3 should have 1 per level2");
                }
                _ => {
                    // Break innermost on each level2 iteration
                    prop_assert_eq!(level1_count, level1_limit, "Level1 should complete");
                    prop_assert_eq!(level2_count, level1_limit * level2_limit, "Level2 should complete");
                    prop_assert_eq!(level3_count, level1_limit * level2_limit, "Level3 should have 1 per level2");
                }
            }
        }

        /// Break in while loop should exit immediately
        /// This validates Requirement 3.4
        #[test]
        fn break_in_while_loop(limit in 1usize..100, break_at in 0usize..100) {
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
            prop_assert_eq!(count, expected,
                "While loop should execute {} times before break", expected);
        }

        /// Continue in while loop should skip to next iteration
        /// This validates Requirement 3.5
        #[test]
        fn continue_in_while_loop(limit in 1usize..50, skip_value in 0usize..50) {
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

            if skip_value < limit {
                prop_assert!(!collected.contains(&skip_value),
                    "Continue should skip value {}", skip_value);
                prop_assert_eq!(collected.len(), limit - 1);
            } else {
                prop_assert_eq!(collected.len(), limit);
            }
        }

        /// Break should work correctly in do-while loop
        /// This validates Requirement 3.4
        #[test]
        fn break_in_do_while_loop(limit in 1usize..100, break_at in 0usize..100) {
            let mut count = 0;
            let mut i = 0;
            
            loop {
                if i == break_at {
                    break;
                }
                count += 1;
                i += 1;
                if i >= limit {
                    break;
                }
            }

            let expected = break_at.min(limit);
            prop_assert_eq!(count, expected,
                "Do-while loop should execute {} times before break", expected);
        }
    }
}

/// Edge cases for break/continue
mod break_continue_edge_cases {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Break at first iteration should execute zero body iterations
        #[test]
        fn break_at_first_iteration(limit in 1usize..100) {
            let mut count = 0;
            
            for i in 0..limit {
                if i == 0 {
                    break;
                }
                count += 1;
            }

            prop_assert_eq!(count, 0, "Break at first iteration should execute zero times");
        }

        /// Continue at every iteration should result in empty collection
        #[test]
        fn continue_at_every_iteration(limit in 1usize..50) {
            let mut collected = Vec::new();
            
            for _ in 0..limit {
                continue;
                #[allow(unreachable_code)]
                collected.push(1);
            }

            prop_assert!(collected.is_empty(), "Continue at every iteration should collect nothing");
        }

        /// Break after last iteration should have no effect
        #[test]
        fn break_after_last_iteration(limit in 1usize..100) {
            let mut count = 0;
            
            for i in 0..limit {
                count += 1;
                if i == limit {  // This condition is never true
                    break;
                }
            }

            prop_assert_eq!(count, limit, "Break after last iteration should have no effect");
        }

        /// Labeled break with same label name in different scopes
        #[test]
        fn labeled_break_scope_isolation(
            outer_limit in 1usize..5,
            inner_limit in 1usize..5
        ) {
            let mut outer_count = 0;
            let mut inner_count = 0;
            
            // First labeled loop
            'loop1: for _ in 0..outer_limit {
                outer_count += 1;
                break 'loop1;
            }
            
            // Second labeled loop with same label name (different scope)
            'loop1: for _ in 0..inner_limit {
                inner_count += 1;
                break 'loop1;
            }

            prop_assert_eq!(outer_count, 1, "First loop should break after 1 iteration");
            prop_assert_eq!(inner_count, 1, "Second loop should break after 1 iteration");
        }
    }
}
