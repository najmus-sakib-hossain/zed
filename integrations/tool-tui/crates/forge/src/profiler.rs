//! Performance Profiler for Critical Paths
//!
//! Provides performance profiling, metrics collection, and bottleneck detection.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
}

impl Metric {
    fn new(name: String) -> Self {
        Self {
            name,
            count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
        }
    }

    fn record(&mut self, duration: Duration) {
        self.count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.count as u32;
    }
}

/// Profiler for tracking performance metrics
pub struct Profiler {
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    enabled: bool,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            enabled: true,
        }
    }

    /// Enable or disable profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Start profiling a section
    pub fn start(&self, name: &str) -> ProfileGuard {
        ProfileGuard {
            name: name.to_string(),
            start: Instant::now(),
            profiler: self.metrics.clone(),
            enabled: self.enabled,
        }
    }

    /// Get all metrics
    pub fn get_metrics(&self) -> Vec<Metric> {
        let metrics = self.metrics.read();
        metrics.values().cloned().collect()
    }

    /// Get a specific metric
    pub fn get_metric(&self, name: &str) -> Option<Metric> {
        let metrics = self.metrics.read();
        metrics.get(name).cloned()
    }

    /// Get slowest operations
    pub fn get_slowest(&self, limit: usize) -> Vec<Metric> {
        let metrics = self.metrics.read();
        let mut sorted: Vec<_> = metrics.values().cloned().collect();
        sorted.sort_by(|a, b| b.avg_duration.cmp(&a.avg_duration));
        sorted.truncate(limit);
        sorted
    }

    /// Get hottest operations (most called)
    pub fn get_hottest(&self, limit: usize) -> Vec<Metric> {
        let metrics = self.metrics.read();
        let mut sorted: Vec<_> = metrics.values().cloned().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        sorted.truncate(limit);
        sorted
    }

    /// Reset metrics
    pub fn reset(&self) {
        let mut metrics = self.metrics.write();
        metrics.clear();
    }

    /// Print profiling summary
    pub fn print_summary(&self) {
        let metrics = self.get_metrics();

        if metrics.is_empty() {
            println!("No profiling data collected");
            return;
        }

        println!("\n=== Performance Profile ===\n");
        println!(
            "{:<40} {:>10} {:>15} {:>15} {:>15}",
            "Operation", "Count", "Total (ms)", "Avg (ms)", "Max (ms)"
        );
        println!("{:-<100}", "");

        for metric in metrics {
            println!(
                "{:<40} {:>10} {:>15.2} {:>15.2} {:>15.2}",
                metric.name,
                metric.count,
                metric.total_duration.as_secs_f64() * 1000.0,
                metric.avg_duration.as_secs_f64() * 1000.0,
                metric.max_duration.as_secs_f64() * 1000.0,
            );
        }

        println!("\n== Slowest Operations ==");
        for (i, metric) in self.get_slowest(5).iter().enumerate() {
            println!(
                "{}. {} - {:.2}ms average",
                i + 1,
                metric.name,
                metric.avg_duration.as_secs_f64() * 1000.0
            );
        }

        println!("\n== Hottest Operations ==");
        for (i, metric) in self.get_hottest(5).iter().enumerate() {
            println!("{}. {} - {} calls", i + 1, metric.name, metric.count);
        }
        println!();
    }

    /// Record a manual measurement
    pub fn record(&self, name: &str, duration: Duration) {
        if !self.enabled {
            return;
        }

        let mut metrics = self.metrics.write();
        let metric =
            metrics.entry(name.to_string()).or_insert_with(|| Metric::new(name.to_string()));
        metric.record(duration);
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for automatic profiling
pub struct ProfileGuard {
    name: String,
    start: Instant,
    profiler: Arc<RwLock<HashMap<String, Metric>>>,
    enabled: bool,
}

impl Drop for ProfileGuard {
    fn drop(&mut self) {
        if !self.enabled {
            return;
        }

        let duration = self.start.elapsed();
        let mut metrics = self.profiler.write();
        let metric = metrics
            .entry(self.name.clone())
            .or_insert_with(|| Metric::new(self.name.clone()));
        metric.record(duration);
    }
}

/// Global profiler instance
static GLOBAL_PROFILER: once_cell::sync::Lazy<Profiler> = once_cell::sync::Lazy::new(Profiler::new);

/// Profile a code block
#[allow(unused_macros)]
#[macro_export]
macro_rules! profile {
    ($name:expr) => {
        let _guard = $crate::profiler::GLOBAL_PROFILER.start($name);
    };
}

/// Get the global profiler
pub fn global_profiler() -> &'static Profiler {
    &GLOBAL_PROFILER
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler() {
        let profiler = Profiler::new();

        {
            let _guard = profiler.start("test_operation");
            thread::sleep(Duration::from_millis(10));
        }

        let metrics = profiler.get_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].count, 1);
        assert!(metrics[0].total_duration >= Duration::from_millis(10));
    }

    #[test]
    fn test_multiple_measurements() {
        let profiler = Profiler::new();

        for _ in 0..5 {
            let _guard = profiler.start("repeated_operation");
            thread::sleep(Duration::from_millis(5));
        }

        let metric = profiler.get_metric("repeated_operation").unwrap();
        assert_eq!(metric.count, 5);
    }
}
