//! Graceful shutdown coordination for DCP server.
//!
//! Provides shutdown coordination with in-flight request tracking,
//! configurable drain timeout, and signal handling.

mod signals;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

pub use signals::{setup_default_handlers, wait_for_ctrl_c, Signal, SignalHandler};

/// Shutdown coordinator for graceful server shutdown
pub struct ShutdownCoordinator {
    /// Shutdown signal flag
    shutdown: Arc<AtomicBool>,
    /// In-flight request counter
    in_flight: Arc<AtomicU64>,
    /// Shutdown notify channel
    notify: broadcast::Sender<()>,
    /// Drain timeout
    drain_timeout: Duration,
    /// Whether shutdown has been initiated
    initiated: AtomicBool,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new(drain_timeout: Duration) -> Self {
        let (notify, _) = broadcast::channel(16);
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
            in_flight: Arc::new(AtomicU64::new(0)),
            notify,
            drain_timeout,
            initiated: AtomicBool::new(false),
        }
    }

    /// Create with default 30 second drain timeout
    pub fn with_default_timeout() -> Self {
        Self::new(Duration::from_secs(30))
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        if self.initiated.swap(true, Ordering::SeqCst) {
            // Already initiated
            return;
        }
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = self.notify.send(());
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Get current in-flight request count
    pub fn in_flight_count(&self) -> u64 {
        self.in_flight.load(Ordering::Acquire)
    }

    /// Wait for all in-flight requests to complete with timeout
    /// Returns true if all requests completed, false if timeout
    pub async fn wait_drain(&self) -> bool {
        let deadline = Instant::now() + self.drain_timeout;

        while self.in_flight.load(Ordering::Acquire) > 0 {
            if Instant::now() > deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        true
    }

    /// Wait for drain with custom timeout
    pub async fn wait_drain_with_timeout(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;

        while self.in_flight.load(Ordering::Acquire) > 0 {
            if Instant::now() > deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        true
    }

    /// Track request start - returns a guard that decrements on drop
    pub fn request_start(&self) -> RequestGuard {
        self.in_flight.fetch_add(1, Ordering::AcqRel);
        RequestGuard {
            counter: Arc::clone(&self.in_flight),
        }
    }

    /// Manually increment in-flight counter
    pub fn increment_in_flight(&self) {
        self.in_flight.fetch_add(1, Ordering::AcqRel);
    }

    /// Manually decrement in-flight counter
    pub fn decrement_in_flight(&self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }

    /// Subscribe to shutdown notifications
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.notify.subscribe()
    }

    /// Get the drain timeout
    pub fn drain_timeout(&self) -> Duration {
        self.drain_timeout
    }

    /// Set the drain timeout
    pub fn set_drain_timeout(&mut self, timeout: Duration) {
        self.drain_timeout = timeout;
    }

    /// Create a shared reference
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::with_default_timeout()
    }
}

/// RAII guard for request tracking
/// Automatically decrements the in-flight counter when dropped
pub struct RequestGuard {
    counter: Arc<AtomicU64>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Shutdown progress for logging
#[derive(Debug, Clone)]
pub struct ShutdownProgress {
    /// Whether shutdown has been initiated
    pub initiated: bool,
    /// Current in-flight request count
    pub in_flight: u64,
    /// Time since shutdown was initiated
    pub elapsed: Option<Duration>,
    /// Drain timeout
    pub timeout: Duration,
}

impl ShutdownProgress {
    /// Format progress as a log message
    pub fn to_log_message(&self) -> String {
        if !self.initiated {
            return "Shutdown not initiated".to_string();
        }

        let elapsed_str = self
            .elapsed
            .map(|d| format!("{:.1}s", d.as_secs_f64()))
            .unwrap_or_else(|| "unknown".to_string());

        let timeout_str = format!("{:.1}s", self.timeout.as_secs_f64());

        if self.in_flight == 0 {
            format!("Shutdown complete after {}", elapsed_str)
        } else {
            format!(
                "Shutdown in progress: {} in-flight requests, elapsed: {}, timeout: {}",
                self.in_flight, elapsed_str, timeout_str
            )
        }
    }

    /// Check if shutdown is complete
    pub fn is_complete(&self) -> bool {
        self.initiated && self.in_flight == 0
    }

    /// Check if shutdown has timed out
    pub fn is_timed_out(&self) -> bool {
        if let Some(elapsed) = self.elapsed {
            elapsed >= self.timeout
        } else {
            false
        }
    }
}

/// Shutdown logger for progress reporting
pub struct ShutdownLogger {
    /// Shutdown coordinator
    coordinator: Arc<ShutdownCoordinator>,
    /// Shutdown start time
    start_time: std::sync::Mutex<Option<Instant>>,
    /// Log interval
    log_interval: Duration,
}

impl ShutdownLogger {
    /// Create a new shutdown logger
    pub fn new(coordinator: Arc<ShutdownCoordinator>, log_interval: Duration) -> Self {
        Self {
            coordinator,
            start_time: std::sync::Mutex::new(None),
            log_interval,
        }
    }

    /// Create with default 1 second log interval
    pub fn with_default_interval(coordinator: Arc<ShutdownCoordinator>) -> Self {
        Self::new(coordinator, Duration::from_secs(1))
    }

    /// Mark shutdown as started
    pub fn start(&self) {
        let mut start_time = self.start_time.lock().unwrap();
        if start_time.is_none() {
            *start_time = Some(Instant::now());
        }
    }

    /// Get current progress with elapsed time
    pub fn progress(&self) -> ShutdownProgress {
        let elapsed = self.start_time.lock().unwrap().map(|t| t.elapsed());
        ShutdownProgress {
            initiated: self.coordinator.is_shutdown(),
            in_flight: self.coordinator.in_flight_count(),
            elapsed,
            timeout: self.coordinator.drain_timeout(),
        }
    }

    /// Log current progress to stderr
    pub fn log_progress(&self) {
        let progress = self.progress();
        eprintln!("[SHUTDOWN] {}", progress.to_log_message());
    }

    /// Run periodic progress logging until shutdown completes or times out
    pub async fn run_logging(&self) {
        self.start();

        loop {
            let progress = self.progress();

            // Log progress
            eprintln!("[SHUTDOWN] {}", progress.to_log_message());

            // Check if complete or timed out
            if progress.is_complete() {
                eprintln!("[SHUTDOWN] Graceful shutdown complete");
                break;
            }

            if progress.is_timed_out() {
                eprintln!(
                    "[SHUTDOWN] Timeout reached with {} requests still in-flight",
                    progress.in_flight
                );
                break;
            }

            // Wait for next log interval
            tokio::time::sleep(self.log_interval).await;
        }
    }

    /// Get the log interval
    pub fn log_interval(&self) -> Duration {
        self.log_interval
    }
}

impl ShutdownCoordinator {
    /// Get current shutdown progress
    pub fn progress(&self) -> ShutdownProgress {
        ShutdownProgress {
            initiated: self.is_shutdown(),
            in_flight: self.in_flight_count(),
            elapsed: None, // Use ShutdownLogger for elapsed time tracking
            timeout: self.drain_timeout,
        }
    }

    /// Create a logger for this coordinator
    pub fn create_logger(self: &Arc<Self>) -> ShutdownLogger {
        ShutdownLogger::with_default_interval(Arc::clone(self))
    }

    /// Create a logger with custom interval
    pub fn create_logger_with_interval(self: &Arc<Self>, interval: Duration) -> ShutdownLogger {
        ShutdownLogger::new(Arc::clone(self), interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_coordinator_creation() {
        let coord = ShutdownCoordinator::new(Duration::from_secs(10));
        assert!(!coord.is_shutdown());
        assert_eq!(coord.in_flight_count(), 0);
    }

    #[test]
    fn test_shutdown_signal() {
        let coord = ShutdownCoordinator::default();
        assert!(!coord.is_shutdown());

        coord.shutdown();
        assert!(coord.is_shutdown());

        // Multiple shutdowns should be idempotent
        coord.shutdown();
        assert!(coord.is_shutdown());
    }

    #[test]
    fn test_request_guard() {
        let coord = ShutdownCoordinator::default();
        assert_eq!(coord.in_flight_count(), 0);

        {
            let _guard1 = coord.request_start();
            assert_eq!(coord.in_flight_count(), 1);

            {
                let _guard2 = coord.request_start();
                assert_eq!(coord.in_flight_count(), 2);
            }

            assert_eq!(coord.in_flight_count(), 1);
        }

        assert_eq!(coord.in_flight_count(), 0);
    }

    #[test]
    fn test_manual_increment_decrement() {
        let coord = ShutdownCoordinator::default();

        coord.increment_in_flight();
        coord.increment_in_flight();
        assert_eq!(coord.in_flight_count(), 2);

        coord.decrement_in_flight();
        assert_eq!(coord.in_flight_count(), 1);

        coord.decrement_in_flight();
        assert_eq!(coord.in_flight_count(), 0);
    }

    #[tokio::test]
    async fn test_wait_drain_immediate() {
        let coord = ShutdownCoordinator::new(Duration::from_millis(100));

        // No in-flight requests, should return immediately
        let result = coord.wait_drain().await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_drain_with_requests() {
        let coord = Arc::new(ShutdownCoordinator::new(Duration::from_secs(1)));
        let coord_clone = Arc::clone(&coord);

        // Start a request
        let guard = coord.request_start();

        // Spawn task to complete request after delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            drop(guard);
        });

        // Wait for drain
        let result = coord_clone.wait_drain().await;
        assert!(result);
        assert_eq!(coord_clone.in_flight_count(), 0);
    }

    #[tokio::test]
    async fn test_wait_drain_timeout() {
        let coord = ShutdownCoordinator::new(Duration::from_millis(50));

        // Start a request that won't complete
        let _guard = coord.request_start();

        // Wait for drain should timeout
        let result = coord.wait_drain().await;
        assert!(!result);
        assert_eq!(coord.in_flight_count(), 1);
    }

    #[tokio::test]
    async fn test_shutdown_notification() {
        let coord = ShutdownCoordinator::default();
        let mut rx = coord.subscribe();

        // Shutdown in another task
        let coord_clone = Arc::new(coord);
        let coord_for_shutdown = Arc::clone(&coord_clone);

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            coord_for_shutdown.shutdown();
        });

        // Wait for notification
        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown_progress() {
        let coord = ShutdownCoordinator::new(Duration::from_secs(30));

        let progress = coord.progress();
        assert!(!progress.initiated);
        assert_eq!(progress.in_flight, 0);
        assert_eq!(progress.timeout, Duration::from_secs(30));

        coord.shutdown();
        coord.increment_in_flight();

        let progress = coord.progress();
        assert!(progress.initiated);
        assert_eq!(progress.in_flight, 1);
    }
}
