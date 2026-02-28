//! Health Probe Endpoints
//!
//! Provides health check endpoints for Kubernetes-style liveness and readiness probes.
//!
//! # Overview
//!
//! This module implements two types of health probes:
//!
//! - **Liveness Probe** (`/health/live`): Indicates if the process is alive and running.
//!   Returns 200 OK if the process is responsive. Used by orchestrators to determine
//!   if the process needs to be restarted.
//!
//! - **Readiness Probe** (`/health/ready`): Indicates if the service is ready to accept
//!   traffic. Checks all registered health checks (database connections, external services,
//!   etc.) and returns 200 OK only if all checks pass. Returns 503 Service Unavailable
//!   if any check fails.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_server::ops::health::{HealthChecker, HealthCheck, CheckResult, HealthState};
//! use std::sync::Arc;
//!
//! // Create a custom health check
//! struct DatabaseHealthCheck {
//!     pool: Arc<DatabasePool>,
//! }
//!
//! #[async_trait::async_trait]
//! impl HealthCheck for DatabaseHealthCheck {
//!     fn name(&self) -> &str {
//!         "database"
//!     }
//!
//!     async fn check(&self) -> CheckResult {
//!         match self.pool.ping().await {
//!             Ok(_) => CheckResult::healthy("database"),
//!             Err(e) => CheckResult::unhealthy("database", e.to_string()),
//!         }
//!     }
//! }
//!
//! // Create health checker with custom checks
//! let mut checker = HealthChecker::new();
//! checker.add_check(Box::new(DatabaseHealthCheck { pool }));
//!
//! // Use in router
//! let app = Router::new()
//!     .route("/health/live", get(liveness_handler))
//!     .route("/health/ready", get(readiness_handler))
//!     .with_state(Arc::new(checker));
//! ```

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Health state indicating the overall health of a component or the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    /// The component is fully operational.
    Healthy,
    /// The component is operational but experiencing issues.
    Degraded,
    /// The component is not operational.
    Unhealthy,
}

impl HealthState {
    /// Returns true if the state is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthState::Healthy)
    }

    /// Returns true if the state is at least degraded (not unhealthy).
    pub fn is_operational(&self) -> bool {
        !matches!(self, HealthState::Unhealthy)
    }
}

/// Result of a single health check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Name of the health check.
    pub name: String,
    /// Current health state.
    pub status: HealthState,
    /// Optional message providing details about the check result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Duration of the health check in milliseconds.
    pub duration_ms: u64,
}

impl CheckResult {
    /// Create a healthy check result.
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Healthy,
            message: None,
            duration_ms: 0,
        }
    }

    /// Create a healthy check result with a message.
    pub fn healthy_with_message(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Healthy,
            message: Some(message.into()),
            duration_ms: 0,
        }
    }

    /// Create a degraded check result.
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Degraded,
            message: Some(message.into()),
            duration_ms: 0,
        }
    }

    /// Create an unhealthy check result.
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Unhealthy,
            message: Some(message.into()),
            duration_ms: 0,
        }
    }

    /// Set the duration of the health check.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = duration.as_millis() as u64;
        self
    }
}

/// Overall health status of the service.
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    /// Overall health state.
    pub status: HealthState,
    /// Individual check results.
    pub checks: HashMap<String, CheckResult>,
    /// Service version (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Uptime in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
}

impl HealthStatus {
    /// Create a new health status with the given state.
    pub fn new(status: HealthState) -> Self {
        Self {
            status,
            checks: HashMap::new(),
            version: None,
            uptime_seconds: None,
        }
    }

    /// Create a healthy status with no checks.
    pub fn healthy() -> Self {
        Self::new(HealthState::Healthy)
    }

    /// Add a check result to the status.
    pub fn add_check(&mut self, result: CheckResult) {
        self.checks.insert(result.name.clone(), result);
    }

    /// Set the service version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the uptime in seconds.
    pub fn with_uptime(mut self, uptime_seconds: u64) -> Self {
        self.uptime_seconds = Some(uptime_seconds);
        self
    }

    /// Returns true if the overall status is healthy.
    pub fn is_healthy(&self) -> bool {
        self.status.is_healthy()
    }

    /// Compute the overall status from individual check results.
    pub fn compute_status(&mut self) {
        if self.checks.is_empty() {
            self.status = HealthState::Healthy;
            return;
        }

        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for check in self.checks.values() {
            match check.status {
                HealthState::Unhealthy => has_unhealthy = true,
                HealthState::Degraded => has_degraded = true,
                HealthState::Healthy => {}
            }
        }

        self.status = if has_unhealthy {
            HealthState::Unhealthy
        } else if has_degraded {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };
    }
}

/// Trait for implementing health checks.
///
/// Implement this trait to create custom health checks for your services.
///
/// # Example
///
/// ```rust,ignore
/// use dx_www_server::ops::health::{HealthCheck, CheckResult};
///
/// struct RedisHealthCheck {
///     client: redis::Client,
/// }
///
/// #[async_trait::async_trait]
/// impl HealthCheck for RedisHealthCheck {
///     fn name(&self) -> &str {
///         "redis"
///     }
///
///     async fn check(&self) -> CheckResult {
///         match self.client.ping().await {
///             Ok(_) => CheckResult::healthy("redis"),
///             Err(e) => CheckResult::unhealthy("redis", e.to_string()),
///         }
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Returns the name of this health check.
    fn name(&self) -> &str;

    /// Perform the health check and return the result.
    async fn check(&self) -> CheckResult;
}

/// Health checker that aggregates multiple health checks.
///
/// The `HealthChecker` manages a collection of health checks and provides
/// methods to run all checks and aggregate the results.
pub struct HealthChecker {
    /// Registered health checks.
    checks: Vec<Box<dyn HealthCheck>>,
    /// Service version for reporting.
    version: Option<String>,
    /// Start time for uptime calculation.
    start_time: Instant,
}

impl HealthChecker {
    /// Create a new health checker with no checks.
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            version: None,
            start_time: Instant::now(),
        }
    }

    /// Create a new health checker with the specified version.
    pub fn with_version(version: impl Into<String>) -> Self {
        Self {
            checks: Vec::new(),
            version: Some(version.into()),
            start_time: Instant::now(),
        }
    }

    /// Add a health check to the checker.
    pub fn add_check(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }

    /// Add a health check to the checker (builder pattern).
    pub fn with_check(mut self, check: Box<dyn HealthCheck>) -> Self {
        self.checks.push(check);
        self
    }

    /// Get the number of registered health checks.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Get the uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Run all health checks and return the aggregated status.
    pub async fn check_all(&self) -> HealthStatus {
        let mut status = HealthStatus::new(HealthState::Healthy);

        // Run all checks concurrently
        let check_futures: Vec<_> = self
            .checks
            .iter()
            .map(|check| async move {
                let start = Instant::now();
                let mut result = check.check().await;
                result.duration_ms = start.elapsed().as_millis() as u64;
                result
            })
            .collect();

        let results = futures::future::join_all(check_futures).await;

        for result in results {
            status.add_check(result);
        }

        // Compute overall status from individual checks
        status.compute_status();

        // Add metadata
        if let Some(ref version) = self.version {
            status.version = Some(version.clone());
        }
        status.uptime_seconds = Some(self.uptime_seconds());

        status
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Liveness probe handler - indicates if the process is alive.
///
/// This endpoint always returns 200 OK with a simple JSON response
/// as long as the process is running and can handle HTTP requests.
///
/// # Response
///
/// ```json
/// { "status": "ok" }
/// ```
///
/// # Usage
///
/// ```rust,ignore
/// use axum::{Router, routing::get};
/// use dx_www_server::ops::health::liveness_handler;
///
/// let app = Router::new()
///     .route("/health/live", get(liveness_handler));
/// ```
pub async fn liveness_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Readiness probe handler - indicates if the service is ready to accept traffic.
///
/// This endpoint runs all registered health checks and returns:
/// - 200 OK if all checks pass (status is healthy or degraded)
/// - 503 Service Unavailable if any check fails (status is unhealthy)
///
/// # Response
///
/// ```json
/// {
///   "status": "healthy",
///   "checks": {
///     "database": {
///       "name": "database",
///       "status": "healthy",
///       "duration_ms": 5
///     }
///   },
///   "version": "1.0.0",
///   "uptime_seconds": 3600
/// }
/// ```
///
/// # Usage
///
/// ```rust,ignore
/// use axum::{Router, routing::get};
/// use dx_www_server::ops::health::{readiness_handler, HealthChecker};
/// use std::sync::Arc;
///
/// let checker = Arc::new(HealthChecker::new());
/// let app = Router::new()
///     .route("/health/ready", get(readiness_handler))
///     .with_state(checker);
/// ```
pub async fn readiness_handler(State(checker): State<Arc<HealthChecker>>) -> impl IntoResponse {
    let status = checker.check_all().await;

    // Return 200 if healthy or degraded, 503 if unhealthy
    let status_code = if status.status == HealthState::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (status_code, Json(status))
}

/// Simple readiness handler that doesn't require state.
///
/// Use this when you don't need custom health checks and just want
/// a basic readiness endpoint.
pub async fn simple_readiness_handler() -> impl IntoResponse {
    Json(HealthStatus::healthy())
}

// ============================================================================
// Built-in Health Checks
// ============================================================================

/// A simple health check that always returns healthy.
///
/// Useful for testing or as a placeholder.
pub struct AlwaysHealthyCheck {
    name: String,
}

impl AlwaysHealthyCheck {
    /// Create a new always-healthy check with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl HealthCheck for AlwaysHealthyCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> CheckResult {
        CheckResult::healthy(&self.name)
    }
}

/// A health check that can be manually set to healthy or unhealthy.
///
/// Useful for testing or for implementing circuit breaker patterns.
pub struct ManualHealthCheck {
    name: String,
    healthy: std::sync::atomic::AtomicBool,
    message: std::sync::RwLock<Option<String>>,
}

impl ManualHealthCheck {
    /// Create a new manual health check with the given name, initially healthy.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            healthy: std::sync::atomic::AtomicBool::new(true),
            message: std::sync::RwLock::new(None),
        }
    }

    /// Set the health check to healthy.
    pub fn set_healthy(&self) {
        self.healthy.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut msg) = self.message.write() {
            *msg = None;
        }
    }

    /// Set the health check to unhealthy with a message.
    pub fn set_unhealthy(&self, message: impl Into<String>) {
        self.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut msg) = self.message.write() {
            *msg = Some(message.into());
        }
    }

    /// Check if currently healthy.
    pub fn is_healthy(&self) -> bool {
        self.healthy.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl HealthCheck for ManualHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> CheckResult {
        if self.healthy.load(std::sync::atomic::Ordering::SeqCst) {
            CheckResult::healthy(&self.name)
        } else {
            let message = self
                .message
                .read()
                .ok()
                .and_then(|m| m.clone())
                .unwrap_or_else(|| "unhealthy".to_string());
            CheckResult::unhealthy(&self.name, message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_state_is_healthy() {
        assert!(HealthState::Healthy.is_healthy());
        assert!(!HealthState::Degraded.is_healthy());
        assert!(!HealthState::Unhealthy.is_healthy());
    }

    #[test]
    fn test_health_state_is_operational() {
        assert!(HealthState::Healthy.is_operational());
        assert!(HealthState::Degraded.is_operational());
        assert!(!HealthState::Unhealthy.is_operational());
    }

    #[test]
    fn test_check_result_healthy() {
        let result = CheckResult::healthy("test");
        assert_eq!(result.name, "test");
        assert_eq!(result.status, HealthState::Healthy);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_check_result_unhealthy() {
        let result = CheckResult::unhealthy("test", "connection failed");
        assert_eq!(result.name, "test");
        assert_eq!(result.status, HealthState::Unhealthy);
        assert_eq!(result.message, Some("connection failed".to_string()));
    }

    #[test]
    fn test_check_result_degraded() {
        let result = CheckResult::degraded("test", "high latency");
        assert_eq!(result.name, "test");
        assert_eq!(result.status, HealthState::Degraded);
        assert_eq!(result.message, Some("high latency".to_string()));
    }

    #[test]
    fn test_check_result_with_duration() {
        let result = CheckResult::healthy("test").with_duration(Duration::from_millis(100));
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_health_status_compute_all_healthy() {
        let mut status = HealthStatus::new(HealthState::Unhealthy);
        status.add_check(CheckResult::healthy("check1"));
        status.add_check(CheckResult::healthy("check2"));
        status.compute_status();
        assert_eq!(status.status, HealthState::Healthy);
    }

    #[test]
    fn test_health_status_compute_with_degraded() {
        let mut status = HealthStatus::new(HealthState::Healthy);
        status.add_check(CheckResult::healthy("check1"));
        status.add_check(CheckResult::degraded("check2", "slow"));
        status.compute_status();
        assert_eq!(status.status, HealthState::Degraded);
    }

    #[test]
    fn test_health_status_compute_with_unhealthy() {
        let mut status = HealthStatus::new(HealthState::Healthy);
        status.add_check(CheckResult::healthy("check1"));
        status.add_check(CheckResult::unhealthy("check2", "down"));
        status.compute_status();
        assert_eq!(status.status, HealthState::Unhealthy);
    }

    #[test]
    fn test_health_status_empty_checks() {
        let mut status = HealthStatus::new(HealthState::Unhealthy);
        status.compute_status();
        assert_eq!(status.status, HealthState::Healthy);
    }

    #[test]
    fn test_health_checker_creation() {
        let checker = HealthChecker::new();
        assert_eq!(checker.check_count(), 0);
    }

    #[test]
    fn test_health_checker_with_version() {
        let checker = HealthChecker::with_version("1.0.0");
        assert_eq!(checker.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_health_checker_add_check() {
        let mut checker = HealthChecker::new();
        checker.add_check(Box::new(AlwaysHealthyCheck::new("test")));
        assert_eq!(checker.check_count(), 1);
    }

    #[test]
    fn test_health_checker_with_check() {
        let checker = HealthChecker::new()
            .with_check(Box::new(AlwaysHealthyCheck::new("test1")))
            .with_check(Box::new(AlwaysHealthyCheck::new("test2")));
        assert_eq!(checker.check_count(), 2);
    }

    #[tokio::test]
    async fn test_health_checker_check_all_empty() {
        let checker = HealthChecker::new();
        let status = checker.check_all().await;
        assert_eq!(status.status, HealthState::Healthy);
        assert!(status.checks.is_empty());
    }

    #[tokio::test]
    async fn test_health_checker_check_all_healthy() {
        let checker = HealthChecker::new()
            .with_check(Box::new(AlwaysHealthyCheck::new("check1")))
            .with_check(Box::new(AlwaysHealthyCheck::new("check2")));

        let status = checker.check_all().await;
        assert_eq!(status.status, HealthState::Healthy);
        assert_eq!(status.checks.len(), 2);
    }

    #[tokio::test]
    async fn test_health_checker_check_all_with_unhealthy() {
        let manual_check = Arc::new(ManualHealthCheck::new("manual"));
        manual_check.set_unhealthy("test failure");

        let mut checker = HealthChecker::new();
        checker.add_check(Box::new(AlwaysHealthyCheck::new("healthy")));

        // We need to create a wrapper to use the Arc
        struct ArcHealthCheck(Arc<ManualHealthCheck>);

        #[async_trait::async_trait]
        impl HealthCheck for ArcHealthCheck {
            fn name(&self) -> &str {
                self.0.name()
            }
            async fn check(&self) -> CheckResult {
                self.0.check().await
            }
        }

        checker.add_check(Box::new(ArcHealthCheck(manual_check)));

        let status = checker.check_all().await;
        assert_eq!(status.status, HealthState::Unhealthy);
    }

    #[test]
    fn test_always_healthy_check() {
        let check = AlwaysHealthyCheck::new("test");
        assert_eq!(check.name(), "test");
    }

    #[tokio::test]
    async fn test_always_healthy_check_result() {
        let check = AlwaysHealthyCheck::new("test");
        let result = check.check().await;
        assert_eq!(result.status, HealthState::Healthy);
    }

    #[test]
    fn test_manual_health_check_initial_state() {
        let check = ManualHealthCheck::new("test");
        assert!(check.is_healthy());
    }

    #[test]
    fn test_manual_health_check_set_unhealthy() {
        let check = ManualHealthCheck::new("test");
        check.set_unhealthy("error");
        assert!(!check.is_healthy());
    }

    #[test]
    fn test_manual_health_check_set_healthy() {
        let check = ManualHealthCheck::new("test");
        check.set_unhealthy("error");
        check.set_healthy();
        assert!(check.is_healthy());
    }

    #[tokio::test]
    async fn test_manual_health_check_result_healthy() {
        let check = ManualHealthCheck::new("test");
        let result = check.check().await;
        assert_eq!(result.status, HealthState::Healthy);
    }

    #[tokio::test]
    async fn test_manual_health_check_result_unhealthy() {
        let check = ManualHealthCheck::new("test");
        check.set_unhealthy("connection lost");
        let result = check.check().await;
        assert_eq!(result.status, HealthState::Unhealthy);
        assert_eq!(result.message, Some("connection lost".to_string()));
    }

    #[tokio::test]
    async fn test_liveness_handler() {
        let response = liveness_handler().await;
        let body = response.into_response();
        assert_eq!(body.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_simple_readiness_handler() {
        let response = simple_readiness_handler().await;
        let body = response.into_response();
        assert_eq!(body.status(), StatusCode::OK);
    }

    #[test]
    fn test_health_status_serialization() {
        let mut status =
            HealthStatus::new(HealthState::Healthy).with_version("1.0.0").with_uptime(3600);
        status.add_check(CheckResult::healthy("test"));

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"uptime_seconds\":3600"));
    }

    // ========================================================================
    // Health Probe Handler Tests (Requirements 3.3, 3.4, 3.5)
    // ========================================================================

    /// Test that liveness probe returns 200 OK when process is alive.
    /// This validates Requirement 3.4: THE Server SHALL expose `/health/live`
    /// endpoint returning 200 when the process is alive.
    #[tokio::test]
    async fn test_liveness_returns_200_when_process_alive() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        let app = Router::new().route("/health/live", get(liveness_handler));

        let response = app
            .oneshot(Request::builder().uri("/health/live").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify response body contains status ok
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    /// Test that readiness probe returns 200 OK when all health checks pass.
    /// This validates Requirement 3.3: THE Server SHALL expose `/health/ready`
    /// endpoint returning 200 when ready to accept traffic.
    #[tokio::test]
    async fn test_readiness_returns_200_when_all_checks_pass() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        let checker = Arc::new(
            HealthChecker::with_version("1.0.0")
                .with_check(Box::new(AlwaysHealthyCheck::new("database")))
                .with_check(Box::new(AlwaysHealthyCheck::new("cache"))),
        );

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
        assert!(json["checks"]["database"].is_object());
        assert!(json["checks"]["cache"].is_object());
    }

    /// Test that readiness probe returns 503 when pool is exhausted (simulated via unhealthy check).
    /// This validates Requirement 3.5: WHEN the database connection pool is exhausted,
    /// THE Health_Probe SHALL return 503 on `/health/ready`.
    #[tokio::test]
    async fn test_readiness_returns_503_when_pool_exhausted() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        // Create a manual health check to simulate pool exhaustion
        let pool_check = Arc::new(ManualHealthCheck::new("database_pool"));
        pool_check.set_unhealthy("connection pool exhausted: no available connections");

        // Wrapper to use Arc<ManualHealthCheck> as a HealthCheck
        struct PoolExhaustedCheck(Arc<ManualHealthCheck>);

        #[async_trait::async_trait]
        impl HealthCheck for PoolExhaustedCheck {
            fn name(&self) -> &str {
                self.0.name()
            }
            async fn check(&self) -> CheckResult {
                self.0.check().await
            }
        }

        let checker = Arc::new(
            HealthChecker::with_version("1.0.0")
                .with_check(Box::new(PoolExhaustedCheck(pool_check))),
        );

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should return 503 Service Unavailable when pool is exhausted
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Verify response body indicates unhealthy status
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "unhealthy");
        assert_eq!(json["checks"]["database_pool"]["status"], "unhealthy");
        assert!(
            json["checks"]["database_pool"]["message"]
                .as_str()
                .unwrap()
                .contains("pool exhausted")
        );
    }

    /// Test that readiness probe returns 200 OK when status is degraded (not unhealthy).
    /// Degraded services should still accept traffic, only unhealthy returns 503.
    #[tokio::test]
    async fn test_readiness_returns_200_when_degraded() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        // Create a health check that returns degraded status
        struct DegradedCheck;

        #[async_trait::async_trait]
        impl HealthCheck for DegradedCheck {
            fn name(&self) -> &str {
                "slow_service"
            }
            async fn check(&self) -> CheckResult {
                CheckResult::degraded("slow_service", "high latency detected")
            }
        }

        let checker = Arc::new(
            HealthChecker::with_version("1.0.0")
                .with_check(Box::new(AlwaysHealthyCheck::new("database")))
                .with_check(Box::new(DegradedCheck)),
        );

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Degraded should still return 200 OK (service is operational)
        assert_eq!(response.status(), StatusCode::OK);

        // Verify response body indicates degraded status
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "degraded");
    }

    /// Test that readiness probe returns 503 when any single check is unhealthy,
    /// even if other checks are healthy.
    #[tokio::test]
    async fn test_readiness_returns_503_when_any_check_unhealthy() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        // Create an unhealthy check
        struct UnhealthyCheck;

        #[async_trait::async_trait]
        impl HealthCheck for UnhealthyCheck {
            fn name(&self) -> &str {
                "external_api"
            }
            async fn check(&self) -> CheckResult {
                CheckResult::unhealthy("external_api", "connection refused")
            }
        }

        let checker = Arc::new(
            HealthChecker::with_version("1.0.0")
                .with_check(Box::new(AlwaysHealthyCheck::new("database")))
                .with_check(Box::new(AlwaysHealthyCheck::new("cache")))
                .with_check(Box::new(UnhealthyCheck)),
        );

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should return 503 when any check is unhealthy
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "unhealthy");
        // Verify the unhealthy check is in the response
        assert_eq!(json["checks"]["external_api"]["status"], "unhealthy");
        // Verify healthy checks are also reported
        assert_eq!(json["checks"]["database"]["status"], "healthy");
    }

    /// Test that readiness probe returns 200 with empty checks (no dependencies).
    #[tokio::test]
    async fn test_readiness_returns_200_with_no_checks() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        let checker = Arc::new(HealthChecker::new());

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
        assert!(json["checks"].as_object().unwrap().is_empty());
    }

    /// Test that readiness response includes version and uptime metadata.
    #[tokio::test]
    async fn test_readiness_includes_metadata() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        let checker = Arc::new(HealthChecker::with_version("2.1.0"));

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify version is included
        assert_eq!(json["version"], "2.1.0");
        // Verify uptime is included (should be a small number since checker was just created)
        assert!(json["uptime_seconds"].is_number());
    }

    /// Test that health check duration is recorded in the response.
    #[tokio::test]
    async fn test_readiness_records_check_duration() {
        use axum::Router;
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::get;
        use tower::ServiceExt;

        // Create a health check with a small delay to ensure measurable duration
        struct SlowCheck;

        #[async_trait::async_trait]
        impl HealthCheck for SlowCheck {
            fn name(&self) -> &str {
                "slow_check"
            }
            async fn check(&self) -> CheckResult {
                tokio::time::sleep(Duration::from_millis(10)).await;
                CheckResult::healthy("slow_check")
            }
        }

        let checker = Arc::new(HealthChecker::new().with_check(Box::new(SlowCheck)));

        let app = Router::new().route("/health/ready", get(readiness_handler)).with_state(checker);

        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify duration_ms is recorded and is at least 10ms
        let duration = json["checks"]["slow_check"]["duration_ms"].as_u64().unwrap();
        assert!(duration >= 10, "Expected duration >= 10ms, got {}ms", duration);
    }
}
