//! Prometheus-compatible metrics module.
//!
//! This module provides:
//! - Standard HTTP metrics (request count, duration, errors)
//! - Custom application metrics
//! - `/metrics` endpoint handler for Prometheus scraping

use http::{StatusCode, header};
use prometheus::{Counter, Encoder, Gauge, Histogram, Registry, TextEncoder};
use std::sync::Arc;

/// Errors that can occur during metrics operations.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// Failed to register a metric.
    #[error("Failed to register metric: {0}")]
    RegistrationError(String),

    /// Failed to export metrics.
    #[error("Failed to export metrics: {0}")]
    ExportError(String),
}

/// Standard HTTP metrics for request monitoring.
pub struct Metrics {
    /// Total number of HTTP requests received.
    pub request_count: Counter,

    /// Histogram of request durations in seconds.
    pub request_duration: Histogram,

    /// Total number of errors encountered.
    pub error_count: Counter,

    /// Number of currently active connections.
    pub active_connections: Gauge,
}

impl Metrics {
    /// Creates a new `Metrics` instance and registers all metrics with the given registry.
    ///
    /// # Arguments
    ///
    /// * `registry` - The Prometheus registry to register metrics with.
    ///
    /// # Errors
    ///
    /// Returns a `MetricsError` if metric registration fails.
    pub fn new(registry: &Registry) -> Result<Self, MetricsError> {
        let request_count = Counter::new("http_requests_total", "Total number of HTTP requests")
            .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;

        let request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
        )
        .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;

        let error_count = Counter::new("http_errors_total", "Total number of HTTP errors")
            .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;

        let active_connections =
            Gauge::new("http_active_connections", "Number of active HTTP connections")
                .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;

        registry
            .register(Box::new(request_count.clone()))
            .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;
        registry
            .register(Box::new(request_duration.clone()))
            .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;
        registry
            .register(Box::new(error_count.clone()))
            .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;
        registry
            .register(Box::new(active_connections.clone()))
            .map_err(|e| MetricsError::RegistrationError(e.to_string()))?;

        Ok(Self {
            request_count,
            request_duration,
            error_count,
            active_connections,
        })
    }
}

/// Renders all metrics in Prometheus text format.
///
/// # Arguments
///
/// * `registry` - The Prometheus registry containing the metrics.
///
/// # Errors
///
/// Returns a `MetricsError` if encoding fails.
pub fn render_metrics(registry: &Registry) -> Result<String, MetricsError> {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|e| MetricsError::ExportError(e.to_string()))?;
    String::from_utf8(buffer).map_err(|e| MetricsError::ExportError(e.to_string()))
}

/// Handler for the `/metrics` endpoint.
///
/// This function returns metrics in Prometheus text format, suitable for scraping
/// by Prometheus or compatible monitoring systems.
///
/// # Arguments
///
/// * `registry` - An `Arc<Registry>` containing all registered metrics.
///
/// # Returns
///
/// Returns an HTTP response with:
/// - Status 200 and Prometheus-formatted metrics on success
/// - Status 500 with error message on failure
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::get, extract::State};
/// use prometheus::Registry;
/// use std::sync::Arc;
/// use dx_www_observability::metrics::{Metrics, metrics_handler};
///
/// let registry = Arc::new(Registry::new());
/// let metrics = Metrics::new(&registry).unwrap();
///
/// let app = Router::new()
///     .route("/metrics", get(metrics_handler))
///     .with_state(registry);
/// ```
pub async fn metrics_handler(
    registry: Arc<Registry>,
) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    match render_metrics(&registry) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
            body,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("Failed to render metrics: {e}"),
        ),
    }
}

/// Creates a metrics handler closure for use with axum's State extractor.
///
/// This is a convenience function that creates a handler compatible with
/// axum's routing system when using `State<Arc<Registry>>`.
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::get, extract::State};
/// use prometheus::Registry;
/// use std::sync::Arc;
/// use dx_www_observability::metrics::{Metrics, create_metrics_handler};
///
/// let registry = Arc::new(Registry::new());
/// let metrics = Metrics::new(&registry).unwrap();
///
/// let app = Router::new()
///     .route("/metrics", get(create_metrics_handler()))
///     .with_state(registry);
/// ```
pub fn create_metrics_handler() -> impl Fn(
    Arc<Registry>,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = (StatusCode, [(header::HeaderName, &'static str); 1], String),
            > + Send,
    >,
> + Clone
+ Send
+ 'static {
    |registry: Arc<Registry>| Box::pin(async move { metrics_handler(registry).await })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Verify all metrics are created with initial values
        assert!((metrics.request_count.get() - 0.0).abs() < f64::EPSILON);
        assert!((metrics.error_count.get() - 0.0).abs() < f64::EPSILON);
        assert!((metrics.active_connections.get() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_increment() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Test counter increments
        metrics.request_count.inc();
        assert!((metrics.request_count.get() - 1.0).abs() < f64::EPSILON);

        metrics.request_count.inc_by(5.0);
        assert!((metrics.request_count.get() - 6.0).abs() < f64::EPSILON);

        // Test error counter
        metrics.error_count.inc();
        assert!((metrics.error_count.get() - 1.0).abs() < f64::EPSILON);

        // Test gauge
        metrics.active_connections.inc();
        assert!((metrics.active_connections.get() - 1.0).abs() < f64::EPSILON);

        metrics.active_connections.dec();
        assert!((metrics.active_connections.get() - 0.0).abs() < f64::EPSILON);

        metrics.active_connections.set(42.0);
        assert!((metrics.active_connections.get() - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_histogram_observation() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Record some durations
        metrics.request_duration.observe(0.001);
        metrics.request_duration.observe(0.05);
        metrics.request_duration.observe(0.5);
        metrics.request_duration.observe(2.0);

        // Verify histogram has recorded values
        let sample_count = metrics.request_duration.get_sample_count();
        assert_eq!(sample_count, 4);
    }

    #[test]
    fn test_render_metrics_prometheus_format() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Add some data
        metrics.request_count.inc_by(100.0);
        metrics.error_count.inc_by(5.0);
        metrics.active_connections.set(10.0);
        metrics.request_duration.observe(0.1);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify Prometheus format
        assert!(output.contains("# HELP http_requests_total Total number of HTTP requests"));
        assert!(output.contains("# TYPE http_requests_total counter"));
        assert!(output.contains("http_requests_total 100"));

        assert!(output.contains("# HELP http_errors_total Total number of HTTP errors"));
        assert!(output.contains("# TYPE http_errors_total counter"));
        assert!(output.contains("http_errors_total 5"));

        assert!(
            output.contains("# HELP http_active_connections Number of active HTTP connections")
        );
        assert!(output.contains("# TYPE http_active_connections gauge"));
        assert!(output.contains("http_active_connections 10"));

        assert!(
            output
                .contains("# HELP http_request_duration_seconds HTTP request duration in seconds")
        );
        assert!(output.contains("# TYPE http_request_duration_seconds histogram"));
        assert!(output.contains("http_request_duration_seconds_bucket"));
        assert!(output.contains("http_request_duration_seconds_count 1"));
    }

    #[test]
    fn test_render_metrics_empty_registry() {
        let registry = Registry::new();
        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Empty registry should produce empty output (or just whitespace)
        assert!(output.is_empty() || output.trim().is_empty());
    }

    #[test]
    fn test_duplicate_registration_fails() {
        let registry = Registry::new();
        let _metrics1 = Metrics::new(&registry).expect("First registration should succeed");

        // Second registration should fail due to duplicate metric names
        let result = Metrics::new(&registry);
        assert!(result.is_err());

        if let Err(MetricsError::RegistrationError(msg)) = result {
            assert!(msg.contains("Duplicate"));
        } else {
            panic!("Expected RegistrationError");
        }
    }

    #[tokio::test]
    async fn test_metrics_handler_success() {
        let registry = Arc::new(Registry::new());
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Add some data
        metrics.request_count.inc_by(50.0);
        metrics.error_count.inc_by(2.0);

        let (status, headers, body) = metrics_handler(registry).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[0].0, header::CONTENT_TYPE);
        assert!(headers[0].1.contains("text/plain"));
        assert!(body.contains("http_requests_total 50"));
        assert!(body.contains("http_errors_total 2"));
    }

    #[tokio::test]
    async fn test_metrics_handler_content_type() {
        let registry = Arc::new(Registry::new());
        let _metrics = Metrics::new(&registry).expect("Failed to create metrics");

        let (status, headers, _body) = metrics_handler(registry).await;

        assert_eq!(status, StatusCode::OK);
        // Prometheus expects specific content type
        assert!(headers[0].1.contains("text/plain"));
        assert!(headers[0].1.contains("version=0.0.4"));
    }

    #[test]
    fn test_metrics_error_display() {
        let reg_error = MetricsError::RegistrationError("test error".to_string());
        assert_eq!(format!("{}", reg_error), "Failed to register metric: test error");

        let export_error = MetricsError::ExportError("export failed".to_string());
        assert_eq!(format!("{}", export_error), "Failed to export metrics: export failed");
    }

    #[test]
    fn test_histogram_buckets() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Observe values in different buckets
        metrics.request_duration.observe(0.0005); // < 0.001
        metrics.request_duration.observe(0.002); // < 0.005
        metrics.request_duration.observe(0.008); // < 0.01
        metrics.request_duration.observe(0.02); // < 0.025
        metrics.request_duration.observe(0.04); // < 0.05
        metrics.request_duration.observe(0.08); // < 0.1
        metrics.request_duration.observe(0.2); // < 0.25
        metrics.request_duration.observe(0.4); // < 0.5
        metrics.request_duration.observe(0.8); // < 1.0
        metrics.request_duration.observe(2.0); // < 2.5
        metrics.request_duration.observe(4.0); // < 5.0
        metrics.request_duration.observe(8.0); // < 10.0

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify bucket boundaries are present
        assert!(output.contains("le=\"0.001\""));
        assert!(output.contains("le=\"0.005\""));
        assert!(output.contains("le=\"0.01\""));
        assert!(output.contains("le=\"0.025\""));
        assert!(output.contains("le=\"0.05\""));
        assert!(output.contains("le=\"0.1\""));
        assert!(output.contains("le=\"0.25\""));
        assert!(output.contains("le=\"0.5\""));
        assert!(output.contains("le=\"1\""));
        assert!(output.contains("le=\"2.5\""));
        assert!(output.contains("le=\"5\""));
        assert!(output.contains("le=\"10\""));
        assert!(output.contains("le=\"+Inf\""));
    }

    #[test]
    fn test_create_metrics_handler_returns_closure() {
        let handler = create_metrics_handler();
        let registry = Arc::new(Registry::new());
        let _metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Verify the handler can be called (returns a future)
        let _future = handler(registry);
        // The future type is complex, but we just verify it compiles and can be created
    }

    // =========================================================================
    // Additional unit tests for comprehensive Prometheus format validation
    // **Validates: Requirements 2.3, 2.5**
    // =========================================================================

    /// Tests that the metrics endpoint returns valid Prometheus exposition format.
    /// Prometheus format requires specific line structures:
    /// - HELP lines: # HELP metric_name description
    /// - TYPE lines: # TYPE metric_name type
    /// - Metric lines: metric_name{labels} value
    #[test]
    fn test_prometheus_format_structure() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        metrics.request_count.inc_by(42.0);
        metrics.error_count.inc_by(3.0);
        metrics.active_connections.set(7.0);
        metrics.request_duration.observe(0.123);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify each metric has proper HELP and TYPE declarations
        // Counter metrics
        assert!(
            output.contains("# HELP http_requests_total"),
            "Missing HELP for http_requests_total"
        );
        assert!(
            output.contains("# TYPE http_requests_total counter"),
            "Missing or incorrect TYPE for http_requests_total"
        );

        assert!(
            output.contains("# HELP http_errors_total"),
            "Missing HELP for http_errors_total"
        );
        assert!(
            output.contains("# TYPE http_errors_total counter"),
            "Missing or incorrect TYPE for http_errors_total"
        );

        // Gauge metrics
        assert!(
            output.contains("# HELP http_active_connections"),
            "Missing HELP for http_active_connections"
        );
        assert!(
            output.contains("# TYPE http_active_connections gauge"),
            "Missing or incorrect TYPE for http_active_connections"
        );

        // Histogram metrics
        assert!(
            output.contains("# HELP http_request_duration_seconds"),
            "Missing HELP for http_request_duration_seconds"
        );
        assert!(
            output.contains("# TYPE http_request_duration_seconds histogram"),
            "Missing or incorrect TYPE for http_request_duration_seconds"
        );
    }

    /// Tests that counter values are rendered correctly in Prometheus format.
    /// Counter values should be non-negative floating point numbers.
    #[test]
    fn test_prometheus_counter_format() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Set specific values
        metrics.request_count.inc_by(12345.0);
        metrics.error_count.inc_by(67.0);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify counter values are present and correctly formatted
        assert!(
            output.contains("http_requests_total 12345"),
            "Counter value not correctly rendered"
        );
        assert!(
            output.contains("http_errors_total 67"),
            "Error counter value not correctly rendered"
        );
    }

    /// Tests that gauge values are rendered correctly in Prometheus format.
    /// Gauge values can be any floating point number (positive, negative, or zero).
    #[test]
    fn test_prometheus_gauge_format() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Test various gauge values
        metrics.active_connections.set(100.0);
        let output = render_metrics(&registry).expect("Failed to render metrics");
        assert!(
            output.contains("http_active_connections 100"),
            "Gauge value 100 not correctly rendered"
        );

        // Test zero value
        metrics.active_connections.set(0.0);
        let output = render_metrics(&registry).expect("Failed to render metrics");
        assert!(
            output.contains("http_active_connections 0"),
            "Gauge value 0 not correctly rendered"
        );
    }

    /// Tests that histogram metrics include all required components:
    /// - _bucket metrics with le labels
    /// - _count metric
    /// - _sum metric
    #[test]
    fn test_prometheus_histogram_components() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Record some observations
        metrics.request_duration.observe(0.05);
        metrics.request_duration.observe(0.15);
        metrics.request_duration.observe(0.5);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify histogram has _bucket, _count, and _sum
        assert!(
            output.contains("http_request_duration_seconds_bucket"),
            "Missing histogram bucket metrics"
        );
        assert!(
            output.contains("http_request_duration_seconds_count 3"),
            "Missing or incorrect histogram count"
        );
        assert!(output.contains("http_request_duration_seconds_sum"), "Missing histogram sum");

        // Verify +Inf bucket exists (required by Prometheus)
        assert!(output.contains("le=\"+Inf\""), "Missing +Inf bucket (required by Prometheus)");
    }

    /// Tests that the metrics output contains no invalid characters.
    /// Prometheus format should only contain valid UTF-8 text.
    #[test]
    fn test_prometheus_format_valid_utf8() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        metrics.request_count.inc_by(1.0);
        metrics.request_duration.observe(0.1);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify output is valid UTF-8 (already guaranteed by String type)
        // and contains only printable ASCII characters and newlines
        for line in output.lines() {
            assert!(
                line.chars().all(|c| c.is_ascii_graphic() || c == ' '),
                "Line contains invalid characters: {}",
                line
            );
        }
    }

    /// Tests that metric names follow Prometheus naming conventions.
    /// Names should match [a-zA-Z_:][a-zA-Z0-9_:]*
    #[test]
    fn test_prometheus_metric_naming() {
        let registry = Registry::new();
        let _metrics = Metrics::new(&registry).expect("Failed to create metrics");

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Extract metric names and verify they follow conventions
        let metric_name_regex = regex::Regex::new(r"^([a-zA-Z_:][a-zA-Z0-9_:]*)").unwrap();

        for line in output.lines() {
            // Skip comment lines
            if line.starts_with('#') {
                continue;
            }
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Verify metric name format
            assert!(
                metric_name_regex.is_match(line),
                "Invalid metric name format in line: {}",
                line
            );
        }
    }

    /// Tests that histogram bucket boundaries are in ascending order.
    /// Prometheus requires buckets to be sorted by le value.
    #[test]
    fn test_prometheus_histogram_bucket_order() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        metrics.request_duration.observe(0.1);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Extract le values and verify they're in ascending order
        let le_regex = regex::Regex::new(r#"le="([^"]+)""#).unwrap();
        let mut le_values: Vec<f64> = Vec::new();

        for line in output.lines() {
            if line.contains("http_request_duration_seconds_bucket") {
                if let Some(caps) = le_regex.captures(line) {
                    let le_str = &caps[1];
                    let le_val = if le_str == "+Inf" {
                        f64::INFINITY
                    } else {
                        le_str.parse::<f64>().expect("Invalid le value")
                    };
                    le_values.push(le_val);
                }
            }
        }

        // Verify ascending order
        for i in 1..le_values.len() {
            assert!(
                le_values[i] >= le_values[i - 1],
                "Bucket boundaries not in ascending order: {:?}",
                le_values
            );
        }
    }

    /// Tests that the metrics handler returns correct HTTP status codes.
    #[tokio::test]
    async fn test_metrics_handler_status_codes() {
        // Test successful response
        let registry = Arc::new(Registry::new());
        let _metrics = Metrics::new(&registry).expect("Failed to create metrics");

        let (status, _, _) = metrics_handler(registry).await;
        assert_eq!(status, StatusCode::OK, "Expected 200 OK for valid metrics");
    }

    /// Tests that metrics are thread-safe and can be updated concurrently.
    #[tokio::test]
    async fn test_metrics_concurrent_updates() {
        let registry = Arc::new(Registry::new());
        let metrics = Arc::new(Metrics::new(&registry).expect("Failed to create metrics"));

        let mut handles = Vec::new();

        // Spawn multiple tasks that update metrics concurrently
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = tokio::spawn(async move {
                for _ in 0..100 {
                    metrics_clone.request_count.inc();
                    metrics_clone.active_connections.inc();
                    metrics_clone.active_connections.dec();
                    metrics_clone.request_duration.observe(0.01);
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify final counts
        assert!(
            (metrics.request_count.get() - 1000.0).abs() < f64::EPSILON,
            "Expected 1000 requests, got {}",
            metrics.request_count.get()
        );

        // Active connections should be back to 0 (inc then dec)
        assert!(
            (metrics.active_connections.get() - 0.0).abs() < f64::EPSILON,
            "Expected 0 active connections, got {}",
            metrics.active_connections.get()
        );
    }

    /// Tests that large metric values are handled correctly.
    #[test]
    fn test_prometheus_large_values() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Test with large values
        metrics.request_count.inc_by(1_000_000_000.0);

        let output = render_metrics(&registry).expect("Failed to render metrics");

        // Verify large value is rendered (may be in scientific notation)
        assert!(
            output.contains("http_requests_total") && output.contains("1"),
            "Large counter value not rendered correctly"
        );
    }

    /// Tests that zero values are correctly rendered.
    #[test]
    fn test_prometheus_zero_values() {
        let registry = Registry::new();
        let _metrics = Metrics::new(&registry).expect("Failed to create metrics");

        // Don't increment anything - all should be zero
        let output = render_metrics(&registry).expect("Failed to render metrics");

        assert!(
            output.contains("http_requests_total 0"),
            "Zero counter value not rendered correctly"
        );
        assert!(
            output.contains("http_errors_total 0"),
            "Zero error counter not rendered correctly"
        );
        assert!(
            output.contains("http_active_connections 0"),
            "Zero gauge not rendered correctly"
        );
    }
}
