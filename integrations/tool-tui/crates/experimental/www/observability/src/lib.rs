//! Observability infrastructure for dx-www.
//!
//! This crate provides comprehensive observability capabilities including:
//! - Distributed tracing with OpenTelemetry integration
//! - Prometheus-compatible metrics
//! - Structured JSON logging with trace correlation
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_observability::{ObservabilityConfig, init_observability};
//!
//! let config = ObservabilityConfig::default();
//! init_observability(&config).expect("Failed to initialize observability");
//! ```

pub mod logging;
pub mod metrics;
pub mod tracing;

use serde::{Deserialize, Serialize};

/// Configuration for the observability stack.
///
/// This struct contains all configuration options for tracing, metrics, and logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// OTLP endpoint for exporting traces (e.g., `http://localhost:4317`).
    /// If `None`, tracing will be disabled.
    pub otlp_endpoint: Option<String>,

    /// Port for exposing Prometheus metrics endpoint.
    pub metrics_port: u16,

    /// Sampling rate for traces (0.0 - 1.0).
    /// A value of 1.0 means all traces are sampled, 0.0 means none.
    pub sampling_rate: f64,

    /// Service name used for telemetry identification.
    pub service_name: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            metrics_port: 9090,
            sampling_rate: 0.1,
            service_name: "dx-www".to_string(),
        }
    }
}

impl ObservabilityConfig {
    /// Creates a new `ObservabilityConfig` with the specified service name.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Sets the OTLP endpoint for trace export.
    #[must_use]
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Sets the metrics port.
    #[must_use]
    pub const fn with_metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = port;
        self
    }

    /// Sets the sampling rate for traces.
    ///
    /// # Panics
    ///
    /// Panics if the sampling rate is not in the range [0.0, 1.0].
    #[must_use]
    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        assert!((0.0..=1.0).contains(&rate), "Sampling rate must be between 0.0 and 1.0");
        self.sampling_rate = rate;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.otlp_endpoint, None);
        assert_eq!(config.metrics_port, 9090);
        assert!((config.sampling_rate - 0.1).abs() < f64::EPSILON);
        assert_eq!(config.service_name, "dx-www");
    }

    #[test]
    fn test_config_builder() {
        let config = ObservabilityConfig::new("test-service")
            .with_otlp_endpoint("http://localhost:4317")
            .with_metrics_port(9091)
            .with_sampling_rate(0.5);

        assert_eq!(config.otlp_endpoint, Some("http://localhost:4317".to_string()));
        assert_eq!(config.metrics_port, 9091);
        assert!((config.sampling_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.service_name, "test-service");
    }

    #[test]
    #[should_panic(expected = "Sampling rate must be between 0.0 and 1.0")]
    fn test_invalid_sampling_rate_high() {
        let _ = ObservabilityConfig::default().with_sampling_rate(1.5);
    }

    #[test]
    #[should_panic(expected = "Sampling rate must be between 0.0 and 1.0")]
    fn test_invalid_sampling_rate_negative() {
        let _ = ObservabilityConfig::default().with_sampling_rate(-0.1);
    }
}
