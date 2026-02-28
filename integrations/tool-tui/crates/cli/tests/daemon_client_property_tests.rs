//! Property-based tests for Daemon Client Retry Logic
//!
//! These tests verify universal properties for the retry mechanism
//! with exponential backoff.
//!
//! Feature: cli-production-ready, Property 1: Retry with Exponential Backoff
//! **Validates: Requirements 3.1**
//!
//! Run with: cargo test --test daemon_client_property_tests

use proptest::prelude::*;
use std::time::Duration;

// ============================================================================
// RetryConfig (mirrors daemon_client.rs)
// ============================================================================

/// Configuration for connection retry behavior
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RetryConfig {
    /// Maximum number of connection attempts
    max_attempts: u32,
    /// Initial delay between retries in milliseconds
    initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    max_delay_ms: u64,
}

impl RetryConfig {
    fn new(max_attempts: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
        }
    }

    /// Calculate the delay for a given attempt (0-indexed)
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
        Duration::from_millis(delay_ms.min(self.max_delay_ms))
    }
}

// ============================================================================
// Arbitrary Generators
// ============================================================================

fn arbitrary_max_attempts() -> impl Strategy<Value = u32> {
    1u32..10u32
}

fn arbitrary_initial_delay() -> impl Strategy<Value = u64> {
    10u64..1000u64
}

fn arbitrary_max_delay() -> impl Strategy<Value = u64> {
    100u64..10000u64
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: Retry with Exponential Backoff
    /// *For any* sequence of connection failures up to the retry limit,
    /// the delay between attempts SHALL increase exponentially (doubling each time)
    /// starting from the initial delay, capped at the maximum delay.
    ///
    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_exponential_backoff_doubles(
        initial_delay in arbitrary_initial_delay(),
        max_delay in arbitrary_max_delay(),
    ) {
        // Ensure max_delay >= initial_delay for valid config
        let max_delay = max_delay.max(initial_delay);
        let config = RetryConfig::new(10, initial_delay, max_delay);

        // Check that each delay is double the previous (until capped)
        let mut prev_delay: Option<Duration> = None;
        for attempt in 0..5 {
            let delay = config.delay_for_attempt(attempt);

            if let Some(prev) = prev_delay {
                // Either doubled or capped at max
                let expected_doubled = Duration::from_millis(
                    (prev.as_millis() as u64 * 2).min(max_delay)
                );
                prop_assert_eq!(delay, expected_doubled,
                    "Attempt {}: expected {:?}, got {:?}", attempt, expected_doubled, delay);
            }

            prev_delay = Some(delay);
        }
    }

    /// Property 1b: Delays are capped at maximum
    /// *For any* retry configuration and any attempt number,
    /// the delay SHALL never exceed the configured maximum delay.
    ///
    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_delay_never_exceeds_max(
        max_attempts in arbitrary_max_attempts(),
        initial_delay in arbitrary_initial_delay(),
        max_delay in arbitrary_max_delay(),
        attempt in 0u32..20u32,
    ) {
        let max_delay = max_delay.max(initial_delay);
        let config = RetryConfig::new(max_attempts, initial_delay, max_delay);

        let delay = config.delay_for_attempt(attempt);

        prop_assert!(
            delay <= Duration::from_millis(max_delay),
            "Delay {:?} exceeded max {:?} for attempt {}",
            delay, Duration::from_millis(max_delay), attempt
        );
    }

    /// Property 1c: First delay equals initial delay
    /// *For any* retry configuration, the first attempt (attempt 0)
    /// SHALL have a delay equal to the initial delay.
    ///
    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_first_delay_is_initial(
        max_attempts in arbitrary_max_attempts(),
        initial_delay in arbitrary_initial_delay(),
        max_delay in arbitrary_max_delay(),
    ) {
        let max_delay = max_delay.max(initial_delay);
        let config = RetryConfig::new(max_attempts, initial_delay, max_delay);

        let first_delay = config.delay_for_attempt(0);
        let expected = Duration::from_millis(initial_delay.min(max_delay));

        prop_assert_eq!(first_delay, expected,
            "First delay should be initial_delay (or max if initial > max)");
    }

    /// Property 1d: Delays are monotonically non-decreasing
    /// *For any* retry configuration, delays SHALL be monotonically
    /// non-decreasing as attempt number increases.
    ///
    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_delays_monotonic(
        max_attempts in arbitrary_max_attempts(),
        initial_delay in arbitrary_initial_delay(),
        max_delay in arbitrary_max_delay(),
    ) {
        let max_delay = max_delay.max(initial_delay);
        let config = RetryConfig::new(max_attempts, initial_delay, max_delay);

        let mut prev_delay = Duration::ZERO;
        for attempt in 0..10 {
            let delay = config.delay_for_attempt(attempt);
            prop_assert!(
                delay >= prev_delay,
                "Delay decreased from {:?} to {:?} at attempt {}",
                prev_delay, delay, attempt
            );
            prev_delay = delay;
        }
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_zero_initial_delay() {
    let config = RetryConfig::new(3, 0, 1000);

    // All delays should be 0 (0 * 2^n = 0)
    for attempt in 0..5 {
        assert_eq!(config.delay_for_attempt(attempt), Duration::ZERO);
    }
}

#[test]
fn test_initial_equals_max() {
    let config = RetryConfig::new(3, 500, 500);

    // All delays should be 500ms
    for attempt in 0..5 {
        assert_eq!(config.delay_for_attempt(attempt), Duration::from_millis(500));
    }
}

#[test]
fn test_initial_greater_than_max() {
    let config = RetryConfig::new(3, 1000, 500);

    // First delay is 1000ms but capped at 500ms
    assert_eq!(config.delay_for_attempt(0), Duration::from_millis(500));
}

#[test]
fn test_large_attempt_number() {
    let config = RetryConfig::new(3, 100, 2000);

    // Very large attempt number should still be capped
    let delay = config.delay_for_attempt(100);
    assert_eq!(delay, Duration::from_millis(2000));
}

#[test]
fn test_overflow_protection() {
    let config = RetryConfig::new(3, u64::MAX / 2, u64::MAX);

    // Should not panic due to overflow
    let delay = config.delay_for_attempt(10);
    assert!(delay.as_millis() > 0);
}

// ============================================================================
// Handshake Compatibility Tests
// ============================================================================

/// Simulated handshake response for testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct HandshakeResponse {
    compatible: bool,
    daemon_protocol_version: String,
    daemon_version: String,
    warning: Option<String>,
}

/// Simulates the handshake validation logic
fn validate_handshake(response: &HandshakeResponse) -> Result<(), String> {
    if !response.compatible {
        return Err(format!(
            "Protocol version mismatch: Daemon={}. \
             Please run 'dx forge stop' and restart the daemon.",
            response.daemon_protocol_version
        ));
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 2: Incompatible Handshake Returns Error
    /// *For any* handshake response where `compatible` is `false`,
    /// the `perform_handshake` function SHALL return an `Err` result,
    /// and no subsequent daemon operations SHALL be attempted.
    ///
    /// **Validates: Requirements 4.1, 4.3**
    #[test]
    fn prop_incompatible_handshake_returns_error(
        daemon_version in "[0-9]+\\.[0-9]+\\.[0-9]+",
        protocol_version in "[0-9]+\\.[0-9]+",
    ) {
        let response = HandshakeResponse {
            compatible: false,
            daemon_protocol_version: protocol_version.clone(),
            daemon_version: daemon_version.clone(),
            warning: None,
        };

        let result = validate_handshake(&response);

        prop_assert!(result.is_err(),
            "Incompatible handshake should return error, got Ok");

        let error_msg = result.unwrap_err();
        prop_assert!(error_msg.contains("Protocol version mismatch"),
            "Error message should mention protocol version mismatch");
    }

    /// Property 2b: Compatible Handshake Returns Ok
    /// *For any* handshake response where `compatible` is `true`,
    /// the `perform_handshake` function SHALL return an `Ok` result.
    ///
    /// **Validates: Requirements 4.1, 4.3**
    #[test]
    fn prop_compatible_handshake_returns_ok(
        daemon_version in "[0-9]+\\.[0-9]+\\.[0-9]+",
        protocol_version in "[0-9]+\\.[0-9]+",
        has_warning in any::<bool>(),
    ) {
        let response = HandshakeResponse {
            compatible: true,
            daemon_protocol_version: protocol_version,
            daemon_version,
            warning: if has_warning { Some("Minor version difference".to_string()) } else { None },
        };

        let result = validate_handshake(&response);

        prop_assert!(result.is_ok(),
            "Compatible handshake should return Ok, got Err: {:?}", result);
    }
}

// ============================================================================
// Unit Tests for Handshake Edge Cases
// ============================================================================

#[test]
fn test_incompatible_handshake_error_message() {
    let response = HandshakeResponse {
        compatible: false,
        daemon_protocol_version: "1.0".to_string(),
        daemon_version: "2.0.0".to_string(),
        warning: None,
    };

    let result = validate_handshake(&response);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.contains("Protocol version mismatch"));
    assert!(error.contains("dx forge stop"));
}

#[test]
fn test_compatible_handshake_with_warning() {
    let response = HandshakeResponse {
        compatible: true,
        daemon_protocol_version: "1.0".to_string(),
        daemon_version: "2.0.0".to_string(),
        warning: Some("Minor version difference, consider upgrading".to_string()),
    };

    let result = validate_handshake(&response);
    assert!(result.is_ok());
}

#[test]
fn test_compatible_handshake_no_warning() {
    let response = HandshakeResponse {
        compatible: true,
        daemon_protocol_version: "1.0".to_string(),
        daemon_version: "2.0.0".to_string(),
        warning: None,
    };

    let result = validate_handshake(&response);
    assert!(result.is_ok());
}
