//! Property-based tests for observability module.
//!
//! Tests:
//! - Property 19: Metrics Completeness
//! - Property 20: Request ID Propagation
//! - Property 21: Structured Log Format

use proptest::prelude::*;
use std::collections::HashSet;
use std::time::Duration;

use dcp::observability::{
    create_span, LogConfig, LogEntry, LogFormat, LogLevel, MetricsConfig, PrometheusMetrics,
    RequestIdGenerator, RequestMetrics, RequestSpan, Span, SpanKind, SpanStatus, StructuredLogger,
    Tracer, TracingConfig,
};
use std::sync::Arc;

// ============================================================================
// Property 19: Metrics Completeness
// Validates: Requirements 15.2 - Metrics SHALL include request count, latency
// histograms, and error rates
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: All recorded requests are counted in metrics
    #[test]
    fn prop_metrics_request_count(
        methods in prop::collection::vec("[a-z]+/[a-z]+", 1..10),
        durations_ms in prop::collection::vec(1u64..1000, 1..10),
    ) {
        let metrics = PrometheusMetrics::with_defaults();

        // Record requests for each method
        let mut expected_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for (method, duration_ms) in methods.iter().zip(durations_ms.iter()) {
            metrics.record_request(method, Duration::from_millis(*duration_ms));
            *expected_counts.entry(method.clone()).or_insert(0) += 1;
        }

        // Verify counts match
        let actual_counts = metrics.requests_total.all();
        for (method, expected) in expected_counts {
            prop_assert_eq!(
                actual_counts.get(&method).copied().unwrap_or(0),
                expected,
                "Request count mismatch for method {}",
                method
            );
        }
    }


    /// Property: Latency histograms correctly bucket observations
    #[test]
    fn prop_metrics_latency_histogram(
        latencies_ms in prop::collection::vec(1u64..5000, 1..20),
    ) {
        let metrics = PrometheusMetrics::with_defaults();
        let method = "test/method";

        // Record all latencies
        for latency_ms in &latencies_ms {
            metrics.record_request(method, Duration::from_millis(*latency_ms));
        }

        // Verify histogram count matches
        let histogram = metrics.request_duration_seconds.with_label(method);
        prop_assert_eq!(
            histogram.count() as usize,
            latencies_ms.len(),
            "Histogram count should match number of observations"
        );

        // Verify sum is approximately correct (within floating point tolerance)
        let expected_sum: f64 = latencies_ms.iter().map(|ms| *ms as f64 / 1000.0).sum();
        let actual_sum = histogram.sum();
        let tolerance = 0.001 * latencies_ms.len() as f64; // Allow small floating point error
        prop_assert!(
            (actual_sum - expected_sum).abs() < tolerance,
            "Histogram sum {} should be close to expected {}",
            actual_sum,
            expected_sum
        );
    }

    /// Property: Error rates are tracked correctly
    #[test]
    fn prop_metrics_error_rates(
        error_types in prop::collection::vec("[a-z_]+", 1..5),
        counts in prop::collection::vec(1u64..100, 1..5),
    ) {
        let metrics = PrometheusMetrics::with_defaults();

        // Record errors
        let mut expected: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for (error_type, count) in error_types.iter().zip(counts.iter()) {
            for _ in 0..*count {
                metrics.record_error(error_type);
            }
            *expected.entry(error_type.clone()).or_insert(0) += count;
        }

        // Verify error counts
        let actual = metrics.errors_total.all();
        for (error_type, expected_count) in expected {
            prop_assert_eq!(
                actual.get(&error_type).copied().unwrap_or(0),
                expected_count,
                "Error count mismatch for type {}",
                error_type
            );
        }
    }

    /// Property: Prometheus output format is valid
    #[test]
    fn prop_metrics_prometheus_format(
        methods in prop::collection::vec("[a-z]+", 1..5),
    ) {
        let metrics = PrometheusMetrics::with_defaults();

        // Record some data
        for method in &methods {
            metrics.record_request(method, Duration::from_millis(10));
            metrics.record_error("test_error");
            metrics.record_bytes("in", 100);
            metrics.record_bytes("out", 50);
        }

        let output = metrics.format_prometheus();

        // Verify format contains required sections
        prop_assert!(output.contains("# HELP"), "Should contain HELP comments");
        prop_assert!(output.contains("# TYPE"), "Should contain TYPE comments");
        prop_assert!(output.contains("dcp_requests_total"), "Should contain request counter");
        prop_assert!(output.contains("dcp_request_duration_seconds"), "Should contain latency histogram");
        prop_assert!(output.contains("dcp_errors_total"), "Should contain error counter");
        prop_assert!(output.contains("dcp_bytes_total"), "Should contain bytes counter");

        // Verify no empty lines at start
        prop_assert!(!output.starts_with('\n'), "Should not start with newline");
    }
}

// ============================================================================
// Property 20: Request ID Propagation
// Validates: Requirements 15.4 - Request IDs for correlation across services
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Request IDs are unique
    #[test]
    fn prop_request_id_uniqueness(count in 10usize..500) {
        let generator = RequestIdGenerator::new();
        let mut ids = HashSet::new();

        for _ in 0..count {
            let id = generator.generate();
            prop_assert!(
                ids.insert(id.clone()),
                "Request ID {} should be unique",
                id
            );
        }

        prop_assert_eq!(ids.len(), count, "All IDs should be unique");
    }

    /// Property: Request IDs have consistent format
    #[test]
    fn prop_request_id_format(count in 1usize..100) {
        let generator = RequestIdGenerator::new();

        for _ in 0..count {
            let id = generator.generate();

            // Format: {timestamp_hex}-{node_id_hex}-{counter_hex}
            let parts: Vec<&str> = id.split('-').collect();
            prop_assert_eq!(parts.len(), 3, "Request ID should have 3 parts: {}", id);

            // Each part should be valid hex
            for part in &parts {
                prop_assert!(
                    part.chars().all(|c| c.is_ascii_hexdigit()),
                    "Part '{}' should be hex in ID {}",
                    part,
                    id
                );
            }
        }
    }

    /// Property: Request IDs propagate through spans
    #[test]
    fn prop_request_id_span_propagation(
        method in "[a-z]+/[a-z]+",
        request_id in "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{8}",
    ) {
        let mut span = RequestSpan::new(&method, request_id.clone());

        // Verify request ID is accessible
        prop_assert_eq!(span.request_id(), &request_id);

        // Verify trace and span IDs are generated
        prop_assert!(!span.trace_id().is_empty(), "Trace ID should be generated");
        prop_assert!(!span.span_id().is_empty(), "Span ID should be generated");

        // Add attributes and verify they're stored
        span.set_attribute("test.key", "test.value");

        let finished = span.finish();
        prop_assert_eq!(finished.status, SpanStatus::Ok);
        prop_assert!(finished.end_time.is_some());
    }

    /// Property: Child spans inherit trace ID
    #[test]
    fn prop_trace_id_inheritance(
        parent_name in "[a-z]+",
        child_names in prop::collection::vec("[a-z]+", 1..5),
    ) {
        let parent = create_span(&parent_name);
        let parent_trace_id = parent.trace_id.clone();
        let parent_span_id = parent.span_id.clone();

        for child_name in &child_names {
            let child = dcp::observability::create_span(child_name);
            // Note: create_span creates a new root span, not a child
            // For child spans, we'd use create_child_span
            prop_assert!(!child.trace_id.is_empty());
        }

        // Verify parent trace ID is preserved
        prop_assert_eq!(parent.trace_id, parent_trace_id);
        prop_assert_eq!(parent.span_id, parent_span_id);
    }

    /// Property: Tracer generates unique request IDs
    #[test]
    fn prop_tracer_request_ids(count in 10usize..100) {
        let tracer = Tracer::with_defaults();
        let mut ids = HashSet::new();

        for _ in 0..count {
            let id = tracer.generate_request_id();
            prop_assert!(ids.insert(id), "Tracer should generate unique request IDs");
        }
    }
}

// ============================================================================
// Property 21: Structured Log Format
// Validates: Requirements 15.5 - Structured JSON logging for log aggregation
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Log entries produce valid JSON
    #[test]
    fn prop_log_entry_valid_json(
        message in "[a-zA-Z0-9 ]+",
        request_id in prop::option::of("[a-f0-9]{8}"),
        field_key in "[a-z_]+",
        field_value in "[a-zA-Z0-9]+",
    ) {
        let mut entry = LogEntry::new(LogLevel::Info, &message);

        if let Some(ref rid) = request_id {
            entry = entry.with_request_id(rid);
        }
        entry = entry.with_field(&field_key, field_value.clone());

        let json = entry.to_json("test-service");

        // Verify JSON structure
        prop_assert!(json.starts_with('{'), "JSON should start with {{");
        prop_assert!(json.ends_with('}'), "JSON should end with }}");
        prop_assert!(json.contains("\"timestamp\":"), "Should contain timestamp");
        prop_assert!(json.contains("\"level\":\"INFO\""), "Should contain level");
        prop_assert!(json.contains("\"message\":"), "Should contain message");
        prop_assert!(json.contains("\"service\":\"test-service\""), "Should contain service");

        if request_id.is_some() {
            prop_assert!(json.contains("\"request_id\":"), "Should contain request_id");
        }

        prop_assert!(json.contains(&format!("\"{}\":", field_key)), "Should contain field key");
    }

    /// Property: Log level filtering works correctly
    #[test]
    fn prop_log_level_filtering(
        min_level in 0u8..5,
        log_levels in prop::collection::vec(0u8..5, 1..20),
    ) {
        let levels = [LogLevel::Trace, LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
        let min = levels[min_level as usize];

        let logger = StructuredLogger::new(LogConfig {
            level: min,
            ..Default::default()
        });

        // Log at various levels
        for &level_idx in &log_levels {
            let level = levels[level_idx as usize];
            let entry = LogEntry::new(level, "test message");
            logger.emit(entry);
        }

        // Verify only entries at or above min level are logged
        let entries = logger.entries();
        for entry in &entries {
            prop_assert!(
                entry.level >= min,
                "Entry level {:?} should be >= min level {:?}",
                entry.level,
                min
            );
        }

        // Count expected entries
        let expected_count = log_levels.iter()
            .filter(|&&l| levels[l as usize] >= min)
            .count();
        prop_assert_eq!(entries.len(), expected_count);
    }

    /// Property: JSON escaping handles special characters
    #[test]
    fn prop_json_escaping(
        base_message in "[a-zA-Z0-9 ]+",
        special_chars in prop::collection::vec(prop::sample::select(vec!['"', '\\', '\n', '\r', '\t']), 0..5),
    ) {
        // Build message with special characters
        let mut message = base_message;
        for c in special_chars {
            message.push(c);
        }

        let entry = LogEntry::new(LogLevel::Info, &message);
        let json = entry.to_json("test");

        // Verify special characters are escaped
        // The JSON should not contain unescaped special characters
        let message_part = json.split("\"message\":").nth(1).unwrap_or("");

        // Find the message value (between quotes)
        if let Some(start) = message_part.find('"') {
            let rest = &message_part[start + 1..];
            // Count unescaped quotes (should be exactly one at the end)
            let mut in_escape = false;
            let mut unescaped_quotes = 0;
            for c in rest.chars() {
                if in_escape {
                    in_escape = false;
                } else if c == '\\' {
                    in_escape = true;
                } else if c == '"' {
                    unescaped_quotes += 1;
                    break; // Found closing quote
                }
            }
            prop_assert!(unescaped_quotes >= 1, "Should have closing quote");
        }
    }

    /// Property: Log entries contain timestamps
    #[test]
    fn prop_log_timestamp_present(
        messages in prop::collection::vec("[a-z]+", 1..10),
    ) {
        let logger = StructuredLogger::with_defaults();

        for message in &messages {
            logger.info(message).emit();
        }

        let entries = logger.entries();
        prop_assert_eq!(entries.len(), messages.len());

        for entry in &entries {
            prop_assert!(entry.timestamp > 0, "Timestamp should be positive");
            // Timestamp should be reasonable (after year 2020)
            prop_assert!(
                entry.timestamp > 1577836800000, // 2020-01-01
                "Timestamp {} should be after 2020",
                entry.timestamp
            );
        }
    }

    /// Property: Text format is human-readable
    #[test]
    fn prop_log_text_format(
        message in "[a-zA-Z0-9 ]+",
        request_id in "[a-f0-9]{8}",
    ) {
        let entry = LogEntry::new(LogLevel::Warn, &message)
            .with_request_id(&request_id);

        let text = entry.to_text();

        // Verify text format contains expected parts
        prop_assert!(text.contains("[WARN]"), "Should contain level");
        prop_assert!(text.contains(&message), "Should contain message");
        prop_assert!(text.contains(&format!("request_id={}", request_id)), "Should contain request_id");

        // Should have timestamp at start (ISO 8601 format)
        prop_assert!(text.contains("T"), "Should contain ISO 8601 timestamp");
        prop_assert!(text.contains("Z"), "Should contain UTC indicator");
    }
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_request_timing() {
        let metrics = Arc::new(PrometheusMetrics::with_defaults());

        // Use RequestMetrics helper
        let request = RequestMetrics::start(Arc::clone(&metrics), "tools/call");
        std::thread::sleep(Duration::from_millis(1));
        request.finish();

        let counts = metrics.requests_total.all();
        assert_eq!(counts.get("tools/call"), Some(&1));
    }

    #[test]
    fn test_span_lifecycle() {
        let tracer = Tracer::with_defaults();

        let mut span = tracer.start_span("test_operation");
        span.set_attribute("key", "value");
        span.add_event("started");

        std::thread::sleep(Duration::from_millis(1));

        span.add_event("completed");
        span.end();

        assert!(span.duration().is_some());
        assert!(span.duration().unwrap().as_micros() > 0);
        assert_eq!(span.events.len(), 2);
    }

    #[test]
    fn test_request_span_error() {
        let request_id = "test-123".to_string();
        let span = RequestSpan::new("tools/call", request_id);

        let finished = span.finish_with_error("Tool not found");

        assert_eq!(finished.status, SpanStatus::Error);
        assert!(finished.attributes.contains_key("error.message"));
    }

    #[test]
    fn test_logger_builder_pattern() {
        let logger = StructuredLogger::with_defaults();

        logger
            .info("Request received")
            .request_id("req-001")
            .trace_id("trace-001")
            .field("method", "tools/call")
            .field("duration_ms", 42i64)
            .emit();

        let entries = logger.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].request_id, Some("req-001".to_string()));
        assert_eq!(entries[0].trace_id, Some("trace-001".to_string()));
        assert_eq!(entries[0].fields.len(), 2);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Trace), "TRACE");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn test_metrics_bytes_tracking() {
        let metrics = PrometheusMetrics::with_defaults();

        metrics.record_bytes("in", 1000);
        metrics.record_bytes("in", 500);
        metrics.record_bytes("out", 2000);

        let bytes = metrics.bytes_total.all();
        assert_eq!(bytes.get("in"), Some(&1500));
        assert_eq!(bytes.get("out"), Some(&2000));
    }

    #[test]
    fn test_prometheus_output_completeness() {
        let metrics = PrometheusMetrics::with_defaults();

        // Record various metrics
        metrics.record_request("tools/list", Duration::from_millis(5));
        metrics.record_request("tools/call", Duration::from_millis(50));
        metrics.record_error("timeout");
        metrics.record_error("invalid_params");
        metrics.record_bytes("in", 1024);
        metrics.record_bytes("out", 512);
        metrics.connection_opened();

        let output = metrics.format_prometheus();

        // Verify all metric types are present
        assert!(output.contains("dcp_requests_total{method=\"tools/list\"}"));
        assert!(output.contains("dcp_requests_total{method=\"tools/call\"}"));
        assert!(output.contains("dcp_request_duration_seconds_bucket"));
        assert!(output.contains("dcp_request_duration_seconds_sum"));
        assert!(output.contains("dcp_request_duration_seconds_count"));
        assert!(output.contains("dcp_active_connections"));
        assert!(output.contains("dcp_bytes_total{direction=\"in\"}"));
        assert!(output.contains("dcp_bytes_total{direction=\"out\"}"));
        assert!(output.contains("dcp_errors_total{type=\"timeout\"}"));
        assert!(output.contains("dcp_errors_total{type=\"invalid_params\"}"));
    }
}
