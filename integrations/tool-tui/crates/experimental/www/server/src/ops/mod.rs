//! Production Operations Module
//!
//! This module provides production-ready operational infrastructure including:
//! - Graceful shutdown handling with configurable timeouts
//! - Health probes (liveness and readiness)
//! - Circuit breaker pattern for resilience
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_server::ops::{GracefulShutdown, ShutdownConfig};
//! use std::time::Duration;
//!
//! let config = ShutdownConfig {
//!     timeout: Duration::from_secs(30),
//! };
//! let shutdown = GracefulShutdown::new(config);
//!
//! // Wait for shutdown signal
//! shutdown.wait_for_signal().await;
//!
//! // Initiate graceful shutdown
//! shutdown.shutdown().await?;
//! ```
//!
//! # Health Probes Example
//!
//! ```rust,ignore
//! use dx_www_server::ops::health::{HealthChecker, liveness_handler, readiness_handler};
//! use axum::{Router, routing::get};
//! use std::sync::Arc;
//!
//! let checker = Arc::new(HealthChecker::with_version("1.0.0"));
//!
//! let app = Router::new()
//!     .route("/health/live", get(liveness_handler))
//!     .route("/health/ready", get(readiness_handler))
//!     .with_state(checker);
//! ```
//!
//! # Circuit Breaker Example
//!
//! ```rust,ignore
//! use dx_www_server::ops::{CircuitBreaker, CircuitBreakerConfig};
//! use std::time::Duration;
//!
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     reset_timeout: Duration::from_secs(30),
//!     half_open_max_calls: 3,
//! };
//!
//! let breaker = CircuitBreaker::new(config);
//!
//! // Wrap external calls with the circuit breaker
//! let result = breaker.call(async {
//!     external_service_call().await
//! }).await;
//! ```

pub mod circuit_breaker;
pub mod health;
pub mod shutdown;

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState,
};
pub use health::{
    AlwaysHealthyCheck, CheckResult, HealthCheck, HealthChecker, HealthState, HealthStatus,
    ManualHealthCheck, liveness_handler, readiness_handler, simple_readiness_handler,
};
pub use shutdown::{GracefulShutdown, ShutdownConfig, ShutdownError};
