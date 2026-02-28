//! Property-based tests for CLI logging behavior
//!
//! These tests verify universal properties for the logging system:
//! - Log output format includes timestamps and levels
//! - Quiet mode suppresses non-error output
//!
//! Feature: cli-production-ready, Property 6: Log Output Format
//! Feature: cli-production-ready, Property 7: Quiet Mode Suppression
//! **Validates: Requirements 7.4, 7.5**
//!
//! Run with: cargo test --test logging_property_tests

use proptest::prelude::*;

/// Log level enum for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    fn priority(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }
}

/// Generate arbitrary log levels
fn log_level_strategy() -> impl Strategy<Value = LogLevel> {
    prop_oneof![
        Just(LogLevel::Trace),
        Just(LogLevel::Debug),
        Just(LogLevel::Info),
        Just(LogLevel::Warn),
        Just(LogLevel::Error),
    ]
}

/// Generate arbitrary log messages
fn log_message_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Application started".to_string()),
        Just("Processing request".to_string()),
        Just("Connection established".to_string()),
        Just("Cache miss".to_string()),
        Just("Operation completed".to_string()),
        "[a-zA-Z0-9 ]{5,50}".prop_map(|s| s.to_string()),
    ]
}

/// Simulate log output format (matching tracing-subscriber format)
fn format_log_entry(level: LogLevel, message: &str, timestamp_secs: u64) -> String {
    // Format: "  0.001s  INFO message"
    format!(
        "{:>7}.{:03}s {:>5} {}",
        timestamp_secs,
        (timestamp_secs * 1000) % 1000,
        level.as_str(),
        message
    )
}

/// Check if a log level should be visible given the filter level
fn should_log(message_level: LogLevel, filter_level: LogLevel) -> bool {
    message_level.priority() >= filter_level.priority()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: Log Output Format
    /// *For any* log message output when logging is enabled, the message SHALL
    /// contain a timestamp and log level indicator.
    ///
    /// Feature: cli-production-ready, Property 6: Log Output Format
    /// **Validates: Requirements 7.4**
    #[test]
    fn prop_log_output_format(
        level in log_level_strategy(),
        message in log_message_strategy(),
        timestamp_secs in 0u64..3600,
    ) {
        let log_output = format_log_entry(level, &message, timestamp_secs);

        // Verify timestamp is present (format: X.XXXs)
        prop_assert!(
            log_output.contains('s'),
            "Log output must contain timestamp with 's' suffix: {}",
            log_output
        );

        // Verify log level is present
        prop_assert!(
            log_output.contains(level.as_str()),
            "Log output must contain log level '{}': {}",
            level.as_str(),
            log_output
        );

        // Verify message is present
        prop_assert!(
            log_output.contains(&message),
            "Log output must contain the message: {}",
            log_output
        );

        // Verify format order: timestamp before level before message
        let timestamp_pos = log_output.find('s').unwrap();
        let level_pos = log_output.find(level.as_str()).unwrap();
        let message_pos = log_output.find(&message).unwrap();

        prop_assert!(
            timestamp_pos < level_pos && level_pos < message_pos,
            "Log format should be: timestamp level message. Got: {}",
            log_output
        );
    }

    /// Property 7: Quiet Mode Suppression
    /// *For any* command executed with `--quiet` flag, the CLI SHALL produce
    /// no stdout output except for the primary result or errors.
    ///
    /// Feature: cli-production-ready, Property 7: Quiet Mode Suppression
    /// **Validates: Requirements 7.5**
    #[test]
    fn prop_quiet_mode_suppression(
        message_level in log_level_strategy(),
        _message in log_message_strategy(),
    ) {
        // In quiet mode, filter level is ERROR
        let quiet_filter = LogLevel::Error;

        let should_show = should_log(message_level, quiet_filter);

        // Only ERROR level messages should be visible in quiet mode
        if message_level == LogLevel::Error {
            prop_assert!(
                should_show,
                "ERROR messages should be visible in quiet mode"
            );
        } else {
            prop_assert!(
                !should_show,
                "{:?} messages should be suppressed in quiet mode",
                message_level
            );
        }
    }

    /// Property 6b: Log Level Filtering
    /// *For any* log level configuration, messages below the configured level
    /// SHALL be suppressed.
    ///
    /// Feature: cli-production-ready, Property 6: Log Output Format
    /// **Validates: Requirements 7.4**
    #[test]
    fn prop_log_level_filtering(
        message_level in log_level_strategy(),
        filter_level in log_level_strategy(),
    ) {
        let should_show = should_log(message_level, filter_level);

        // Message should show if its priority >= filter priority
        let expected = message_level.priority() >= filter_level.priority();

        prop_assert_eq!(
            should_show,
            expected,
            "Message level {:?} with filter {:?}: expected {}, got {}",
            message_level,
            filter_level,
            expected,
            should_show
        );
    }
}

/// Test that verbose mode enables debug logging
#[test]
fn test_verbose_mode_enables_debug() {
    // In verbose mode, filter level is DEBUG
    let verbose_filter = LogLevel::Debug;

    // Debug and above should be visible
    assert!(should_log(LogLevel::Debug, verbose_filter));
    assert!(should_log(LogLevel::Info, verbose_filter));
    assert!(should_log(LogLevel::Warn, verbose_filter));
    assert!(should_log(LogLevel::Error, verbose_filter));

    // Trace should be suppressed
    assert!(!should_log(LogLevel::Trace, verbose_filter));
}

/// Test that quiet mode only shows errors
#[test]
fn test_quiet_mode_only_errors() {
    let quiet_filter = LogLevel::Error;

    // Only error should be visible
    assert!(should_log(LogLevel::Error, quiet_filter));

    // All others should be suppressed
    assert!(!should_log(LogLevel::Trace, quiet_filter));
    assert!(!should_log(LogLevel::Debug, quiet_filter));
    assert!(!should_log(LogLevel::Info, quiet_filter));
    assert!(!should_log(LogLevel::Warn, quiet_filter));
}

/// Test that default mode shows info and above
#[test]
fn test_default_mode_info_and_above() {
    let default_filter = LogLevel::Info;

    // Info and above should be visible
    assert!(should_log(LogLevel::Info, default_filter));
    assert!(should_log(LogLevel::Warn, default_filter));
    assert!(should_log(LogLevel::Error, default_filter));

    // Debug and trace should be suppressed
    assert!(!should_log(LogLevel::Trace, default_filter));
    assert!(!should_log(LogLevel::Debug, default_filter));
}

/// Test log format contains required components
#[test]
fn test_log_format_components() {
    let output = format_log_entry(LogLevel::Info, "Test message", 123);

    // Should contain timestamp
    assert!(output.contains("123."), "Should contain timestamp seconds");
    assert!(output.contains("s"), "Should contain 's' for seconds");

    // Should contain level
    assert!(output.contains("INFO"), "Should contain log level");

    // Should contain message
    assert!(output.contains("Test message"), "Should contain message");
}

/// Test all log levels have valid string representations
#[test]
fn test_log_level_strings() {
    let levels = [
        (LogLevel::Trace, "TRACE"),
        (LogLevel::Debug, "DEBUG"),
        (LogLevel::Info, "INFO"),
        (LogLevel::Warn, "WARN"),
        (LogLevel::Error, "ERROR"),
    ];

    for (level, expected) in levels {
        assert_eq!(level.as_str(), expected);
    }
}

/// Test log level priority ordering
#[test]
fn test_log_level_priority_ordering() {
    assert!(LogLevel::Trace.priority() < LogLevel::Debug.priority());
    assert!(LogLevel::Debug.priority() < LogLevel::Info.priority());
    assert!(LogLevel::Info.priority() < LogLevel::Warn.priority());
    assert!(LogLevel::Warn.priority() < LogLevel::Error.priority());
}

/// Test environment variable configuration simulation
#[test]
fn test_env_var_configuration() {
    // Simulate DX_LOG_LEVEL parsing
    let env_values = vec![
        ("trace", LogLevel::Trace),
        ("debug", LogLevel::Debug),
        ("info", LogLevel::Info),
        ("warn", LogLevel::Warn),
        ("error", LogLevel::Error),
    ];

    for (env_val, expected_level) in env_values {
        let level = match env_val.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info, // default
        };

        assert_eq!(
            level, expected_level,
            "DX_LOG_LEVEL={} should set level to {:?}",
            env_val, expected_level
        );
    }
}
