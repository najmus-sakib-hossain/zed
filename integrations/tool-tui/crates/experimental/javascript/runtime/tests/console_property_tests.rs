//! Property tests for Console API
//!
//! These tests verify universal correctness properties for the console API:
//! - Timer round-trip: time() followed by timeEnd() returns elapsed time
//! - Counter monotonicity: count() always returns increasing values
//!
//! **Feature: production-readiness**

use dx_js_runtime::runtime::console::{
    console_count, console_count_reset, console_time, console_time_end, get_counter,
    get_timer_elapsed, reset_console_state,
};
use proptest::prelude::*;
use std::thread;
use std::time::Duration;

// ============================================================================
// Property 1: Console Timer Round-Trip
// For any timer label, calling time() then timeEnd() SHALL return a non-negative
// elapsed time that is at least as long as any sleep performed between the calls.
// **Validates: Requirements 1.1, 1.2**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_console_timer_roundtrip(
        label in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        sleep_ms in 0u64..50u64
    ) {
        // Reset state for clean test
        reset_console_state();

        // Start timer
        console_time(&label);

        // Verify timer exists
        let elapsed_before = get_timer_elapsed(&label);
        prop_assert!(elapsed_before.is_some(), "Timer should exist after time()");

        // Sleep for specified duration
        if sleep_ms > 0 {
            thread::sleep(Duration::from_millis(sleep_ms));
        }

        // End timer and get elapsed time
        let elapsed = console_time_end(&label);

        // Property 1: timeEnd() should return Some(elapsed_time)
        prop_assert!(elapsed.is_some(), "timeEnd() should return elapsed time");

        // Property 2: Elapsed time should be non-negative
        let elapsed_ms = elapsed.unwrap();
        prop_assert!(elapsed_ms >= 0.0, "Elapsed time should be non-negative");

        // Property 3: Elapsed time should be at least the sleep duration
        // (with some tolerance for timing precision)
        if sleep_ms > 5 {
            prop_assert!(
                elapsed_ms >= (sleep_ms as f64 * 0.8),
                "Elapsed time {} should be at least ~{}ms",
                elapsed_ms,
                sleep_ms
            );
        }

        // Property 4: Timer should be removed after timeEnd()
        let elapsed_after = get_timer_elapsed(&label);
        prop_assert!(elapsed_after.is_none(), "Timer should be removed after timeEnd()");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_console_timer_default_label(sleep_ms in 0u64..20u64) {
        // Reset state for clean test
        reset_console_state();

        // Empty label should use "default"
        console_time("");

        // Verify timer exists with default label
        let elapsed_before = get_timer_elapsed("");
        prop_assert!(elapsed_before.is_some(), "Default timer should exist");

        if sleep_ms > 0 {
            thread::sleep(Duration::from_millis(sleep_ms));
        }

        // End timer
        let elapsed = console_time_end("");
        prop_assert!(elapsed.is_some(), "Default timer should return elapsed time");
        prop_assert!(elapsed.unwrap() >= 0.0, "Elapsed time should be non-negative");
    }
}

#[test]
fn test_console_timer_nonexistent() {
    reset_console_state();

    // Calling timeEnd on non-existent timer should return None
    let result = console_time_end("nonexistent");
    assert!(result.is_none(), "timeEnd on non-existent timer should return None");
}

#[test]
fn test_console_timer_duplicate_start() {
    reset_console_state();

    // Start timer
    console_time("test");
    let elapsed1 = get_timer_elapsed("test");
    assert!(elapsed1.is_some());

    // Sleep a bit
    thread::sleep(Duration::from_millis(10));

    // Try to start same timer again (should be ignored)
    console_time("test");

    // Timer should still have the original start time (elapsed should be >= 10ms)
    let elapsed2 = get_timer_elapsed("test");
    assert!(elapsed2.is_some());
    assert!(elapsed2.unwrap() >= 10.0, "Timer should not be reset on duplicate start");

    // Clean up
    console_time_end("test");
}

// ============================================================================
// Property 2: Console Counter Monotonicity
// For any counter label, successive calls to count() SHALL return strictly
// increasing values starting from 1.
// **Validates: Requirements 1.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_console_counter_monotonicity(
        label in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        call_count in 1usize..50usize
    ) {
        // Reset state for clean test
        reset_console_state();

        let mut previous_count = 0u32;

        for i in 0..call_count {
            let count = console_count(&label);

            // Property 1: Count should be exactly i + 1
            prop_assert_eq!(
                count,
                (i + 1) as u32,
                "Count should be {} on call {}", i + 1, i
            );

            // Property 2: Count should be strictly greater than previous
            prop_assert!(
                count > previous_count,
                "Count {} should be greater than previous {}",
                count,
                previous_count
            );

            // Property 3: Count should increase by exactly 1
            if previous_count > 0 {
                prop_assert_eq!(
                    count,
                    previous_count + 1,
                    "Count should increase by exactly 1"
                );
            }

            previous_count = count;
        }

        // Property 4: Final count should equal call_count
        let final_count = get_counter(&label);
        prop_assert_eq!(
            final_count,
            Some(call_count as u32),
            "Final count should equal number of calls"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_console_counter_reset(
        label in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        count_before_reset in 1usize..20usize,
        count_after_reset in 1usize..20usize
    ) {
        // Reset state for clean test
        reset_console_state();

        // Count several times
        for _ in 0..count_before_reset {
            console_count(&label);
        }

        // Verify count before reset
        let before = get_counter(&label);
        prop_assert_eq!(before, Some(count_before_reset as u32));

        // Reset counter
        console_count_reset(&label);

        // Property: Counter should be removed after reset
        let after_reset = get_counter(&label);
        prop_assert!(after_reset.is_none(), "Counter should be None after reset");

        // Count again - should start from 1
        for i in 0..count_after_reset {
            let count = console_count(&label);
            prop_assert_eq!(
                count,
                (i + 1) as u32,
                "Count should restart from 1 after reset"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_console_counter_independent_labels(
        label1 in "[a-z]{1,10}",
        label2 in "[A-Z]{1,10}",
        count1 in 1usize..20usize,
        count2 in 1usize..20usize
    ) {
        // Reset state for clean test
        reset_console_state();

        // Interleave counts for two different labels
        for i in 0..count1.max(count2) {
            if i < count1 {
                let c1 = console_count(&label1);
                prop_assert_eq!(c1, (i + 1) as u32, "Label1 count should be {}", i + 1);
            }
            if i < count2 {
                let c2 = console_count(&label2);
                prop_assert_eq!(c2, (i + 1) as u32, "Label2 count should be {}", i + 1);
            }
        }

        // Property: Each label should have independent count
        let final1 = get_counter(&label1);
        let final2 = get_counter(&label2);

        prop_assert_eq!(final1, Some(count1 as u32), "Label1 final count");
        prop_assert_eq!(final2, Some(count2 as u32), "Label2 final count");
    }
}

#[test]
fn test_console_counter_default_label() {
    reset_console_state();

    // Empty label should use "default"
    assert_eq!(console_count(""), 1);
    assert_eq!(console_count(""), 2);
    assert_eq!(get_counter(""), Some(2));

    // Reset with empty label
    console_count_reset("");
    assert_eq!(get_counter(""), None);

    // Should restart from 1
    assert_eq!(console_count(""), 1);
}

#[test]
fn test_console_counter_reset_nonexistent() {
    reset_console_state();

    // Resetting non-existent counter should not panic
    console_count_reset("nonexistent");

    // Counter should still not exist
    assert_eq!(get_counter("nonexistent"), None);
}
