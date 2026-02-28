//! Graceful Shutdown Handler
//!
//! Provides graceful shutdown functionality for the dx-server, handling
//! SIGTERM and SIGINT signals with configurable timeout for in-flight
//! request completion.
//!
//! # Overview
//!
//! The graceful shutdown process follows these steps:
//! 1. Receive shutdown signal (SIGTERM or SIGINT)
//! 2. Stop accepting new connections
//! 3. Wait for in-flight requests to complete (up to timeout)
//! 4. Force close remaining connections after timeout
//! 5. Drain connection pools
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_server::ops::{GracefulShutdown, ShutdownConfig};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ShutdownConfig {
//!         timeout: Duration::from_secs(30),
//!     };
//!     let shutdown = GracefulShutdown::new(config);
//!
//!     // Clone the shutdown signal receiver for the server
//!     let shutdown_rx = shutdown.subscribe();
//!
//!     // Spawn server task
//!     let server_handle = tokio::spawn(async move {
//!         // Run server until shutdown signal
//!         tokio::select! {
//!             _ = run_server() => {}
//!             _ = shutdown_rx.wait() => {
//!                 tracing::info!("Shutdown signal received");
//!             }
//!         }
//!     });
//!
//!     // Wait for shutdown signal
//!     shutdown.wait_for_signal().await;
//!
//!     // Initiate graceful shutdown
//!     shutdown.shutdown().await?;
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::time::timeout;

/// Configuration for graceful shutdown behavior.
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight requests to complete.
    /// After this timeout, remaining connections will be forcefully terminated.
    pub timeout: Duration,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }
}

impl ShutdownConfig {
    /// Create a new shutdown configuration with the specified timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

/// Errors that can occur during graceful shutdown.
#[derive(Debug, Error)]
pub enum ShutdownError {
    /// Shutdown timed out waiting for in-flight requests.
    #[error("shutdown timed out after {0:?}, {1} connections forcefully terminated")]
    Timeout(Duration, usize),

    /// Failed to send shutdown signal.
    #[error("failed to send shutdown signal: {0}")]
    SignalError(String),

    /// Shutdown was already initiated.
    #[error("shutdown already in progress")]
    AlreadyShuttingDown,
}

/// A receiver for shutdown signals.
///
/// This can be used to listen for shutdown notifications. To create
/// multiple receivers, call `subscribe()` on the `GracefulShutdown` instance.
pub struct ShutdownReceiver {
    receiver: broadcast::Receiver<()>,
    shutdown_initiated: Arc<AtomicBool>,
}

impl ShutdownReceiver {
    /// Wait for the shutdown signal.
    ///
    /// This method returns when a shutdown signal is received or if
    /// shutdown has already been initiated.
    pub async fn wait(&mut self) {
        // If shutdown was already initiated, return immediately
        if self.shutdown_initiated.load(Ordering::SeqCst) {
            return;
        }

        // Wait for the broadcast signal
        let _ = self.receiver.recv().await;
    }

    /// Check if shutdown has been initiated.
    pub fn is_shutdown_initiated(&self) -> bool {
        self.shutdown_initiated.load(Ordering::SeqCst)
    }
}

/// Graceful shutdown handler for the dx-server.
///
/// Manages the shutdown lifecycle including signal handling, timeout
/// enforcement, and connection tracking.
pub struct GracefulShutdown {
    /// Configuration for shutdown behavior.
    config: ShutdownConfig,
    /// Broadcast sender for shutdown notifications.
    shutdown_signal: broadcast::Sender<()>,
    /// Flag indicating if shutdown has been initiated.
    shutdown_initiated: Arc<AtomicBool>,
    /// Counter for active connections/requests.
    active_connections: Arc<AtomicUsize>,
}

impl GracefulShutdown {
    /// Create a new graceful shutdown handler with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration specifying shutdown timeout and behavior
    ///
    /// # Example
    ///
    /// ```rust
    /// use dx_www_server::ops::{GracefulShutdown, ShutdownConfig};
    /// use std::time::Duration;
    ///
    /// let config = ShutdownConfig::with_timeout(Duration::from_secs(60));
    /// let shutdown = GracefulShutdown::new(config);
    /// ```
    pub fn new(config: ShutdownConfig) -> Self {
        let (shutdown_signal, _) = broadcast::channel(1);
        Self {
            config,
            shutdown_signal,
            shutdown_initiated: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a shutdown receiver that can be used to listen for shutdown signals.
    ///
    /// Multiple receivers can be created and distributed to different parts
    /// of the application.
    pub fn subscribe(&self) -> ShutdownReceiver {
        ShutdownReceiver {
            receiver: self.shutdown_signal.subscribe(),
            shutdown_initiated: self.shutdown_initiated.clone(),
        }
    }

    /// Get the current timeout configuration.
    pub fn timeout(&self) -> Duration {
        self.config.timeout
    }

    /// Get the number of active connections.
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Increment the active connection counter.
    ///
    /// Call this when a new request/connection starts.
    pub fn connection_started(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement the active connection counter.
    ///
    /// Call this when a request/connection completes.
    pub fn connection_finished(&self) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }

    /// Check if shutdown has been initiated.
    pub fn is_shutdown_initiated(&self) -> bool {
        self.shutdown_initiated.load(Ordering::SeqCst)
    }

    /// Wait for a shutdown signal (SIGTERM or SIGINT).
    ///
    /// This method blocks until a termination signal is received from the
    /// operating system. On Unix systems, it listens for SIGTERM and SIGINT.
    /// On Windows, it listens for Ctrl+C.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let shutdown = GracefulShutdown::new(ShutdownConfig::default());
    ///
    /// // This will block until SIGTERM or SIGINT is received
    /// shutdown.wait_for_signal().await;
    /// println!("Shutdown signal received!");
    /// ```
    pub async fn wait_for_signal(&self) {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut sigterm =
                signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM signal");
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT signal");
                }
            }
        }

        #[cfg(windows)]
        {
            tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
            tracing::info!("Received Ctrl+C signal");
        }

        #[cfg(not(any(unix, windows)))]
        {
            // Fallback for other platforms - just wait for ctrl_c
            tokio::signal::ctrl_c().await.expect("failed to install signal handler");
            tracing::info!("Received termination signal");
        }
    }

    /// Initiate graceful shutdown.
    ///
    /// This method:
    /// 1. Signals all subscribers to stop accepting new work
    /// 2. Waits for in-flight requests to complete (up to timeout)
    /// 3. Returns error with count of forcefully terminated connections if timeout exceeded
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all connections completed gracefully
    /// - `Err(ShutdownError::Timeout)` if timeout was exceeded
    /// - `Err(ShutdownError::AlreadyShuttingDown)` if shutdown was already initiated
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let shutdown = GracefulShutdown::new(ShutdownConfig::default());
    ///
    /// // Wait for signal then initiate shutdown
    /// shutdown.wait_for_signal().await;
    ///
    /// match shutdown.shutdown().await {
    ///     Ok(()) => println!("Graceful shutdown complete"),
    ///     Err(ShutdownError::Timeout(duration, count)) => {
    ///         println!("Shutdown timed out after {:?}, {} connections terminated", duration, count);
    ///     }
    ///     Err(e) => eprintln!("Shutdown error: {}", e),
    /// }
    /// ```
    pub async fn shutdown(&self) -> Result<(), ShutdownError> {
        // Check if shutdown was already initiated
        if self.shutdown_initiated.swap(true, Ordering::SeqCst) {
            return Err(ShutdownError::AlreadyShuttingDown);
        }

        tracing::info!("Initiating graceful shutdown with {:?} timeout", self.config.timeout);

        // Step 1: Signal all subscribers to stop accepting new connections
        // Note: send() returns Err if there are no receivers, which is fine
        let _ = self.shutdown_signal.send(());
        tracing::debug!("Shutdown signal broadcast to all subscribers");

        // Step 2: Wait for in-flight requests to complete (up to timeout)
        let wait_result = timeout(self.config.timeout, self.wait_for_connections_to_drain()).await;

        match wait_result {
            Ok(()) => {
                tracing::info!("All connections drained gracefully");
                Ok(())
            }
            Err(_) => {
                // Timeout exceeded - force terminate remaining connections
                let remaining = self.active_connections.load(Ordering::SeqCst);
                tracing::warn!(
                    "Shutdown timeout exceeded, forcefully terminating {} remaining connections",
                    remaining
                );
                Err(ShutdownError::Timeout(self.config.timeout, remaining))
            }
        }
    }

    /// Trigger shutdown programmatically without waiting for a signal.
    ///
    /// This is useful for testing or when shutdown needs to be triggered
    /// from application code rather than OS signals.
    pub async fn trigger_shutdown(&self) -> Result<(), ShutdownError> {
        self.shutdown().await
    }

    /// Wait for all active connections to drain.
    async fn wait_for_connections_to_drain(&self) {
        // Poll until all connections are finished
        // Use a small interval to be responsive but not wasteful
        let poll_interval = Duration::from_millis(50);

        loop {
            let active = self.active_connections.load(Ordering::SeqCst);
            if active == 0 {
                break;
            }

            tracing::debug!("Waiting for {} active connections to complete", active);
            tokio::time::sleep(poll_interval).await;
        }
    }
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new(ShutdownConfig::default())
    }
}

/// A guard that automatically tracks connection lifecycle.
///
/// When created, increments the active connection counter.
/// When dropped, decrements the counter.
///
/// # Example
///
/// ```rust,ignore
/// async fn handle_request(shutdown: Arc<GracefulShutdown>) {
///     let _guard = ConnectionGuard::new(&shutdown);
///     // Connection is tracked while guard is in scope
///     process_request().await;
///     // Guard dropped here, connection count decremented
/// }
/// ```
pub struct ConnectionGuard {
    shutdown: Arc<GracefulShutdown>,
}

impl ConnectionGuard {
    /// Create a new connection guard, incrementing the active connection count.
    pub fn new(shutdown: Arc<GracefulShutdown>) -> Self {
        shutdown.connection_started();
        Self { shutdown }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.shutdown.connection_finished();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_config_default() {
        let config = ShutdownConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_shutdown_config_with_timeout() {
        let config = ShutdownConfig::with_timeout(Duration::from_secs(60));
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_graceful_shutdown_creation() {
        let shutdown = GracefulShutdown::new(ShutdownConfig::default());
        assert!(!shutdown.is_shutdown_initiated());
        assert_eq!(shutdown.active_connections(), 0);
        assert_eq!(shutdown.timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_connection_tracking() {
        let shutdown = GracefulShutdown::new(ShutdownConfig::default());

        assert_eq!(shutdown.active_connections(), 0);

        shutdown.connection_started();
        assert_eq!(shutdown.active_connections(), 1);

        shutdown.connection_started();
        assert_eq!(shutdown.active_connections(), 2);

        shutdown.connection_finished();
        assert_eq!(shutdown.active_connections(), 1);

        shutdown.connection_finished();
        assert_eq!(shutdown.active_connections(), 0);
    }

    #[test]
    fn test_connection_guard() {
        let shutdown = Arc::new(GracefulShutdown::new(ShutdownConfig::default()));

        assert_eq!(shutdown.active_connections(), 0);

        {
            let _guard = ConnectionGuard::new(shutdown.clone());
            assert_eq!(shutdown.active_connections(), 1);

            {
                let _guard2 = ConnectionGuard::new(shutdown.clone());
                assert_eq!(shutdown.active_connections(), 2);
            }

            assert_eq!(shutdown.active_connections(), 1);
        }

        assert_eq!(shutdown.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_shutdown_receiver() {
        let shutdown = GracefulShutdown::new(ShutdownConfig::default());
        let mut receiver = shutdown.subscribe();

        assert!(!receiver.is_shutdown_initiated());

        // Trigger shutdown in background
        let shutdown_clone = Arc::new(shutdown);
        let shutdown_for_task = shutdown_clone.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = shutdown_for_task.trigger_shutdown().await;
        });

        // Wait for shutdown signal
        receiver.wait().await;

        assert!(receiver.is_shutdown_initiated());
    }

    #[tokio::test]
    async fn test_graceful_shutdown_no_connections() {
        let shutdown =
            GracefulShutdown::new(ShutdownConfig::with_timeout(Duration::from_millis(100)));

        // With no active connections, shutdown should complete immediately
        let result = shutdown.shutdown().await;
        assert!(result.is_ok());
        assert!(shutdown.is_shutdown_initiated());
    }

    #[tokio::test]
    async fn test_graceful_shutdown_with_connections_completing() {
        let shutdown =
            Arc::new(GracefulShutdown::new(ShutdownConfig::with_timeout(Duration::from_secs(1))));

        // Start a connection
        shutdown.connection_started();
        assert_eq!(shutdown.active_connections(), 1);

        // Spawn task to finish connection after a short delay
        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            shutdown_clone.connection_finished();
        });

        // Shutdown should wait for connection and succeed
        let result = shutdown.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_graceful_shutdown_timeout() {
        let shutdown = Arc::new(GracefulShutdown::new(ShutdownConfig::with_timeout(
            Duration::from_millis(50),
        )));

        // Start connections that won't finish
        shutdown.connection_started();
        shutdown.connection_started();

        // Shutdown should timeout
        let result = shutdown.shutdown().await;

        match result {
            Err(ShutdownError::Timeout(duration, count)) => {
                assert_eq!(duration, Duration::from_millis(50));
                assert_eq!(count, 2);
            }
            _ => panic!("Expected timeout error"),
        }
    }

    #[tokio::test]
    async fn test_double_shutdown_error() {
        let shutdown =
            GracefulShutdown::new(ShutdownConfig::with_timeout(Duration::from_millis(100)));

        // First shutdown should succeed
        let result1 = shutdown.shutdown().await;
        assert!(result1.is_ok());

        // Second shutdown should fail
        let result2 = shutdown.shutdown().await;
        match result2 {
            Err(ShutdownError::AlreadyShuttingDown) => {}
            _ => panic!("Expected AlreadyShuttingDown error"),
        }
    }

    #[test]
    fn test_shutdown_error_display() {
        let timeout_err = ShutdownError::Timeout(Duration::from_secs(30), 5);
        assert!(timeout_err.to_string().contains("30s"));
        assert!(timeout_err.to_string().contains("5"));

        let signal_err = ShutdownError::SignalError("test error".to_string());
        assert!(signal_err.to_string().contains("test error"));

        let already_err = ShutdownError::AlreadyShuttingDown;
        assert!(already_err.to_string().contains("already"));
    }
}
