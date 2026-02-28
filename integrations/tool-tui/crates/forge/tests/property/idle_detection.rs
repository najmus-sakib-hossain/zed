//! Property test for idle detection correctness
//!
//! This test verifies that for any idle period exceeding the configured idle threshold,
//! the idle event handler is triggered exactly once per idle period.
//!
//! Note: These tests use global state for idle detection, so they must be run
//! sequentially to avoid interference. Use `cargo test -- --test-threads=1` or
//! the TEST_MUTEX to serialize access.

use dx_forge::{
    configure_idle_threshold, get_idle_threshold, is_idle, record_activity,
    time_since_last_activity, trigger_idle_event_with_threshold,
};
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

// Mutex to serialize tests that use global idle state
static TEST_MUTEX: Mutex<()> = Mutex::new(());

// Default idle threshold to reset to after tests
const DEFAULT_IDLE_THRESHOLD_MS: u64 = 2000;

/// Helper to reset idle state before each test
fn reset_idle_state() {
    // Reset threshold to a known value
    configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    // Record activity to reset idle flags
    record_activity();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Feature: forge-production-ready, Property 3: Idle Detection Correctness
    /// For any idle period exceeding the configured idle threshold, the idle event handler
    /// SHALL be triggered exactly once per idle period.
    /// **Validates: Requirements 2.7, 2.8**
    #[test]
    fn prop_idle_event_fires_after_threshold(
        // Use larger thresholds for timing reliability
        threshold_ms in 80u64..150,
        file_id in 0u32..100,
    ) {
        let _guard = TEST_MUTEX.lock().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let threshold = Duration::from_millis(threshold_ms);
            let file = PathBuf::from(format!("test_idle_prop1_{}.ts", file_id));

            // Reset state
            reset_idle_state();
            configure_idle_threshold(threshold);

            // Verify we're not idle immediately after activity
            prop_assert!(!is_idle(), "Should not be idle immediately after activity");

            // Wait for threshold to pass with extra buffer for timing variance
            tokio::time::sleep(threshold + Duration::from_millis(50)).await;

            // Should be idle now
            prop_assert!(is_idle(), "Should be idle after threshold");

            // Trigger idle event - should succeed
            let result = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result.is_ok(), "Idle event should not error");
            prop_assert!(result.unwrap(), "Idle event should trigger after threshold");

            // Reset threshold to default
            configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));

            Ok(())
        })?;
    }

    /// Property 3: Idle Detection - No duplicate idle events during continued inactivity
    /// Once an idle event fires, it should NOT fire again during the same idle period.
    /// **Validates: Requirements 2.7, 2.8**
    #[test]
    fn prop_idle_event_fires_exactly_once_per_period(
        threshold_ms in 80u64..150,
        file_id in 0u32..100,
    ) {
        let _guard = TEST_MUTEX.lock().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let threshold = Duration::from_millis(threshold_ms);
            let file = PathBuf::from(format!("test_idle_prop2_{}.ts", file_id));

            // Reset state
            reset_idle_state();
            configure_idle_threshold(threshold);

            // Wait for threshold to pass
            tokio::time::sleep(threshold + Duration::from_millis(50)).await;

            // First idle event should trigger
            let result1 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result1.is_ok());
            prop_assert!(result1.unwrap(), "First idle event should trigger");

            // Second idle event should NOT trigger (same idle period)
            let result2 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result2.is_ok());
            prop_assert!(!result2.unwrap(), "Second idle event should NOT trigger in same period");

            // Third attempt should also not trigger
            let result3 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result3.is_ok());
            prop_assert!(!result3.unwrap(), "Third idle event should NOT trigger in same period");

            // Reset threshold to default
            configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));

            Ok(())
        })?;
    }

    /// Property 3: Idle Detection - Activity resets idle period
    /// After activity is recorded, a new idle period begins and idle events can fire again.
    /// **Validates: Requirements 2.7, 2.8**
    #[test]
    fn prop_activity_resets_idle_period(
        threshold_ms in 80u64..120,
        file_id in 0u32..100,
    ) {
        let _guard = TEST_MUTEX.lock().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let threshold = Duration::from_millis(threshold_ms);
            let file = PathBuf::from(format!("test_idle_prop3_{}.ts", file_id));

            // Reset state
            reset_idle_state();
            configure_idle_threshold(threshold);

            // Wait for threshold to pass
            tokio::time::sleep(threshold + Duration::from_millis(50)).await;

            // First idle event should trigger
            let result1 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result1.is_ok());
            prop_assert!(result1.unwrap(), "First idle event should trigger");

            // Record new activity - this starts a new idle period
            record_activity();

            // Verify we're no longer idle
            prop_assert!(!is_idle(), "Should not be idle after activity");

            // Wait for threshold again
            tokio::time::sleep(threshold + Duration::from_millis(50)).await;

            // Now idle event should trigger again (new idle period)
            let result2 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result2.is_ok());
            prop_assert!(result2.unwrap(), "Idle event should trigger for new idle period");

            // Reset threshold to default
            configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));

            Ok(())
        })?;
    }

    /// Property 3: Idle Detection - Activity cancels pending idle event
    /// If activity is detected before the idle threshold is reached, the idle event should not fire.
    /// **Validates: Requirements 2.7, 2.8**
    #[test]
    fn prop_activity_cancels_pending_idle_event(
        threshold_ms in 150u64..250,
        activity_delay_ms in 30u64..80,
        file_id in 0u32..100,
    ) {
        // Ensure activity happens well before threshold
        prop_assume!(activity_delay_ms < threshold_ms - 50);

        let _guard = TEST_MUTEX.lock().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let threshold = Duration::from_millis(threshold_ms);
            let activity_delay = Duration::from_millis(activity_delay_ms);
            let file = PathBuf::from(format!("test_idle_prop4_{}.ts", file_id));

            // Reset state
            reset_idle_state();
            configure_idle_threshold(threshold);

            // Start idle event detection in background
            let file_clone = file.clone();
            let handle = tokio::spawn(async move {
                trigger_idle_event_with_threshold(file_clone, Some(threshold)).await
            });

            // Wait a bit, then record activity (before threshold)
            tokio::time::sleep(activity_delay).await;
            record_activity();

            // The idle event should be cancelled
            let result = handle.await.unwrap();
            prop_assert!(result.is_ok());
            prop_assert!(!result.unwrap(), "Idle event should be cancelled by activity");

            // Reset threshold to default
            configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));

            Ok(())
        })?;
    }

    /// Property 3: Idle Detection - Configurable threshold is respected
    /// The configured idle threshold should determine when idle state is detected.
    /// **Validates: Requirements 2.7, 2.8**
    #[test]
    fn prop_idle_threshold_configurable(
        threshold_ms in 100u64..180,
        file_id in 0u32..100,
    ) {
        let _guard = TEST_MUTEX.lock().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let threshold = Duration::from_millis(threshold_ms);
            let file = PathBuf::from(format!("test_idle_prop5_{}.ts", file_id));

            // Reset state
            reset_idle_state();

            // Configure the idle threshold
            configure_idle_threshold(threshold);

            // Verify the configuration was applied
            let actual_threshold = get_idle_threshold();
            prop_assert_eq!(actual_threshold, threshold);

            // Wait for half the threshold - should NOT be idle
            tokio::time::sleep(threshold / 2).await;
            prop_assert!(!is_idle(), "Should not be idle before threshold");

            // Wait for the remaining time plus a buffer
            tokio::time::sleep(threshold / 2 + Duration::from_millis(50)).await;
            prop_assert!(is_idle(), "Should be idle after threshold");

            // Trigger idle event - should succeed
            let result = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap(), "Idle event should trigger");

            // Reset threshold to default
            configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));

            Ok(())
        })?;
    }

    /// Property 3: Idle Detection - Time since last activity is accurate
    /// The time_since_last_activity function should accurately track elapsed time.
    /// **Validates: Requirements 2.7, 2.8**
    #[test]
    fn prop_time_since_activity_accurate(
        wait_ms in 50u64..120,
    ) {
        let _guard = TEST_MUTEX.lock().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let wait_duration = Duration::from_millis(wait_ms);

            // Reset state
            reset_idle_state();

            // Time since activity should be very small
            let initial_elapsed = time_since_last_activity();
            prop_assert!(
                initial_elapsed < Duration::from_millis(100),
                "Initial elapsed {:?} should be small",
                initial_elapsed
            );

            // Wait for specified duration
            tokio::time::sleep(wait_duration).await;

            // Time since activity should be approximately the wait duration
            let elapsed = time_since_last_activity();
            prop_assert!(
                elapsed >= wait_duration,
                "Elapsed {:?} should be >= wait duration {:?}",
                elapsed,
                wait_duration
            );

            // Allow generous margin for timing variance (especially on Windows)
            prop_assert!(
                elapsed < wait_duration + Duration::from_millis(200),
                "Elapsed {:?} should be close to wait duration {:?}",
                elapsed,
                wait_duration
            );

            Ok(())
        })?;
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_idle_detection() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let file = PathBuf::from("test_basic_idle.ts");
        let threshold = Duration::from_millis(100);

        // Reset state
        reset_idle_state();
        configure_idle_threshold(threshold);

        // Should not be idle immediately
        assert!(!is_idle());

        // Wait for threshold with buffer
        tokio::time::sleep(threshold + Duration::from_millis(50)).await;

        // Should be idle now
        assert!(is_idle());

        // Trigger idle event
        let result = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Reset
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_idle_event_not_triggered_twice() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let file = PathBuf::from("test_idle_twice.ts");
        let threshold = Duration::from_millis(100);

        // Reset state
        reset_idle_state();
        configure_idle_threshold(threshold);

        // Wait for threshold
        tokio::time::sleep(threshold + Duration::from_millis(50)).await;

        // First idle event should trigger
        let result1 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
        assert!(result1.is_ok());
        assert!(result1.unwrap());

        // Second idle event should NOT trigger
        let result2 = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result2.is_ok());
        assert!(!result2.unwrap());

        // Reset
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_activity_resets_idle_state() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let file = PathBuf::from("test_activity_reset.ts");
        let threshold = Duration::from_millis(100);

        // Reset state
        reset_idle_state();
        configure_idle_threshold(threshold);

        // Wait for threshold
        tokio::time::sleep(threshold + Duration::from_millis(50)).await;

        // First idle event should trigger
        let result1 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
        assert!(result1.unwrap());

        // Record new activity
        record_activity();

        // Should not be idle anymore
        assert!(!is_idle());

        // Wait for threshold again
        tokio::time::sleep(threshold + Duration::from_millis(50)).await;

        // Idle event should trigger again (new period)
        let result2 = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result2.is_ok());
        assert!(result2.unwrap());

        // Reset
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_activity_cancels_pending_idle() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let file = PathBuf::from("test_cancel_idle.ts");
        let threshold = Duration::from_millis(300);

        // Reset state
        reset_idle_state();
        configure_idle_threshold(threshold);

        // Start idle event detection in background
        let file_clone = file.clone();
        let handle = tokio::spawn(async move {
            trigger_idle_event_with_threshold(file_clone, Some(threshold)).await
        });

        // Wait a bit, then record activity (before threshold)
        tokio::time::sleep(Duration::from_millis(80)).await;
        record_activity();

        // The idle event should be cancelled
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Reset
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_configure_idle_threshold() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let custom_threshold = Duration::from_millis(150);

        configure_idle_threshold(custom_threshold);
        assert_eq!(get_idle_threshold(), custom_threshold);

        // Reset to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_time_since_last_activity() {
        let _guard = TEST_MUTEX.lock().unwrap();

        // Reset state
        reset_idle_state();

        // Time since activity should be very small
        let elapsed = time_since_last_activity();
        assert!(elapsed < Duration::from_millis(100));

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Time since activity should have increased
        let elapsed2 = time_since_last_activity();
        assert!(elapsed2 >= Duration::from_millis(100));
    }
}
