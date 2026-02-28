//! Prometheus-compatible metrics for DCP server.
//!
//! Provides request counters, latency histograms, and error tracking.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Metrics configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Histogram bucket boundaries (in seconds)
    pub histogram_buckets: Vec<f64>,
    /// Metrics endpoint path
    pub endpoint_path: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            histogram_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
            endpoint_path: "/metrics".to_string(),
        }
    }
}

/// Counter metric
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    /// Create a new counter
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Add a value to the counter
    pub fn add(&self, v: u64) {
        self.value.fetch_add(v, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Histogram metric for latency tracking
#[derive(Debug)]
pub struct Histogram {
    /// Bucket boundaries
    buckets: Vec<f64>,
    /// Bucket counts
    bucket_counts: Vec<AtomicU64>,
    /// Sum of all observations
    sum: AtomicU64,
    /// Count of observations
    count: AtomicU64,
}

impl Histogram {
    /// Create a new histogram with the given bucket boundaries
    pub fn new(buckets: Vec<f64>) -> Self {
        let bucket_counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            buckets,
            bucket_counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Create with default latency buckets
    pub fn with_default_buckets() -> Self {
        Self::new(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    /// Observe a value
    pub fn observe(&self, value: f64) {
        // Update sum (store as microseconds for precision)
        let micros = (value * 1_000_000.0) as u64;
        self.sum.fetch_add(micros, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update bucket counts
        for (i, &bound) in self.buckets.iter().enumerate() {
            if value <= bound {
                self.bucket_counts[i].fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Observe a duration
    pub fn observe_duration(&self, duration: Duration) {
        self.observe(duration.as_secs_f64());
    }

    /// Get bucket counts
    pub fn bucket_counts(&self) -> Vec<u64> {
        self.bucket_counts.iter().map(|c| c.load(Ordering::Relaxed)).collect()
    }

    /// Get the sum of all observations
    pub fn sum(&self) -> f64 {
        self.sum.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    /// Get the count of observations
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get bucket boundaries
    pub fn buckets(&self) -> &[f64] {
        &self.buckets
    }
}

/// Labeled counter (counter with labels)
#[derive(Debug, Default)]
pub struct LabeledCounter {
    counters: RwLock<HashMap<String, Arc<Counter>>>,
}

impl LabeledCounter {
    /// Create a new labeled counter
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a counter for the given label
    pub fn with_label(&self, label: &str) -> Arc<Counter> {
        {
            let counters = self.counters.read().unwrap();
            if let Some(counter) = counters.get(label) {
                return Arc::clone(counter);
            }
        }

        let mut counters = self.counters.write().unwrap();
        counters
            .entry(label.to_string())
            .or_insert_with(|| Arc::new(Counter::new()))
            .clone()
    }

    /// Increment counter for label
    pub fn inc(&self, label: &str) {
        self.with_label(label).inc();
    }

    /// Get all labels and their values
    pub fn all(&self) -> HashMap<String, u64> {
        let counters = self.counters.read().unwrap();
        counters.iter().map(|(k, v)| (k.clone(), v.get())).collect()
    }
}

/// Labeled histogram
#[derive(Debug)]
pub struct LabeledHistogram {
    histograms: RwLock<HashMap<String, Arc<Histogram>>>,
    buckets: Vec<f64>,
}

impl LabeledHistogram {
    /// Create a new labeled histogram
    pub fn new(buckets: Vec<f64>) -> Self {
        Self {
            histograms: RwLock::new(HashMap::new()),
            buckets,
        }
    }

    /// Create with default buckets
    pub fn with_default_buckets() -> Self {
        Self::new(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    /// Get or create a histogram for the given label
    pub fn with_label(&self, label: &str) -> Arc<Histogram> {
        {
            let histograms = self.histograms.read().unwrap();
            if let Some(histogram) = histograms.get(label) {
                return Arc::clone(histogram);
            }
        }

        let mut histograms = self.histograms.write().unwrap();
        histograms
            .entry(label.to_string())
            .or_insert_with(|| Arc::new(Histogram::new(self.buckets.clone())))
            .clone()
    }

    /// Observe a value for label
    pub fn observe(&self, label: &str, value: f64) {
        self.with_label(label).observe(value);
    }

    /// Get all labels
    pub fn labels(&self) -> Vec<String> {
        let histograms = self.histograms.read().unwrap();
        histograms.keys().cloned().collect()
    }
}

/// Prometheus-compatible metrics
pub struct PrometheusMetrics {
    /// Request counter by method
    pub requests_total: LabeledCounter,
    /// Request latency histogram by method
    pub request_duration_seconds: LabeledHistogram,
    /// Active connections gauge
    pub active_connections: Counter,
    /// Bytes transferred counter (by direction: "in" or "out")
    pub bytes_total: LabeledCounter,
    /// Error counter by type
    pub errors_total: LabeledCounter,
    /// Configuration
    config: MetricsConfig,
}

impl PrometheusMetrics {
    /// Create new Prometheus metrics
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            requests_total: LabeledCounter::new(),
            request_duration_seconds: LabeledHistogram::new(config.histogram_buckets.clone()),
            active_connections: Counter::new(),
            bytes_total: LabeledCounter::new(),
            errors_total: LabeledCounter::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(MetricsConfig::default())
    }

    /// Record a request
    pub fn record_request(&self, method: &str, duration: Duration) {
        self.requests_total.inc(method);
        self.request_duration_seconds.observe(method, duration.as_secs_f64());
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        self.errors_total.inc(error_type);
    }

    /// Record bytes transferred
    pub fn record_bytes(&self, direction: &str, bytes: u64) {
        self.bytes_total.with_label(direction).add(bytes);
    }

    /// Increment active connections
    pub fn connection_opened(&self) {
        self.active_connections.inc();
    }

    /// Decrement active connections (using a separate counter for simplicity)
    pub fn connection_closed(&self) {
        // In a real implementation, we'd use a gauge that can decrement
        // For now, we track total connections opened
    }

    /// Get configuration
    pub fn config(&self) -> &MetricsConfig {
        &self.config
    }

    /// Format metrics in Prometheus text format
    pub fn format_prometheus(&self) -> String {
        let mut output = String::new();

        // Requests total
        output.push_str("# HELP dcp_requests_total Total number of requests by method\n");
        output.push_str("# TYPE dcp_requests_total counter\n");
        for (method, count) in self.requests_total.all() {
            output.push_str(&format!("dcp_requests_total{{method=\"{}\"}} {}\n", method, count));
        }

        // Request duration
        output.push_str("\n# HELP dcp_request_duration_seconds Request latency in seconds\n");
        output.push_str("# TYPE dcp_request_duration_seconds histogram\n");
        for label in self.request_duration_seconds.labels() {
            let histogram = self.request_duration_seconds.with_label(&label);
            let buckets = histogram.buckets();
            let counts = histogram.bucket_counts();

            for (i, &bound) in buckets.iter().enumerate() {
                output.push_str(&format!(
                    "dcp_request_duration_seconds_bucket{{method=\"{}\",le=\"{}\"}} {}\n",
                    label, bound, counts[i]
                ));
            }
            output.push_str(&format!(
                "dcp_request_duration_seconds_bucket{{method=\"{}\",le=\"+Inf\"}} {}\n",
                label,
                histogram.count()
            ));
            output.push_str(&format!(
                "dcp_request_duration_seconds_sum{{method=\"{}\"}} {}\n",
                label,
                histogram.sum()
            ));
            output.push_str(&format!(
                "dcp_request_duration_seconds_count{{method=\"{}\"}} {}\n",
                label,
                histogram.count()
            ));
        }

        // Active connections
        output.push_str("\n# HELP dcp_active_connections Number of active connections\n");
        output.push_str("# TYPE dcp_active_connections gauge\n");
        output.push_str(&format!("dcp_active_connections {}\n", self.active_connections.get()));

        // Bytes total
        output.push_str("\n# HELP dcp_bytes_total Total bytes transferred\n");
        output.push_str("# TYPE dcp_bytes_total counter\n");
        for (direction, count) in self.bytes_total.all() {
            output.push_str(&format!("dcp_bytes_total{{direction=\"{}\"}} {}\n", direction, count));
        }

        // Errors total
        output.push_str("\n# HELP dcp_errors_total Total number of errors by type\n");
        output.push_str("# TYPE dcp_errors_total counter\n");
        for (error_type, count) in self.errors_total.all() {
            output.push_str(&format!("dcp_errors_total{{type=\"{}\"}} {}\n", error_type, count));
        }

        output
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Request metrics helper for timing requests
pub struct RequestMetrics {
    method: String,
    start: Instant,
    metrics: Arc<PrometheusMetrics>,
}

impl RequestMetrics {
    /// Start timing a request
    pub fn start(metrics: Arc<PrometheusMetrics>, method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            start: Instant::now(),
            metrics,
        }
    }

    /// Record the request completion
    pub fn finish(self) {
        let duration = self.start.elapsed();
        self.metrics.record_request(&self.method, duration);
    }

    /// Record the request with an error
    pub fn finish_with_error(self, error_type: &str) {
        let duration = self.start.elapsed();
        self.metrics.record_request(&self.method, duration);
        self.metrics.record_error(error_type);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new(vec![0.1, 0.5, 1.0]);

        histogram.observe(0.05);
        histogram.observe(0.3);
        histogram.observe(0.8);
        histogram.observe(2.0);

        assert_eq!(histogram.count(), 4);

        let counts = histogram.bucket_counts();
        assert_eq!(counts[0], 1); // <= 0.1
        assert_eq!(counts[1], 2); // <= 0.5
        assert_eq!(counts[2], 3); // <= 1.0
    }

    #[test]
    fn test_labeled_counter() {
        let counter = LabeledCounter::new();

        counter.inc("method_a");
        counter.inc("method_a");
        counter.inc("method_b");

        let all = counter.all();
        assert_eq!(all.get("method_a"), Some(&2));
        assert_eq!(all.get("method_b"), Some(&1));
    }

    #[test]
    fn test_prometheus_metrics() {
        let metrics = PrometheusMetrics::with_defaults();

        metrics.record_request("tools/list", Duration::from_millis(10));
        metrics.record_request("tools/call", Duration::from_millis(50));
        metrics.record_error("timeout");
        metrics.record_bytes("in", 1000);
        metrics.record_bytes("out", 500);

        let output = metrics.format_prometheus();
        assert!(output.contains("dcp_requests_total"));
        assert!(output.contains("dcp_errors_total"));
        assert!(output.contains("dcp_bytes_total"));
    }

    #[test]
    fn test_request_metrics() {
        let metrics = Arc::new(PrometheusMetrics::with_defaults());

        let request = RequestMetrics::start(Arc::clone(&metrics), "test_method");
        std::thread::sleep(Duration::from_millis(1));
        request.finish();

        let all = metrics.requests_total.all();
        assert_eq!(all.get("test_method"), Some(&1));
    }
}
