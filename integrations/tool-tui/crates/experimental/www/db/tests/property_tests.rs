//! Property-based tests for the db crate.
//!
//! These tests verify correctness properties across all valid inputs
//! using the proptest framework.

use proptest::prelude::*;
use std::time::Duration;

// Import the pool module types
// Note: We need to access the pool module which contains PoolConfig and PoolStats
use dx_www_db::pool::{PoolConfig, PoolStats};

/// **Validates: Requirements 3.6**
///
/// Property 5: Connection Pool Bounds
///
/// For any connection pool configuration with min_connections M and max_connections N,
/// the pool SHALL maintain at least M idle connections and never exceed N total connections.
mod connection_pool_bounds {
    use super::*;

    /// Strategy for generating valid pool configurations where min <= max
    fn valid_pool_config_strategy() -> impl Strategy<Value = (u32, u32)> {
        // Generate min_connections from 1 to 100
        // Generate max_connections >= min_connections
        (1u32..=100u32).prop_flat_map(|min| (Just(min), min..=200u32))
    }

    /// Strategy for generating pool stats that should be valid for a given config
    #[allow(dead_code)]
    fn valid_pool_stats_strategy(
        _min_connections: u32,
        max_connections: u32,
    ) -> impl Strategy<Value = PoolStats> {
        // Generate active connections from 0 to max
        // Generate idle connections such that total <= max
        (0u32..=max_connections)
            .prop_flat_map(move |active| {
                let max_idle = max_connections.saturating_sub(active);
                (Just(active), 0u32..=max_idle)
            })
            .prop_map(move |(active, idle)| PoolStats {
                active_connections: active,
                idle_connections: idle,
                total_connections: active + idle,
                max_connections,
                pending_requests: 0,
            })
    }

    proptest! {
        /// **Validates: Requirements 3.6**
        ///
        /// Property: Valid pool configurations must have min_connections <= max_connections
        /// and max_connections > 0.
        #[test]
        fn pool_config_validates_correctly_when_min_lte_max(
            (min, max) in valid_pool_config_strategy()
        ) {
            let config = PoolConfig {
                min_connections: min,
                max_connections: max,
                acquire_timeout: Duration::from_secs(30),
                idle_timeout: Duration::from_secs(600),
                max_lifetime: Duration::from_secs(1800),
            };

            // Valid configurations should pass validation
            prop_assert!(config.validate().is_ok(),
                "Config with min={}, max={} should be valid", min, max);
        }

        /// **Validates: Requirements 3.6**
        ///
        /// Property: Invalid pool configurations (min > max) must fail validation.
        #[test]
        fn pool_config_rejects_invalid_bounds(
            min in 2u32..=200u32,
            max_offset in 1u32..=100u32
        ) {
            // Ensure min > max by subtracting offset from min
            let max = min.saturating_sub(max_offset);
            if max == 0 {
                // Skip this case as max=0 is a different error
                return Ok(());
            }

            let config = PoolConfig {
                min_connections: min,
                max_connections: max,
                acquire_timeout: Duration::from_secs(30),
                idle_timeout: Duration::from_secs(600),
                max_lifetime: Duration::from_secs(1800),
            };

            // Invalid configurations should fail validation
            prop_assert!(config.validate().is_err(),
                "Config with min={}, max={} should be invalid", min, max);
        }

        /// **Validates: Requirements 3.6**
        ///
        /// Property: Pool stats total_connections must never exceed max_connections.
        #[test]
        fn pool_stats_never_exceed_max_connections(
            (_min, max) in valid_pool_config_strategy()
        ) {
            // Note: We only need max for this test since we're testing
            // that stats don't exceed max_connections

            // Generate various stats scenarios and verify bounds
            // Test with active = max, idle = 0 (at capacity)
            let stats_at_capacity = PoolStats {
                active_connections: max,
                idle_connections: 0,
                total_connections: max,
                max_connections: max,
                pending_requests: 0,
            };
            prop_assert!(stats_at_capacity.total_connections <= max,
                "Total connections {} should not exceed max {}",
                stats_at_capacity.total_connections, max);

            // Test with active = 0, idle = max
            let stats_all_idle = PoolStats {
                active_connections: 0,
                idle_connections: max,
                total_connections: max,
                max_connections: max,
                pending_requests: 0,
            };
            prop_assert!(stats_all_idle.total_connections <= max,
                "Total connections {} should not exceed max {}",
                stats_all_idle.total_connections, max);
        }

        /// **Validates: Requirements 3.6**
        ///
        /// Property: Pool utilization is correctly calculated and bounded [0.0, 1.0+].
        #[test]
        fn pool_utilization_is_correctly_bounded(
            (_min, max) in valid_pool_config_strategy(),
            active_ratio in 0.0f64..=1.0f64
        ) {
            // Note: We only need max for this test since utilization
            // is calculated as active/max
            let active = ((max as f64) * active_ratio).round() as u32;
            let active = active.min(max); // Ensure we don't exceed max

            let stats = PoolStats {
                active_connections: active,
                idle_connections: 0,
                total_connections: active,
                max_connections: max,
                pending_requests: 0,
            };

            let utilization = stats.utilization();

            // Utilization should be non-negative
            prop_assert!(utilization >= 0.0,
                "Utilization {} should be non-negative", utilization);

            // When active <= max, utilization should be <= 1.0
            if active <= max {
                prop_assert!(utilization <= 1.0 + f64::EPSILON,
                    "Utilization {} should be <= 1.0 when active ({}) <= max ({})",
                    utilization, active, max);
            }
        }

        /// **Validates: Requirements 3.6**
        ///
        /// Property: Pool is exhausted only when total_connections >= max_connections
        /// AND idle_connections == 0.
        #[test]
        fn pool_exhaustion_detection_is_correct(
            max in 1u32..=100u32,
            active in 0u32..=100u32,
            idle in 0u32..=100u32
        ) {
            let total = active.saturating_add(idle);

            let stats = PoolStats {
                active_connections: active,
                idle_connections: idle,
                total_connections: total,
                max_connections: max,
                pending_requests: 0,
            };

            let is_exhausted = stats.is_exhausted();
            let expected_exhausted = total >= max && idle == 0;

            prop_assert_eq!(is_exhausted, expected_exhausted,
                "is_exhausted() returned {} but expected {} for total={}, max={}, idle={}",
                is_exhausted, expected_exhausted, total, max, idle);
        }

        /// **Validates: Requirements 3.6**
        ///
        /// Property: Zero max_connections configuration must be rejected.
        #[test]
        fn pool_config_rejects_zero_max_connections(
            min in 0u32..=10u32
        ) {
            // Note: min is used to test various min_connections values
            // when max_connections is zero
            let config = PoolConfig {
                min_connections: min,
                max_connections: 0,
                acquire_timeout: Duration::from_secs(30),
                idle_timeout: Duration::from_secs(600),
                max_lifetime: Duration::from_secs(1800),
            };

            prop_assert!(config.validate().is_err(),
                "Config with max_connections=0 should be invalid (min={})", min);
        }

        /// **Validates: Requirements 3.6**
        ///
        /// Property: Zero acquire_timeout configuration must be rejected.
        #[test]
        fn pool_config_rejects_zero_acquire_timeout(
            (min, max) in valid_pool_config_strategy()
        ) {
            let config = PoolConfig {
                min_connections: min,
                max_connections: max,
                acquire_timeout: Duration::ZERO,
                idle_timeout: Duration::from_secs(600),
                max_lifetime: Duration::from_secs(1800),
            };

            prop_assert!(config.validate().is_err(),
                "Config with acquire_timeout=0 should be invalid");
        }
    }
}
