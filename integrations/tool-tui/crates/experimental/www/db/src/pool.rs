//! Connection pool configuration and health checking.
//!
//! This module provides enhanced connection pool configuration with
//! production-ready defaults and health check capabilities for
//! Kubernetes readiness probes.

use std::time::Duration;

/// Enhanced connection pool configuration.
///
/// Provides comprehensive configuration for database connection pools
/// with production-ready defaults suitable for high-traffic applications.
///
/// # Example
///
/// ```rust
/// use db::pool::PoolConfig;
/// use std::time::Duration;
///
/// let config = PoolConfig {
///     min_connections: 10,
///     max_connections: 50,
///     acquire_timeout: Duration::from_secs(15),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain in the pool.
    /// The pool will attempt to keep at least this many idle connections.
    pub min_connections: u32,

    /// Maximum number of connections allowed in the pool.
    /// Requests for connections when at capacity will wait up to `acquire_timeout`.
    pub max_connections: u32,

    /// Maximum time to wait when acquiring a connection from the pool.
    /// If no connection becomes available within this duration, an error is returned.
    pub acquire_timeout: Duration,

    /// Maximum time a connection can remain idle before being closed.
    /// Helps prevent stale connections and reduces resource usage during low traffic.
    pub idle_timeout: Duration,

    /// Maximum lifetime of a connection.
    /// Connections older than this will be closed and replaced,
    /// helping to prevent issues with long-lived connections.
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 100,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl PoolConfig {
    /// Creates a new `PoolConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum number of connections.
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Sets the maximum number of connections.
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Sets the acquire timeout.
    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    /// Sets the idle timeout.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Sets the maximum connection lifetime.
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Validates the configuration.
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), PoolConfigError> {
        if self.min_connections > self.max_connections {
            return Err(PoolConfigError::InvalidBounds {
                min: self.min_connections,
                max: self.max_connections,
            });
        }

        if self.max_connections == 0 {
            return Err(PoolConfigError::ZeroMaxConnections);
        }

        if self.acquire_timeout.is_zero() {
            return Err(PoolConfigError::ZeroAcquireTimeout);
        }

        Ok(())
    }
}

/// Errors that can occur when validating pool configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolConfigError {
    /// min_connections exceeds max_connections
    InvalidBounds { min: u32, max: u32 },
    /// max_connections is zero
    ZeroMaxConnections,
    /// acquire_timeout is zero
    ZeroAcquireTimeout,
}

impl std::fmt::Display for PoolConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolConfigError::InvalidBounds { min, max } => {
                write!(f, "min_connections ({}) cannot exceed max_connections ({})", min, max)
            }
            PoolConfigError::ZeroMaxConnections => {
                write!(f, "max_connections must be greater than zero")
            }
            PoolConfigError::ZeroAcquireTimeout => {
                write!(f, "acquire_timeout must be greater than zero")
            }
        }
    }
}

impl std::error::Error for PoolConfigError {}

/// Health status of a connection pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolHealthStatus {
    /// Pool is healthy and ready to accept connections.
    Healthy,
    /// Pool is degraded but still functional.
    Degraded {
        /// Reason for degraded status.
        reason: String,
    },
    /// Pool is unhealthy and cannot serve requests.
    Unhealthy {
        /// Reason for unhealthy status.
        reason: String,
    },
}

impl PoolHealthStatus {
    /// Returns true if the pool is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, PoolHealthStatus::Healthy)
    }

    /// Returns true if the pool can still serve requests (healthy or degraded).
    pub fn is_ready(&self) -> bool {
        matches!(self, PoolHealthStatus::Healthy | PoolHealthStatus::Degraded { .. })
    }
}

/// Statistics about the connection pool.
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Current number of active (in-use) connections.
    pub active_connections: u32,
    /// Current number of idle connections.
    pub idle_connections: u32,
    /// Total number of connections (active + idle).
    pub total_connections: u32,
    /// Maximum number of connections allowed.
    pub max_connections: u32,
    /// Number of waiters for a connection.
    pub pending_requests: u32,
}

impl PoolStats {
    /// Returns the pool utilization as a percentage (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        if self.max_connections == 0 {
            return 0.0;
        }
        self.active_connections as f64 / self.max_connections as f64
    }

    /// Returns true if the pool is at capacity.
    pub fn is_exhausted(&self) -> bool {
        self.total_connections >= self.max_connections && self.idle_connections == 0
    }
}

/// Trait for pool health checking.
///
/// Implement this trait to enable health checks for your connection pool.
/// This is used by readiness probes to determine if the service can accept traffic.
pub trait PoolHealthCheck: Send + Sync {
    /// Returns the current health status of the pool.
    fn health_status(&self) -> PoolHealthStatus;

    /// Returns current pool statistics.
    fn stats(&self) -> PoolStats;

    /// Performs a health check by attempting to acquire and release a connection.
    ///
    /// This is an async operation that verifies the pool can actually serve requests.
    fn check(&self) -> impl std::future::Future<Output = PoolHealthStatus> + Send;
}

/// A mock pool implementation for testing health checks.
#[cfg(test)]
pub struct MockPool {
    config: PoolConfig,
    active: std::sync::atomic::AtomicU32,
    idle: std::sync::atomic::AtomicU32,
    healthy: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl MockPool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            active: std::sync::atomic::AtomicU32::new(0),
            idle: std::sync::atomic::AtomicU32::new(config.min_connections),
            healthy: std::sync::atomic::AtomicBool::new(true),
            config,
        }
    }

    pub fn set_active(&self, count: u32) {
        self.active.store(count, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_idle(&self, count: u32) {
        self.idle.store(count, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_healthy(&self, healthy: bool) {
        self.healthy.store(healthy, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
impl PoolHealthCheck for MockPool {
    fn health_status(&self) -> PoolHealthStatus {
        if !self.healthy.load(std::sync::atomic::Ordering::SeqCst) {
            return PoolHealthStatus::Unhealthy {
                reason: "Pool marked unhealthy".to_string(),
            };
        }

        let stats = self.stats();
        if stats.is_exhausted() {
            return PoolHealthStatus::Unhealthy {
                reason: "Connection pool exhausted".to_string(),
            };
        }

        if stats.utilization() > 0.9 {
            return PoolHealthStatus::Degraded {
                reason: format!("Pool utilization at {:.0}%", stats.utilization() * 100.0),
            };
        }

        PoolHealthStatus::Healthy
    }

    fn stats(&self) -> PoolStats {
        let active = self.active.load(std::sync::atomic::Ordering::SeqCst);
        let idle = self.idle.load(std::sync::atomic::Ordering::SeqCst);
        PoolStats {
            active_connections: active,
            idle_connections: idle,
            total_connections: active + idle,
            max_connections: self.config.max_connections,
            pending_requests: 0,
        }
    }

    async fn check(&self) -> PoolHealthStatus {
        self.health_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new()
            .min_connections(10)
            .max_connections(50)
            .acquire_timeout(Duration::from_secs(15))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(900));

        assert_eq!(config.min_connections, 10);
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.acquire_timeout, Duration::from_secs(15));
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.max_lifetime, Duration::from_secs(900));
    }

    #[test]
    fn test_pool_config_validation_valid() {
        let config = PoolConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pool_config_validation_invalid_bounds() {
        let config = PoolConfig {
            min_connections: 100,
            max_connections: 10,
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(PoolConfigError::InvalidBounds { min: 100, max: 10 })));
    }

    #[test]
    fn test_pool_config_validation_zero_max() {
        let config = PoolConfig {
            max_connections: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(PoolConfigError::ZeroMaxConnections)));
    }

    #[test]
    fn test_pool_config_validation_zero_timeout() {
        let config = PoolConfig {
            acquire_timeout: Duration::ZERO,
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(PoolConfigError::ZeroAcquireTimeout)));
    }

    #[test]
    fn test_pool_stats_utilization() {
        let stats = PoolStats {
            active_connections: 50,
            idle_connections: 10,
            total_connections: 60,
            max_connections: 100,
            pending_requests: 0,
        };
        assert!((stats.utilization() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pool_stats_exhausted() {
        let stats = PoolStats {
            active_connections: 100,
            idle_connections: 0,
            total_connections: 100,
            max_connections: 100,
            pending_requests: 5,
        };
        assert!(stats.is_exhausted());

        let stats_not_exhausted = PoolStats {
            active_connections: 90,
            idle_connections: 10,
            total_connections: 100,
            max_connections: 100,
            pending_requests: 0,
        };
        assert!(!stats_not_exhausted.is_exhausted());
    }

    #[test]
    fn test_pool_health_status() {
        assert!(PoolHealthStatus::Healthy.is_healthy());
        assert!(PoolHealthStatus::Healthy.is_ready());

        let degraded = PoolHealthStatus::Degraded {
            reason: "test".to_string(),
        };
        assert!(!degraded.is_healthy());
        assert!(degraded.is_ready());

        let unhealthy = PoolHealthStatus::Unhealthy {
            reason: "test".to_string(),
        };
        assert!(!unhealthy.is_healthy());
        assert!(!unhealthy.is_ready());
    }

    #[test]
    fn test_mock_pool_health_check() {
        let config = PoolConfig::default();
        let pool = MockPool::new(config);

        // Initially healthy
        assert!(pool.health_status().is_healthy());

        // Set high utilization - should be degraded
        pool.set_active(95);
        pool.set_idle(0);
        assert!(matches!(pool.health_status(), PoolHealthStatus::Degraded { .. }));

        // Exhaust pool - should be unhealthy
        pool.set_active(100);
        pool.set_idle(0);
        assert!(matches!(pool.health_status(), PoolHealthStatus::Unhealthy { .. }));

        // Mark unhealthy explicitly
        pool.set_active(10);
        pool.set_idle(10);
        pool.set_healthy(false);
        assert!(matches!(pool.health_status(), PoolHealthStatus::Unhealthy { .. }));
    }

    #[test]
    fn test_pool_config_error_display() {
        let err = PoolConfigError::InvalidBounds { min: 100, max: 10 };
        assert_eq!(err.to_string(), "min_connections (100) cannot exceed max_connections (10)");

        let err = PoolConfigError::ZeroMaxConnections;
        assert_eq!(err.to_string(), "max_connections must be greater than zero");

        let err = PoolConfigError::ZeroAcquireTimeout;
        assert_eq!(err.to_string(), "acquire_timeout must be greater than zero");
    }
}
