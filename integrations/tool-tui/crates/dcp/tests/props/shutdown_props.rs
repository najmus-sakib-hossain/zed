//! Property-based tests for graceful shutdown.
//!
//! Feature: dcp-production, Property 18: Graceful Shutdown Drain

use dcp::shutdown::ShutdownCoordinator;
use proptest::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Create a runtime for async tests
fn rt() -> Runtime {
    Runtime::new().unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 18: Graceful Shutdown Drain
    /// For any in-flight request when shutdown is initiated, the server SHALL
    /// complete processing that request before terminating, up to the configured timeout.
    /// **Validates: Requirements 14.2**
    #[test]
    fn prop_graceful_shutdown_completes_in_flight(
        num_requests in 1usize..20,
        request_duration_ms in 10u64..100,
        drain_timeout_ms in 200u64..500,
    ) {
        let rt = rt();
        rt.block_on(async {
            let coord = Arc::new(ShutdownCoordinator::new(Duration::from_millis(drain_timeout_ms)));

            // Start multiple in-flight requests
            let mut guards = Vec::new();
            for _ in 0..num_requests {
                guards.push(coord.request_start());
            }

            assert_eq!(coord.in_flight_count(), num_requests as u64);

            // Initiate shutdown
            coord.shutdown();
            assert!(coord.is_shutdown());

            // Spawn task to complete requests after delay
            let guards_to_drop = guards;
            let request_duration = Duration::from_millis(request_duration_ms);
            tokio::spawn(async move {
                tokio::time::sleep(request_duration).await;
                drop(guards_to_drop);
            });

            // Wait for drain - should succeed since request_duration < drain_timeout
            let result = coord.wait_drain().await;
            assert!(result, "Drain should complete when requests finish before timeout");
            assert_eq!(coord.in_flight_count(), 0);
        });
    }

    /// Feature: dcp-production, Property 18: Graceful Shutdown Drain
    /// Drain SHALL timeout if requests don't complete in time.
    /// **Validates: Requirements 14.2, 14.3**
    #[test]
    fn prop_graceful_shutdown_timeout(
        num_requests in 1usize..10,
        drain_timeout_ms in 20u64..50,
    ) {
        let rt = rt();
        rt.block_on(async {
            let coord = Arc::new(ShutdownCoordinator::new(Duration::from_millis(drain_timeout_ms)));

            // Start requests that won't complete
            let mut guards = Vec::new();
            for _ in 0..num_requests {
                guards.push(coord.request_start());
            }

            // Initiate shutdown
            coord.shutdown();

            // Wait for drain - should timeout
            let result = coord.wait_drain().await;
            assert!(!result, "Drain should timeout when requests don't complete");
            assert_eq!(coord.in_flight_count(), num_requests as u64);

            // Clean up
            drop(guards);
        });
    }

    /// Feature: dcp-production, Property 18: Graceful Shutdown Drain
    /// Request guards SHALL correctly track in-flight count.
    /// **Validates: Requirements 14.2**
    #[test]
    fn prop_request_guard_tracking(
        operations in prop::collection::vec(prop::bool::ANY, 1..50),
    ) {
        let coord = ShutdownCoordinator::default();
        let mut guards = Vec::new();
        let mut expected_count = 0u64;

        for should_add in operations {
            if should_add {
                guards.push(coord.request_start());
                expected_count += 1;
            } else if !guards.is_empty() {
                guards.pop();
                expected_count -= 1;
            }

            assert_eq!(coord.in_flight_count(), expected_count);
        }

        // Drop all remaining guards
        drop(guards);
        assert_eq!(coord.in_flight_count(), 0);
    }

    /// Feature: dcp-production, Property 18: Graceful Shutdown Drain
    /// Shutdown SHALL be idempotent - multiple calls have same effect as one.
    /// **Validates: Requirements 14.1**
    #[test]
    fn prop_shutdown_idempotent(
        num_shutdowns in 1usize..10,
    ) {
        let coord = ShutdownCoordinator::default();

        assert!(!coord.is_shutdown());

        for _ in 0..num_shutdowns {
            coord.shutdown();
            assert!(coord.is_shutdown());
        }

        // Still shutdown after multiple calls
        assert!(coord.is_shutdown());
    }

    /// Feature: dcp-production, Property 18: Graceful Shutdown Drain
    /// Drain with zero in-flight requests SHALL complete immediately.
    /// **Validates: Requirements 14.2**
    #[test]
    fn prop_drain_immediate_when_empty(
        drain_timeout_ms in 100u64..1000,
    ) {
        let rt = rt();
        rt.block_on(async {
            let coord = ShutdownCoordinator::new(Duration::from_millis(drain_timeout_ms));

            // No in-flight requests
            assert_eq!(coord.in_flight_count(), 0);

            // Drain should complete immediately
            let start = std::time::Instant::now();
            let result = coord.wait_drain().await;
            let elapsed = start.elapsed();

            assert!(result);
            // Should complete in much less than the timeout
            assert!(elapsed < Duration::from_millis(50));
        });
    }

    /// Feature: dcp-production, Property 18: Graceful Shutdown Drain
    /// Concurrent request tracking SHALL be thread-safe.
    /// **Validates: Requirements 14.2**
    #[test]
    fn prop_concurrent_request_tracking(
        num_tasks in 2usize..10,
        requests_per_task in 1usize..20,
    ) {
        let rt = rt();
        rt.block_on(async {
            let coord = Arc::new(ShutdownCoordinator::default());

            let mut handles = Vec::new();

            // Spawn multiple tasks that add and remove requests
            for _ in 0..num_tasks {
                let coord_clone = Arc::clone(&coord);
                let count = requests_per_task;

                handles.push(tokio::spawn(async move {
                    let mut guards = Vec::new();

                    // Add requests
                    for _ in 0..count {
                        guards.push(coord_clone.request_start());
                        tokio::task::yield_now().await;
                    }

                    // Remove requests
                    while let Some(_) = guards.pop() {
                        tokio::task::yield_now().await;
                    }
                }));
            }

            // Wait for all tasks
            for handle in handles {
                handle.await.unwrap();
            }

            // All requests should be completed
            assert_eq!(coord.in_flight_count(), 0);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordinator_basic() {
        let coord = ShutdownCoordinator::default();

        assert!(!coord.is_shutdown());
        assert_eq!(coord.in_flight_count(), 0);

        coord.shutdown();
        assert!(coord.is_shutdown());
    }

    #[tokio::test]
    async fn test_drain_timeout_value() {
        let timeout = Duration::from_secs(42);
        let coord = ShutdownCoordinator::new(timeout);

        assert_eq!(coord.drain_timeout(), timeout);
    }
}
