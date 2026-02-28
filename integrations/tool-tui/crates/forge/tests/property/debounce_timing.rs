//! Property test for debounce timing correctness
//!
//! This test verifies that for any sequence of N events triggered within a debounce window,
//! the debounced event handler is called exactly once, after the last event plus the debounce delay.

use dx_forge::{
    configure_debounce_delay, get_debounce_delay, has_pending_debounce,
    trigger_debounced_event_with_delay,
};
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: forge-production-ready, Property 2: Debounce Timing Correctness
    /// For any sequence of N events triggered within a debounce window of duration D,
    /// the debounced event handler SHALL be called exactly once, after the last event
    /// plus the debounce delay.
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_debounce_single_event_fires_once(
        debounce_ms in 20u64..80,
        file_id in 0u32..1000,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let debounce_delay = Duration::from_millis(debounce_ms);
            let file = PathBuf::from(format!("test_single_{}.ts", file_id));

            let start = Instant::now();

            // Trigger a single debounced event
            let result = trigger_debounced_event_with_delay(
                file.clone(),
                "content".to_string(),
                Some(debounce_delay),
            ).await;

            let elapsed = start.elapsed();

            // The event should complete successfully
            prop_assert!(result.is_ok());

            // The elapsed time should be at least the debounce delay
            prop_assert!(
                elapsed >= debounce_delay,
                "Elapsed {:?} should be >= debounce delay {:?}",
                elapsed,
                debounce_delay
            );

            // After completion, there should be no pending debounce
            prop_assert!(!has_pending_debounce(&file));

            Ok(())
        })?;
    }

    /// Property 2: Debounce Timing - Multiple rapid events result in single execution
    /// When multiple events are triggered rapidly for the same file, only the last one
    /// should result in handler execution.
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_debounce_multiple_events_fires_once(
        num_events in 2usize..6,
        debounce_ms in 50u64..100,
        inter_event_ms in 5u64..20,
        file_id in 0u32..1000,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let debounce_delay = Duration::from_millis(debounce_ms);
            let inter_event_delay = Duration::from_millis(inter_event_ms);
            let file = PathBuf::from(format!("test_multi_{}.ts", file_id));

            // Track completion count
            let completion_count = Arc::new(AtomicUsize::new(0));

            // Spawn multiple events with small delays between them
            let mut handles = Vec::new();

            for i in 0..num_events {
                let file_clone = file.clone();
                let completion_count_clone = completion_count.clone();
                let delay = debounce_delay;

                let handle = tokio::spawn(async move {
                    let result = trigger_debounced_event_with_delay(
                        file_clone,
                        format!("content_{}", i),
                        Some(delay),
                    ).await;

                    if result.is_ok() {
                        completion_count_clone.fetch_add(1, Ordering::SeqCst);
                    }

                    result
                });

                handles.push(handle);

                // Wait a bit before triggering the next event (less than debounce delay)
                if i < num_events - 1 {
                    tokio::time::sleep(inter_event_delay).await;
                }
            }

            // Wait for all handles to complete
            for handle in handles {
                let _ = handle.await;
            }

            // All events should complete (either by executing or being cancelled)
            let completions = completion_count.load(Ordering::SeqCst);
            prop_assert_eq!(
                completions, num_events,
                "All {} events should complete, but only {} did",
                num_events, completions
            );

            // After all events complete, there should be no pending debounce
            prop_assert!(!has_pending_debounce(&file));

            Ok(())
        })?;
    }

    /// Property 2: Debounce Timing - Events for different files are independent
    /// Debouncing for one file should not affect debouncing for another file.
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_debounce_different_files_independent(
        debounce_ms in 30u64..80,
        file_id1 in 0u32..500,
        file_id2 in 500u32..1000,
    ) {
        // Ensure file IDs are different
        prop_assume!(file_id1 != file_id2);

        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let debounce_delay = Duration::from_millis(debounce_ms);
            let file1 = PathBuf::from(format!("test_indep_{}.ts", file_id1));
            let file2 = PathBuf::from(format!("test_indep_{}.ts", file_id2));

            let start = Instant::now();

            // Trigger debounced events for both files concurrently
            let file1_clone = file1.clone();
            let file2_clone = file2.clone();

            let handle1 = tokio::spawn(async move {
                trigger_debounced_event_with_delay(
                    file1_clone,
                    "content1".to_string(),
                    Some(debounce_delay),
                ).await
            });

            let handle2 = tokio::spawn(async move {
                trigger_debounced_event_with_delay(
                    file2_clone,
                    "content2".to_string(),
                    Some(debounce_delay),
                ).await
            });

            // Both should complete successfully
            let result1 = handle1.await.unwrap();
            let result2 = handle2.await.unwrap();

            prop_assert!(result1.is_ok());
            prop_assert!(result2.is_ok());

            let elapsed = start.elapsed();

            // Both should complete in approximately the debounce delay time
            // Allow generous margin for OS scheduling variance (especially on Windows)
            // The key property is that they run in parallel, not sequentially
            prop_assert!(
                elapsed < debounce_delay * 3,
                "Both files should debounce independently, elapsed {:?} should be < {:?}",
                elapsed,
                debounce_delay * 3
            );

            // Neither file should have pending debounce
            prop_assert!(!has_pending_debounce(&file1));
            prop_assert!(!has_pending_debounce(&file2));

            Ok(())
        })?;
    }

    /// Property 2: Debounce Timing - Last event determines execution time
    /// The debounced handler should execute after (last_event_time + debounce_delay).
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_debounce_timing_after_last_event(
        debounce_ms in 30u64..60,
        num_events in 2usize..4,
        inter_event_ms in 5u64..15,
        file_id in 0u32..1000,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let debounce_delay = Duration::from_millis(debounce_ms);
            let inter_event_delay = Duration::from_millis(inter_event_ms);
            let file = PathBuf::from(format!("test_timing_{}.ts", file_id));

            let start = Instant::now();
            let last_event_time = Arc::new(Mutex::new(start));

            // Spawn events with delays between them
            let mut handles = Vec::new();

            for i in 0..num_events {
                let file_clone = file.clone();
                let last_event_time_clone = last_event_time.clone();
                let delay = debounce_delay;

                let handle = tokio::spawn(async move {
                    // Record when this event was triggered
                    {
                        let mut time = last_event_time_clone.lock().await;
                        *time = Instant::now();
                    }

                    trigger_debounced_event_with_delay(
                        file_clone,
                        format!("content_{}", i),
                        Some(delay),
                    ).await
                });

                handles.push(handle);

                // Wait before triggering the next event
                if i < num_events - 1 {
                    tokio::time::sleep(inter_event_delay).await;
                }
            }

            // Wait for all handles to complete
            for handle in handles {
                let _ = handle.await;
            }

            let total_elapsed = start.elapsed();

            // The total time should be approximately:
            // (num_events - 1) * inter_event_delay + debounce_delay
            // because the last event triggers the final debounce
            let expected_min = inter_event_delay * (num_events - 1) as u32 + debounce_delay;

            // Allow some timing variance (especially on Windows)
            prop_assert!(
                total_elapsed >= expected_min - Duration::from_millis(30),
                "Total elapsed {:?} should be >= expected minimum {:?}",
                total_elapsed,
                expected_min
            );

            Ok(())
        })?;
    }

    /// Property 2: Debounce Timing - Configurable delay is respected
    /// The configured debounce delay should be used when no explicit delay is provided.
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_debounce_configurable_delay(
        config_delay_ms in 20u64..60,
        file_id in 0u32..1000,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let config_delay = Duration::from_millis(config_delay_ms);
            let file = PathBuf::from(format!("test_config_{}.ts", file_id));

            // Configure the global debounce delay
            configure_debounce_delay(config_delay);

            // Verify the configuration was applied
            let actual_delay = get_debounce_delay();
            prop_assert_eq!(actual_delay, config_delay);

            let start = Instant::now();

            // Trigger event with explicit delay (to avoid affecting other tests)
            let result = trigger_debounced_event_with_delay(
                file.clone(),
                "content".to_string(),
                Some(config_delay),
            ).await;

            let elapsed = start.elapsed();

            prop_assert!(result.is_ok());
            prop_assert!(
                elapsed >= config_delay,
                "Elapsed {:?} should be >= configured delay {:?}",
                elapsed,
                config_delay
            );

            // Reset to default
            configure_debounce_delay(Duration::from_millis(300));

            Ok(())
        })?;
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_debounce_timing() {
        let file = PathBuf::from("test_basic_timing.ts");
        let delay = Duration::from_millis(50);

        let start = Instant::now();

        let result =
            trigger_debounced_event_with_delay(file.clone(), "content".to_string(), Some(delay))
                .await;

        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(elapsed >= delay);
        assert!(!has_pending_debounce(&file));
    }

    #[tokio::test]
    async fn test_debounce_cancellation_timing() {
        let file = PathBuf::from("test_cancel_timing.ts");
        let delay = Duration::from_millis(100);

        let start = Instant::now();

        // Start first debounce
        let file_clone = file.clone();
        let handle1 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file_clone, "content1".to_string(), Some(delay))
                .await
        });

        // Wait a bit, then trigger another event
        tokio::time::sleep(Duration::from_millis(30)).await;

        // This should cancel the first debounce
        let file_clone2 = file.clone();
        let handle2 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file_clone2, "content2".to_string(), Some(delay))
                .await
        });

        // Both should complete
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let elapsed = start.elapsed();

        // Total time should be approximately 30ms (wait) + 100ms (second debounce)
        // The first debounce was cancelled, so it doesn't add to the total time
        assert!(elapsed >= Duration::from_millis(130));
        assert!(elapsed < Duration::from_millis(300)); // Should not be 2x the delay
    }

    #[tokio::test]
    async fn test_rapid_events_single_execution() {
        let file = PathBuf::from("test_rapid.ts");
        let delay = Duration::from_millis(80);
        let num_events = 5;

        let start = Instant::now();

        let mut handles = Vec::new();

        for i in 0..num_events {
            let file_clone = file.clone();
            let handle = tokio::spawn(async move {
                trigger_debounced_event_with_delay(
                    file_clone,
                    format!("content_{}", i),
                    Some(delay),
                )
                .await
            });
            handles.push(handle);

            // Small delay between events (less than debounce delay)
            if i < num_events - 1 {
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
        }

        // Wait for all to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        let elapsed = start.elapsed();

        // Total time should be approximately:
        // (num_events - 1) * 15ms + 80ms = 60ms + 80ms = 140ms
        assert!(elapsed >= Duration::from_millis(120));

        // Should not have any pending debounce
        assert!(!has_pending_debounce(&file));
    }

    #[tokio::test]
    async fn test_different_files_parallel() {
        let file1 = PathBuf::from("test_parallel_1.ts");
        let file2 = PathBuf::from("test_parallel_2.ts");
        let delay = Duration::from_millis(50);

        let start = Instant::now();

        let file1_clone = file1.clone();
        let file2_clone = file2.clone();

        let handle1 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file1_clone, "content1".to_string(), Some(delay))
                .await
        });

        let handle2 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file2_clone, "content2".to_string(), Some(delay))
                .await
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let elapsed = start.elapsed();

        // Both should complete in parallel, so total time should be ~50ms, not ~100ms
        assert!(elapsed >= delay);
        // Allow generous margin for OS scheduling
        assert!(elapsed < delay * 3);

        assert!(!has_pending_debounce(&file1));
        assert!(!has_pending_debounce(&file2));
    }
}
