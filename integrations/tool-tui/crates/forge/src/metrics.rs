//! Metrics Collector for DX Forge
//!
//! Provides comprehensive metrics collection including:
//! - Atomic counters for operations
//! - Histogram for I/O latency
//! - JSON export for observability
//!
//! # Example
//! ```rust,ignore
//! use dx_forge::metrics::MetricsCollector;
//! use std::time::Duration;
//!
//! let metrics = MetricsCollector::new();
//! metrics.record_io_operation(Duration::from_millis(50), true);
//! metrics.increment_files_watched();
//!
//! let json = metrics.export_json();
//! println!("{}", json);
//! ```

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Metrics collector for Forge operations
pub struct MetricsCollector {
    /// Number of files currently being watched
    files_watched: AtomicU64,
    /// Total number of I/O operations
    operations_total: AtomicU64,
    /// Number of successful operations
    operations_success: AtomicU64,
    /// Number of failed operations
    operations_failed: AtomicU64,
    /// Cache hits
    cache_hits: AtomicU64,
    /// Cache misses
    cache_misses: AtomicU64,
    /// Total errors
    errors_total: AtomicU64,
    /// I/O latency samples (in microseconds)
    io_latency_samples: RwLock<Vec<u64>>,
    /// Maximum samples to keep for histogram
    max_samples: usize,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::with_max_samples(10000)
    }

    /// Create a metrics collector with custom max samples
    pub fn with_max_samples(max_samples: usize) -> Self {
        Self {
            files_watched: AtomicU64::new(0),
            operations_total: AtomicU64::new(0),
            operations_success: AtomicU64::new(0),
            operations_failed: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            io_latency_samples: RwLock::new(Vec::with_capacity(max_samples)),
            max_samples,
        }
    }

    /// Record an I/O operation
    pub fn record_io_operation(&self, duration: Duration, success: bool) {
        self.operations_total.fetch_add(1, Ordering::Relaxed);

        if success {
            self.operations_success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.operations_failed.fetch_add(1, Ordering::Relaxed);
            self.errors_total.fetch_add(1, Ordering::Relaxed);
        }

        // Record latency sample
        let micros = duration.as_micros() as u64;
        let mut samples = self.io_latency_samples.write();
        if samples.len() >= self.max_samples {
            // Remove oldest sample
            samples.remove(0);
        }
        samples.push(micros);
    }

    /// Increment files watched counter
    pub fn increment_files_watched(&self) {
        self.files_watched.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement files watched counter
    pub fn decrement_files_watched(&self) {
        self.files_watched.fetch_sub(1, Ordering::Relaxed);
    }

    /// Set files watched counter
    pub fn set_files_watched(&self, count: u64) {
        self.files_watched.store(count, Ordering::Relaxed);
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Get files watched count
    pub fn files_watched(&self) -> u64 {
        self.files_watched.load(Ordering::Relaxed)
    }

    /// Get total operations count
    pub fn operations_total(&self) -> u64 {
        self.operations_total.load(Ordering::Relaxed)
    }

    /// Get successful operations count
    pub fn operations_success(&self) -> u64 {
        self.operations_success.load(Ordering::Relaxed)
    }

    /// Get failed operations count
    pub fn operations_failed(&self) -> u64 {
        self.operations_failed.load(Ordering::Relaxed)
    }

    /// Get cache hits count
    pub fn cache_hits(&self) -> u64 {
        self.cache_hits.load(Ordering::Relaxed)
    }

    /// Get cache misses count
    pub fn cache_misses(&self) -> u64 {
        self.cache_misses.load(Ordering::Relaxed)
    }

    /// Get total errors count
    pub fn errors_total(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }

    /// Calculate cache hit rate (0.0 to 1.0)
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Calculate percentile of I/O latency (in microseconds)
    pub fn io_latency_percentile(&self, percentile: f64) -> u64 {
        let samples = self.io_latency_samples.read();
        if samples.is_empty() {
            return 0;
        }

        let mut sorted: Vec<u64> = samples.clone();
        sorted.sort_unstable();

        let index = ((percentile / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Get P50 latency (median) in microseconds
    pub fn io_latency_p50(&self) -> u64 {
        self.io_latency_percentile(50.0)
    }

    /// Get P95 latency in microseconds
    pub fn io_latency_p95(&self) -> u64 {
        self.io_latency_percentile(95.0)
    }

    /// Get P99 latency in microseconds
    pub fn io_latency_p99(&self) -> u64 {
        self.io_latency_percentile(99.0)
    }

    /// Get average latency in microseconds
    pub fn io_latency_avg(&self) -> u64 {
        let samples = self.io_latency_samples.read();
        if samples.is_empty() {
            return 0;
        }

        let sum: u64 = samples.iter().sum();
        sum / samples.len() as u64
    }

    /// Export metrics as JSON
    pub fn export_json(&self) -> serde_json::Value {
        serde_json::json!({
            "files_watched": self.files_watched(),
            "operations_total": self.operations_total(),
            "operations_success": self.operations_success(),
            "operations_failed": self.operations_failed(),
            "cache_hits": self.cache_hits(),
            "cache_misses": self.cache_misses(),
            "cache_hit_rate": self.cache_hit_rate(),
            "errors_total": self.errors_total(),
            "io_latency_p50_us": self.io_latency_p50(),
            "io_latency_p95_us": self.io_latency_p95(),
            "io_latency_p99_us": self.io_latency_p99(),
            "io_latency_avg_us": self.io_latency_avg(),
        })
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.files_watched.store(0, Ordering::Relaxed);
        self.operations_total.store(0, Ordering::Relaxed);
        self.operations_success.store(0, Ordering::Relaxed);
        self.operations_failed.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.errors_total.store(0, Ordering::Relaxed);
        self.io_latency_samples.write().clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_counters() {
        let metrics = MetricsCollector::new();

        assert_eq!(metrics.files_watched(), 0);
        assert_eq!(metrics.operations_total(), 0);

        metrics.increment_files_watched();
        metrics.increment_files_watched();
        assert_eq!(metrics.files_watched(), 2);

        metrics.decrement_files_watched();
        assert_eq!(metrics.files_watched(), 1);
    }

    #[test]
    fn test_io_operations() {
        let metrics = MetricsCollector::new();

        metrics.record_io_operation(Duration::from_millis(10), true);
        metrics.record_io_operation(Duration::from_millis(20), true);
        metrics.record_io_operation(Duration::from_millis(30), false);

        assert_eq!(metrics.operations_total(), 3);
        assert_eq!(metrics.operations_success(), 2);
        assert_eq!(metrics.operations_failed(), 1);
        assert_eq!(metrics.errors_total(), 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let metrics = MetricsCollector::new();

        // No operations yet
        assert_eq!(metrics.cache_hit_rate(), 0.0);

        // 3 hits, 1 miss = 75% hit rate
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        assert!((metrics.cache_hit_rate() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_latency_percentiles() {
        let metrics = MetricsCollector::new();

        // Add samples: 10, 20, 30, 40, 50, 60, 70, 80, 90, 100
        for i in 1..=10 {
            metrics.record_io_operation(Duration::from_micros(i * 10), true);
        }

        // P50 should be around 50-60
        let p50 = metrics.io_latency_p50();
        assert!(p50 >= 50 && p50 <= 60, "P50 was {}", p50);

        // P99 should be close to 100
        let p99 = metrics.io_latency_p99();
        assert!(p99 >= 90, "P99 was {}", p99);
    }

    #[test]
    fn test_export_json() {
        let metrics = MetricsCollector::new();

        metrics.set_files_watched(10);
        metrics.record_io_operation(Duration::from_millis(5), true);
        metrics.record_cache_hit();

        let json = metrics.export_json();

        assert_eq!(json["files_watched"], 10);
        assert_eq!(json["operations_total"], 1);
        assert_eq!(json["cache_hits"], 1);
    }

    #[test]
    fn test_reset() {
        let metrics = MetricsCollector::new();

        metrics.set_files_watched(100);
        metrics.record_io_operation(Duration::from_millis(10), true);
        metrics.record_cache_hit();

        metrics.reset();

        assert_eq!(metrics.files_watched(), 0);
        assert_eq!(metrics.operations_total(), 0);
        assert_eq!(metrics.cache_hits(), 0);
    }
}

/// Property-based tests for metrics
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 18: Metrics Availability
        /// For any running Forge instance, the metrics endpoint SHALL return
        /// valid values for: files_watched, operations_total, cache_hit_rate,
        /// and errors_total.
        #[test]
        fn prop_metrics_availability(
            files_watched in 0u64..10000u64,
            success_ops in 0usize..100usize,
            failed_ops in 0usize..100usize,
            cache_hits in 0usize..100usize,
            cache_misses in 0usize..100usize,
        ) {
            let metrics = MetricsCollector::new();

            // Set up metrics
            metrics.set_files_watched(files_watched);

            for _ in 0..success_ops {
                metrics.record_io_operation(Duration::from_millis(10), true);
            }

            for _ in 0..failed_ops {
                metrics.record_io_operation(Duration::from_millis(10), false);
            }

            for _ in 0..cache_hits {
                metrics.record_cache_hit();
            }

            for _ in 0..cache_misses {
                metrics.record_cache_miss();
            }

            // Export JSON
            let json = metrics.export_json();

            // Verify all required fields are present and valid
            prop_assert!(json.get("files_watched").is_some(),
                "files_watched should be present");
            prop_assert!(json.get("operations_total").is_some(),
                "operations_total should be present");
            prop_assert!(json.get("cache_hit_rate").is_some(),
                "cache_hit_rate should be present");
            prop_assert!(json.get("errors_total").is_some(),
                "errors_total should be present");

            // Verify values are correct
            prop_assert_eq!(json["files_watched"].as_u64().unwrap(), files_watched);
            prop_assert_eq!(
                json["operations_total"].as_u64().unwrap(),
                (success_ops + failed_ops) as u64
            );
            prop_assert_eq!(json["errors_total"].as_u64().unwrap(), failed_ops as u64);

            // Verify cache hit rate is valid (0.0 to 1.0)
            let hit_rate = json["cache_hit_rate"].as_f64().unwrap();
            prop_assert!(hit_rate >= 0.0 && hit_rate <= 1.0,
                "Cache hit rate should be between 0 and 1, was {}", hit_rate);

            // Verify latency percentiles are present
            prop_assert!(json.get("io_latency_p50_us").is_some());
            prop_assert!(json.get("io_latency_p95_us").is_some());
            prop_assert!(json.get("io_latency_p99_us").is_some());
        }

        /// Property 18 (continued): Metrics are consistent
        #[test]
        fn prop_metrics_consistency(
            ops in prop::collection::vec((1u64..1000u64, any::<bool>()), 1..100),
        ) {
            let metrics = MetricsCollector::new();

            let mut expected_total = 0u64;
            let mut expected_success = 0u64;
            let mut expected_failed = 0u64;

            for (duration_us, success) in &ops {
                metrics.record_io_operation(Duration::from_micros(*duration_us), *success);
                expected_total += 1;
                if *success {
                    expected_success += 1;
                } else {
                    expected_failed += 1;
                }
            }

            // Verify consistency
            prop_assert_eq!(metrics.operations_total(), expected_total);
            prop_assert_eq!(metrics.operations_success(), expected_success);
            prop_assert_eq!(metrics.operations_failed(), expected_failed);
            prop_assert_eq!(metrics.errors_total(), expected_failed);

            // Total should equal success + failed
            prop_assert_eq!(
                metrics.operations_total(),
                metrics.operations_success() + metrics.operations_failed()
            );
        }

        /// Property 18 (continued): Latency percentiles are ordered
        #[test]
        fn prop_latency_percentiles_ordered(
            latencies in prop::collection::vec(1u64..100000u64, 10..100),
        ) {
            let metrics = MetricsCollector::new();

            for latency in &latencies {
                metrics.record_io_operation(Duration::from_micros(*latency), true);
            }

            let p50 = metrics.io_latency_p50();
            let p95 = metrics.io_latency_p95();
            let p99 = metrics.io_latency_p99();

            // Percentiles should be ordered: P50 <= P95 <= P99
            prop_assert!(p50 <= p95,
                "P50 ({}) should be <= P95 ({})", p50, p95);
            prop_assert!(p95 <= p99,
                "P95 ({}) should be <= P99 ({})", p95, p99);

            // All percentiles should be within the range of samples
            let min = *latencies.iter().min().unwrap();
            let max = *latencies.iter().max().unwrap();

            prop_assert!(p50 >= min && p50 <= max,
                "P50 ({}) should be within sample range ({}-{})", p50, min, max);
            prop_assert!(p99 >= min && p99 <= max,
                "P99 ({}) should be within sample range ({}-{})", p99, min, max);
        }

        /// Property 18 (continued): Cache hit rate calculation
        #[test]
        fn prop_cache_hit_rate_calculation(
            hits in 0u64..1000u64,
            misses in 0u64..1000u64,
        ) {
            let metrics = MetricsCollector::new();

            for _ in 0..hits {
                metrics.record_cache_hit();
            }
            for _ in 0..misses {
                metrics.record_cache_miss();
            }

            let rate = metrics.cache_hit_rate();

            if hits + misses == 0 {
                prop_assert_eq!(rate, 0.0,
                    "Rate should be 0 when no cache operations");
            } else {
                let expected = hits as f64 / (hits + misses) as f64;
                prop_assert!((rate - expected).abs() < 0.0001,
                    "Rate {} should equal expected {}", rate, expected);
            }
        }
    }
}
