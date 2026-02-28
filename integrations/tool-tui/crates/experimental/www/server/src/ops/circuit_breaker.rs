//! Circuit Breaker Pattern Implementation
//!
//! This module implements the circuit breaker pattern for resilience against
//! cascading failures when calling external services.
//!
//! # Circuit States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Circuit is tripped, requests fail fast
//! - **HalfOpen**: Testing if the service has recovered
//!
//! # State Transitions
//!
//! ```text
//! ┌─────────┐  failure_threshold  ┌──────┐  reset_timeout  ┌──────────┐
//! │ Closed  │ ─────────────────→  │ Open │ ─────────────→  │ HalfOpen │
//! └─────────┘                     └──────┘                 └──────────┘
//!      ↑                               ↑                        │
//!      │                               │                        │
//!      │         success               │        failure         │
//!      └───────────────────────────────┴────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_server::ops::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
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
//!
//! match result {
//!     Ok(value) => println!("Success: {:?}", value),
//!     Err(CircuitBreakerError::Open) => println!("Circuit is open, failing fast"),
//!     Err(CircuitBreakerError::Inner(e)) => println!("Service error: {:?}", e),
//! }
//! ```

use std::future::Future;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

/// The state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through normally.
    /// Failures are counted and may trip the circuit.
    Closed = 0,
    /// Circuit is open, requests fail fast without calling the service.
    /// After reset_timeout, transitions to HalfOpen.
    Open = 1,
    /// Circuit is half-open, allowing limited probe requests.
    /// Success transitions to Closed, failure transitions back to Open.
    HalfOpen = 2,
}

impl CircuitState {
    /// Convert from u8 representation.
    fn from_u8(value: u8) -> Self {
        match value {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed, // Default to closed for safety
        }
    }
}

/// Configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: u32,
    /// Duration to wait before transitioning from Open to HalfOpen.
    pub reset_timeout: Duration,
    /// Maximum number of probe calls allowed in HalfOpen state.
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }
}

/// Error type for circuit breaker operations.
#[derive(Debug, Error)]
pub enum CircuitBreakerError<E> {
    /// The circuit is open and not accepting requests.
    #[error("circuit breaker is open")]
    Open,
    /// The underlying operation failed.
    #[error("inner error: {0}")]
    Inner(E),
}

/// A thread-safe circuit breaker implementation.
///
/// The circuit breaker monitors failures and prevents cascading failures
/// by failing fast when a service is unhealthy.
pub struct CircuitBreaker {
    /// Current state of the circuit (stored as u8 for atomic operations).
    state: AtomicU8,
    /// Count of consecutive failures.
    failure_count: AtomicU32,
    /// Timestamp of the last failure (milliseconds since circuit creation).
    last_failure_time: AtomicU64,
    /// Count of calls in half-open state.
    half_open_calls: AtomicU32,
    /// Configuration for the circuit breaker.
    config: CircuitBreakerConfig,
    /// Instant when the circuit breaker was created (for time calculations).
    created_at: Instant,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            half_open_calls: AtomicU32::new(0),
            config,
            created_at: Instant::now(),
        }
    }

    /// Create a new circuit breaker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get the current state of the circuit breaker.
    pub fn state(&self) -> CircuitState {
        // First check if we should transition from Open to HalfOpen
        let current_state = CircuitState::from_u8(self.state.load(Ordering::SeqCst));

        if current_state == CircuitState::Open {
            let last_failure = self.last_failure_time.load(Ordering::SeqCst);
            let elapsed_ms = self.created_at.elapsed().as_millis() as u64;
            let reset_timeout_ms = self.config.reset_timeout.as_millis() as u64;

            if elapsed_ms.saturating_sub(last_failure) >= reset_timeout_ms {
                // Try to transition to HalfOpen
                if self
                    .state
                    .compare_exchange(
                        CircuitState::Open as u8,
                        CircuitState::HalfOpen as u8,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    // Reset half-open call counter
                    self.half_open_calls.store(0, Ordering::SeqCst);
                    return CircuitState::HalfOpen;
                }
            }
        }

        CircuitState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Get the current failure count.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }

    /// Get the configuration.
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Record a successful call.
    fn record_success(&self) {
        let current_state = CircuitState::from_u8(self.state.load(Ordering::SeqCst));

        match current_state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                // Transition to Closed on success in HalfOpen state
                self.state.store(CircuitState::Closed as u8, Ordering::SeqCst);
                self.failure_count.store(0, Ordering::SeqCst);
                self.half_open_calls.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {
                // Should not happen, but reset anyway
                self.failure_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Record a failed call.
    fn record_failure(&self) {
        let current_state = CircuitState::from_u8(self.state.load(Ordering::SeqCst));
        let now_ms = self.created_at.elapsed().as_millis() as u64;

        match current_state {
            CircuitState::Closed => {
                let new_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                self.last_failure_time.store(now_ms, Ordering::SeqCst);

                // Check if we should open the circuit
                if new_count >= self.config.failure_threshold {
                    self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
                }
            }
            CircuitState::HalfOpen => {
                // Transition back to Open on failure in HalfOpen state
                self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
                self.last_failure_time.store(now_ms, Ordering::SeqCst);
                self.half_open_calls.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {
                // Update last failure time
                self.last_failure_time.store(now_ms, Ordering::SeqCst);
            }
        }
    }

    /// Check if a call should be allowed in the current state.
    fn should_allow_call(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => {
                // Allow limited calls in half-open state
                let current_calls = self.half_open_calls.fetch_add(1, Ordering::SeqCst);
                current_calls < self.config.half_open_max_calls
            }
        }
    }

    /// Execute a function through the circuit breaker.
    ///
    /// If the circuit is open, returns `CircuitBreakerError::Open` immediately.
    /// If the circuit is closed or half-open, executes the function and records
    /// the result.
    ///
    /// # Arguments
    ///
    /// * `f` - A future that returns a `Result<T, E>`
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The operation succeeded
    /// * `Err(CircuitBreakerError::Open)` - The circuit is open
    /// * `Err(CircuitBreakerError::Inner(E))` - The operation failed
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        if !self.should_allow_call() {
            return Err(CircuitBreakerError::Open);
        }

        match f.await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(CircuitBreakerError::Inner(e))
            }
        }
    }

    /// Manually reset the circuit breaker to closed state.
    ///
    /// This can be useful for administrative purposes or testing.
    pub fn reset(&self) {
        self.state.store(CircuitState::Closed as u8, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.half_open_calls.store(0, Ordering::SeqCst);
    }

    /// Manually trip the circuit breaker to open state.
    ///
    /// This can be useful for maintenance windows or known outages.
    pub fn trip(&self) {
        let now_ms = self.created_at.elapsed().as_millis() as u64;
        self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
        self.last_failure_time.store(now_ms, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("state", &self.state())
            .field("failure_count", &self.failure_count())
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_circuit_state_from_u8() {
        assert_eq!(CircuitState::from_u8(0), CircuitState::Closed);
        assert_eq!(CircuitState::from_u8(1), CircuitState::Open);
        assert_eq!(CircuitState::from_u8(2), CircuitState::HalfOpen);
        assert_eq!(CircuitState::from_u8(255), CircuitState::Closed); // Invalid defaults to Closed
    }

    #[test]
    fn test_default_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.reset_timeout, Duration::from_secs(30));
        assert_eq!(config.half_open_max_calls, 3);
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let breaker = CircuitBreaker::with_defaults();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
    }

    #[tokio::test]
    async fn test_successful_call_keeps_circuit_closed() {
        let breaker = CircuitBreaker::with_defaults();

        let result: Result<i32, CircuitBreakerError<&str>> =
            breaker.call(async { Ok::<_, &str>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
    }

    #[tokio::test]
    async fn test_failed_call_increments_failure_count() {
        let breaker = CircuitBreaker::with_defaults();

        let result: Result<i32, CircuitBreakerError<&str>> =
            breaker.call(async { Err::<i32, _>("error") }).await;

        assert!(matches!(result, Err(CircuitBreakerError::Inner("error"))));
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 1);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Cause 3 failures
        for _ in 0..3 {
            let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
        }

        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_open_circuit_fails_fast() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
        assert_eq!(breaker.state(), CircuitState::Open);

        // Next call should fail fast
        let result: Result<i32, CircuitBreakerError<&str>> =
            breaker.call(async { Ok::<_, &str>(42) }).await;

        assert!(matches!(result, Err(CircuitBreakerError::Open)));
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should transition to HalfOpen
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_half_open_success_closes_circuit() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Successful call in half-open state
        let result: Result<i32, CircuitBreakerError<&str>> =
            breaker.call(async { Ok::<_, &str>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_failure_reopens_circuit() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Failed call in half-open state
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;

        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Cause some failures
        for _ in 0..3 {
            let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
        }
        assert_eq!(breaker.failure_count(), 3);

        // Successful call resets count
        let _: Result<i32, _> = breaker.call(async { Ok::<i32, &str>(42) }).await;
        assert_eq!(breaker.failure_count(), 0);
    }

    #[test]
    fn test_manual_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit manually
        breaker.trip();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Reset manually
        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
    }

    #[test]
    fn test_manual_trip() {
        let breaker = CircuitBreaker::with_defaults();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.trip();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_debug_impl() {
        let breaker = CircuitBreaker::with_defaults();
        let debug_str = format!("{:?}", breaker);
        assert!(debug_str.contains("CircuitBreaker"));
        assert!(debug_str.contains("Closed"));
    }

    #[tokio::test]
    async fn test_half_open_limits_calls() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(10),
            half_open_max_calls: 2,
        };
        let breaker = Arc::new(CircuitBreaker::new(config));

        // Trip the circuit
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // First call in half-open should be allowed
        assert!(breaker.should_allow_call());
        // Second call should be allowed
        assert!(breaker.should_allow_call());
        // Third call should be rejected (max_calls = 2)
        assert!(!breaker.should_allow_call());
    }

    #[tokio::test]
    async fn test_circuit_breaker_error_display() {
        let open_error: CircuitBreakerError<&str> = CircuitBreakerError::Open;
        assert_eq!(format!("{}", open_error), "circuit breaker is open");

        let inner_error: CircuitBreakerError<&str> = CircuitBreakerError::Inner("test error");
        assert_eq!(format!("{}", inner_error), "inner error: test error");
    }
}
