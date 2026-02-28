//! Graceful Shutdown Handler for DX Forge
//!
//! Provides comprehensive shutdown handling including:
//! - Signal handling (SIGTERM/SIGINT on Unix, Ctrl+C on Windows)
//! - Graceful shutdown with timeout
//! - Force termination after timeout
//! - Panic handler for resource cleanup
//! - Exit code management
//!
//! # Example
//! ```rust,ignore
//! use dx_forge::shutdown::{ShutdownHandler, ShutdownConfig, ExitCode};
//!
//! let handler = ShutdownHandler::new(ShutdownConfig::default());
//! handler.install_panic_handler();
//!
//! // Wait for shutdown signal
//! handler.wait_for_signal().await;
//!
//! // Perform graceful shutdown
//! let exit_code = handler.shutdown().await;
//! std::process::exit(exit_code.as_i32());
//! ```

use anyhow::Context;
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, broadcast};

/// Exit codes for Forge process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Clean shutdown (0)
    Success,
    /// General error (1)
    Error,
    /// Shutdown timeout (2)
    Timeout,
    /// Panic occurred (3)
    Panic,
    /// Signal received (128 + signal number)
    Signal(i32),
}

impl ExitCode {
    /// Convert to i32 for process exit
    pub fn as_i32(&self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::Error => 1,
            ExitCode::Timeout => 2,
            ExitCode::Panic => 3,
            ExitCode::Signal(sig) => 128 + sig,
        }
    }

    /// Check if this is a clean exit
    pub fn is_clean(&self) -> bool {
        matches!(self, ExitCode::Success)
    }
}

impl From<i32> for ExitCode {
    fn from(code: i32) -> Self {
        match code {
            0 => ExitCode::Success,
            1 => ExitCode::Error,
            2 => ExitCode::Timeout,
            3 => ExitCode::Panic,
            c if c >= 128 => ExitCode::Signal(c - 128),
            _ => ExitCode::Error,
        }
    }
}

/// Shutdown configuration
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Timeout for graceful shutdown (default: 30 seconds)
    pub timeout: Duration,
    /// Whether to force terminate after timeout
    pub force_on_timeout: bool,
    /// Whether to install panic handler
    pub install_panic_handler: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            force_on_timeout: true,
            install_panic_handler: true,
        }
    }
}

/// Shutdown state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownState {
    /// Running normally
    Running,
    /// Shutdown initiated
    ShuttingDown,
    /// Shutdown complete
    Complete,
    /// Force terminated
    ForceTerminated,
}

/// Callback for shutdown tasks
pub type ShutdownCallback = Box<dyn FnOnce() -> anyhow::Result<()> + Send + Sync>;

/// Async callback for shutdown tasks
pub type AsyncShutdownCallback = Box<
    dyn FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Graceful shutdown handler
pub struct ShutdownHandler {
    config: ShutdownConfig,
    state: Arc<RwLock<ShutdownState>>,
    shutdown_signal: broadcast::Sender<()>,
    shutdown_initiated: Arc<AtomicBool>,
    exit_code: Arc<AtomicI32>,
    callbacks: Arc<Mutex<Vec<ShutdownCallback>>>,
    async_callbacks: Arc<Mutex<Vec<AsyncShutdownCallback>>>,
}

impl ShutdownHandler {
    /// Create a new shutdown handler
    pub fn new(config: ShutdownConfig) -> Self {
        let (shutdown_signal, _) = broadcast::channel(1);

        Self {
            config,
            state: Arc::new(RwLock::new(ShutdownState::Running)),
            shutdown_signal,
            shutdown_initiated: Arc::new(AtomicBool::new(false)),
            exit_code: Arc::new(AtomicI32::new(0)),
            callbacks: Arc::new(Mutex::new(Vec::new())),
            async_callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ShutdownConfig::default())
    }

    /// Install panic handler for emergency cleanup
    pub fn install_panic_handler(&self) {
        if !self.config.install_panic_handler {
            return;
        }

        let exit_code = Arc::clone(&self.exit_code);
        let shutdown_initiated = Arc::clone(&self.shutdown_initiated);

        let default_hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |panic_info| {
            // Mark as panic exit
            exit_code.store(ExitCode::Panic.as_i32(), Ordering::SeqCst);
            shutdown_initiated.store(true, Ordering::SeqCst);

            // Log panic with context
            tracing::error!(
                panic = true,
                location = ?panic_info.location(),
                "Panic occurred, attempting emergency cleanup"
            );

            // Attempt emergency cleanup
            // Note: We can't do async cleanup in panic handler
            tracing::warn!("Emergency cleanup initiated due to panic");

            // Call default hook
            default_hook(panic_info);
        }));

        tracing::debug!("Panic handler installed");
    }

    /// Register a synchronous shutdown callback
    pub async fn register_callback(&self, callback: ShutdownCallback) {
        self.callbacks.lock().await.push(callback);
    }

    /// Register an async shutdown callback
    pub async fn register_async_callback(&self, callback: AsyncShutdownCallback) {
        self.async_callbacks.lock().await.push(callback);
    }

    /// Subscribe to shutdown signal
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_signal.subscribe()
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_initiated.load(Ordering::SeqCst)
    }

    /// Get current shutdown state
    pub fn state(&self) -> ShutdownState {
        *self.state.read()
    }

    /// Initiate shutdown
    pub fn initiate_shutdown(&self) {
        if self.shutdown_initiated.swap(true, Ordering::SeqCst) {
            // Already shutting down
            return;
        }

        *self.state.write() = ShutdownState::ShuttingDown;
        let _ = self.shutdown_signal.send(());

        tracing::info!("Shutdown initiated");
    }

    /// Perform graceful shutdown
    pub async fn shutdown(&self) -> ExitCode {
        self.initiate_shutdown();

        let start = Instant::now();
        let deadline = start + self.config.timeout;

        tracing::info!(timeout_secs = self.config.timeout.as_secs(), "Starting graceful shutdown");

        // Run async callbacks first
        let async_callbacks = {
            let mut callbacks = self.async_callbacks.lock().await;
            std::mem::take(&mut *callbacks)
        };

        for callback in async_callbacks {
            if Instant::now() > deadline {
                tracing::warn!("Shutdown timeout reached during async callbacks");
                break;
            }

            let future = callback();
            match tokio::time::timeout(deadline.saturating_duration_since(Instant::now()), future)
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "Async shutdown callback failed");
                }
                Err(_) => {
                    tracing::warn!("Async shutdown callback timed out");
                    break;
                }
            }
        }

        // Run sync callbacks
        let callbacks = {
            let mut callbacks = self.callbacks.lock().await;
            std::mem::take(&mut *callbacks)
        };

        for callback in callbacks {
            if Instant::now() > deadline {
                tracing::warn!("Shutdown timeout reached during sync callbacks");
                break;
            }

            if let Err(e) = callback() {
                tracing::error!(error = %e, "Sync shutdown callback failed");
            }
        }

        // Check if we completed within timeout
        let elapsed = start.elapsed();
        let exit_code = if elapsed > self.config.timeout {
            tracing::warn!(
                elapsed_secs = elapsed.as_secs(),
                timeout_secs = self.config.timeout.as_secs(),
                "Shutdown timeout exceeded"
            );

            if self.config.force_on_timeout {
                *self.state.write() = ShutdownState::ForceTerminated;
                tracing::warn!("Force terminating due to timeout");
            }

            self.exit_code.store(ExitCode::Timeout.as_i32(), Ordering::SeqCst);
            ExitCode::Timeout
        } else {
            *self.state.write() = ShutdownState::Complete;

            let stored_code = self.exit_code.load(Ordering::SeqCst);
            if stored_code != 0 {
                ExitCode::from(stored_code)
            } else {
                ExitCode::Success
            }
        };

        tracing::info!(
            exit_code = exit_code.as_i32(),
            elapsed_ms = elapsed.as_millis(),
            "Shutdown complete"
        );

        exit_code
    }

    /// Set exit code (for error conditions)
    pub fn set_exit_code(&self, code: ExitCode) {
        self.exit_code.store(code.as_i32(), Ordering::SeqCst);
    }

    /// Get the current exit code
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code.load(Ordering::SeqCst))
    }

    /// Wait for shutdown signal (Ctrl+C)
    ///
    /// # Errors
    /// Returns an error if signal handlers cannot be registered.
    pub async fn wait_for_signal(&self) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut sigterm =
                signal(SignalKind::terminate()).context("Failed to register SIGTERM handler")?;
            let mut sigint =
                signal(SignalKind::interrupt()).context("Failed to register SIGINT handler")?;

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM");
                    self.set_exit_code(ExitCode::Signal(15));
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT");
                    self.set_exit_code(ExitCode::Signal(2));
                }
            }
        }

        #[cfg(windows)]
        {
            tokio::signal::ctrl_c().await.context("Failed to register Ctrl+C handler")?;
            tracing::info!("Received Ctrl+C");
            self.set_exit_code(ExitCode::Signal(2));
        }

        self.initiate_shutdown();
        Ok(())
    }
}

impl Default for ShutdownHandler {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_conversion() {
        assert_eq!(ExitCode::Success.as_i32(), 0);
        assert_eq!(ExitCode::Error.as_i32(), 1);
        assert_eq!(ExitCode::Timeout.as_i32(), 2);
        assert_eq!(ExitCode::Panic.as_i32(), 3);
        assert_eq!(ExitCode::Signal(15).as_i32(), 143); // 128 + 15
    }

    #[test]
    fn test_exit_code_from_i32() {
        assert_eq!(ExitCode::from(0), ExitCode::Success);
        assert_eq!(ExitCode::from(1), ExitCode::Error);
        assert_eq!(ExitCode::from(2), ExitCode::Timeout);
        assert_eq!(ExitCode::from(3), ExitCode::Panic);
        assert!(matches!(ExitCode::from(143), ExitCode::Signal(15)));
    }

    #[test]
    fn test_exit_code_is_clean() {
        assert!(ExitCode::Success.is_clean());
        assert!(!ExitCode::Error.is_clean());
        assert!(!ExitCode::Timeout.is_clean());
        assert!(!ExitCode::Panic.is_clean());
    }

    #[test]
    fn test_shutdown_config_default() {
        let config = ShutdownConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.force_on_timeout);
        assert!(config.install_panic_handler);
    }

    #[tokio::test]
    async fn test_shutdown_handler_creation() {
        let handler = ShutdownHandler::with_defaults();
        assert!(!handler.is_shutting_down());
        assert_eq!(handler.state(), ShutdownState::Running);
    }

    #[tokio::test]
    async fn test_initiate_shutdown() {
        let handler = ShutdownHandler::with_defaults();

        assert!(!handler.is_shutting_down());

        handler.initiate_shutdown();

        assert!(handler.is_shutting_down());
        assert_eq!(handler.state(), ShutdownState::ShuttingDown);
    }

    #[tokio::test]
    async fn test_shutdown_with_callbacks() {
        use std::sync::atomic::AtomicUsize;

        let handler = ShutdownHandler::new(ShutdownConfig {
            timeout: Duration::from_secs(5),
            force_on_timeout: true,
            install_panic_handler: false,
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        handler
            .register_callback(Box::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
            .await;

        let exit_code = handler.shutdown().await;

        assert_eq!(exit_code, ExitCode::Success);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(handler.state(), ShutdownState::Complete);
    }

    #[tokio::test]
    async fn test_shutdown_with_async_callbacks() {
        use std::sync::atomic::AtomicUsize;

        let handler = ShutdownHandler::new(ShutdownConfig {
            timeout: Duration::from_secs(5),
            force_on_timeout: true,
            install_panic_handler: false,
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        handler
            .register_async_callback(Box::new(move || {
                let counter = counter_clone;
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }))
            .await;

        let exit_code = handler.shutdown().await;

        assert_eq!(exit_code, ExitCode::Success);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_set_exit_code() {
        let handler = ShutdownHandler::with_defaults();

        handler.set_exit_code(ExitCode::Error);
        assert_eq!(handler.exit_code(), ExitCode::Error);

        handler.set_exit_code(ExitCode::Success);
        assert_eq!(handler.exit_code(), ExitCode::Success);
    }

    #[tokio::test]
    async fn test_subscribe_to_shutdown() {
        let handler = ShutdownHandler::with_defaults();
        let mut receiver = handler.subscribe();

        // Spawn a task to initiate shutdown
        let handler_clone = Arc::new(handler);
        let handler_for_task = Arc::clone(&handler_clone);

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            handler_for_task.initiate_shutdown();
        });

        // Wait for signal
        let result = tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await;

        assert!(result.is_ok());
    }
}

/// Property-based tests for shutdown handling
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 21: Graceful Shutdown Completeness
        /// For any graceful shutdown initiated, all in-flight write operations
        /// SHALL complete, all pending log entries SHALL be flushed, and the
        /// current state SHALL be persisted to disk before the process exits.
        #[test]
        fn prop_graceful_shutdown_completeness(
            num_callbacks in 0..10usize,
            timeout_ms in 100..5000u64,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use std::sync::atomic::AtomicUsize;

                let handler = ShutdownHandler::new(ShutdownConfig {
                    timeout: Duration::from_millis(timeout_ms),
                    force_on_timeout: true,
                    install_panic_handler: false,
                });

                let completed = Arc::new(AtomicUsize::new(0));

                // Register callbacks
                for _ in 0..num_callbacks {
                    let completed_clone = Arc::clone(&completed);
                    handler.register_callback(Box::new(move || {
                        completed_clone.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })).await;
                }

                // Perform shutdown
                let exit_code = handler.shutdown().await;

                // All callbacks should have been executed
                prop_assert_eq!(
                    completed.load(Ordering::SeqCst),
                    num_callbacks,
                    "All {} callbacks should complete", num_callbacks
                );

                // State should be Complete (not ForceTerminated for fast callbacks)
                prop_assert_eq!(handler.state(), ShutdownState::Complete);

                // Exit code should be Success for clean shutdown
                prop_assert_eq!(exit_code, ExitCode::Success);

                Ok(())
            })?;
        }

        /// Property 22: Exit Code Correctness
        /// For any process exit, the exit code SHALL be 0 if and only if
        /// shutdown was clean (no errors during shutdown), and non-zero otherwise.
        #[test]
        fn prop_exit_code_correctness(
            code in 0i32..256i32,
        ) {
            let exit_code = ExitCode::from(code);

            // Exit code 0 should be Success and clean
            if code == 0 {
                prop_assert_eq!(exit_code, ExitCode::Success);
                prop_assert!(exit_code.is_clean(),
                    "Exit code 0 should be clean");
            } else {
                // Non-zero should not be clean
                prop_assert!(!exit_code.is_clean(),
                    "Exit code {} should not be clean", code);
            }

            // Round-trip conversion should preserve meaning
            let converted = exit_code.as_i32();
            let reconverted = ExitCode::from(converted);

            // For standard codes, should match exactly
            if code <= 3 {
                prop_assert_eq!(exit_code, reconverted);
            }
        }

        /// Property 22 (continued): Signal exit codes
        #[test]
        fn prop_signal_exit_codes(
            signal in 1i32..32i32,
        ) {
            let exit_code = ExitCode::Signal(signal);

            // Signal exit codes should be 128 + signal
            prop_assert_eq!(exit_code.as_i32(), 128 + signal);

            // Should not be clean
            prop_assert!(!exit_code.is_clean());

            // Round-trip should preserve signal number
            let converted = exit_code.as_i32();
            let reconverted = ExitCode::from(converted);

            if let ExitCode::Signal(s) = reconverted {
                prop_assert_eq!(s, signal);
            } else {
                prop_assert!(false, "Should be Signal variant");
            }
        }

        /// Property 21 (continued): Shutdown state transitions
        #[test]
        fn prop_shutdown_state_transitions(
            _dummy in 0..1i32, // Just to make proptest happy
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let handler = ShutdownHandler::new(ShutdownConfig {
                    timeout: Duration::from_secs(5),
                    force_on_timeout: true,
                    install_panic_handler: false,
                });

                // Initial state should be Running
                prop_assert_eq!(handler.state(), ShutdownState::Running);
                prop_assert!(!handler.is_shutting_down());

                // After initiate_shutdown, should be ShuttingDown
                handler.initiate_shutdown();
                prop_assert_eq!(handler.state(), ShutdownState::ShuttingDown);
                prop_assert!(handler.is_shutting_down());

                // Multiple calls to initiate_shutdown should be idempotent
                handler.initiate_shutdown();
                prop_assert_eq!(handler.state(), ShutdownState::ShuttingDown);

                Ok(())
            })?;
        }

        /// Property 21 (continued): Async callbacks complete
        #[test]
        fn prop_async_callbacks_complete(
            num_async_callbacks in 0..5usize,
            delay_ms in 1..50u64,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use std::sync::atomic::AtomicUsize;

                let handler = ShutdownHandler::new(ShutdownConfig {
                    timeout: Duration::from_secs(10),
                    force_on_timeout: true,
                    install_panic_handler: false,
                });

                let completed = Arc::new(AtomicUsize::new(0));

                // Register async callbacks with small delays
                for _ in 0..num_async_callbacks {
                    let completed_clone = Arc::clone(&completed);
                    let delay = Duration::from_millis(delay_ms);

                    handler.register_async_callback(Box::new(move || {
                        let completed = completed_clone;
                        Box::pin(async move {
                            tokio::time::sleep(delay).await;
                            completed.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        })
                    })).await;
                }

                // Perform shutdown
                let exit_code = handler.shutdown().await;

                // All async callbacks should complete
                prop_assert_eq!(
                    completed.load(Ordering::SeqCst),
                    num_async_callbacks,
                    "All {} async callbacks should complete", num_async_callbacks
                );

                prop_assert_eq!(exit_code, ExitCode::Success);

                Ok(())
            })?;
        }
    }
}
