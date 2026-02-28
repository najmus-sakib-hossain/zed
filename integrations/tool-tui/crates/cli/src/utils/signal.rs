//! Signal handling for the DX CLI
//!
//! Provides graceful shutdown handling for Unix and Windows.
//! - Requirement 11.5: Handle SIGINT/SIGTERM on Unix, Ctrl+C on Windows
//!
//! Uses atomic counter for signal tracking:
//! - First signal (count=0) triggers graceful shutdown
//! - Subsequent signals force immediate exit

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Global counter for received shutdown signals
/// - 0: No signals received
/// - 1: First signal, graceful shutdown in progress  
/// - 2+: Multiple signals, force exit
static SIGNAL_COUNT: AtomicU32 = AtomicU32::new(0);

/// Check if shutdown has been requested (counter > 0)
pub fn is_shutdown_requested() -> bool {
    SIGNAL_COUNT.load(Ordering::SeqCst) > 0
}

/// Get the current signal count
pub fn signal_count() -> u32 {
    SIGNAL_COUNT.load(Ordering::SeqCst)
}

/// Request shutdown - increments signal counter
/// Returns the previous count (useful for determining first vs subsequent signals)
pub fn request_shutdown() -> u32 {
    SIGNAL_COUNT.fetch_add(1, Ordering::SeqCst)
}

/// Reset shutdown counter to 0 (useful for tests)
pub fn reset_shutdown() {
    SIGNAL_COUNT.store(0, Ordering::SeqCst);
}

/// Signal handler callback type
pub type SignalCallback = Arc<dyn Fn() + Send + Sync>;

/// Setup signal handlers for graceful shutdown
///
/// Requirement 11.5: Handle SIGINT/SIGTERM on Unix, Ctrl+C on Windows
///
/// Behavior:
/// - First signal: Logs graceful shutdown message, sets flag
/// - Subsequent signals: Force immediate exit with code 130
///
/// Returns Ok(()) if handlers were set up successfully.
pub fn setup_signal_handlers<F>(callback: F) -> Result<(), ctrlc::Error>
where
    F: Fn() + Send + Sync + 'static,
{
    ctrlc::set_handler(move || {
        let previous_count = request_shutdown();

        if previous_count == 0 {
            // First signal - graceful shutdown
            eprintln!("\nReceived interrupt signal, initiating graceful shutdown...");
            eprintln!("Press Ctrl+C again to force exit.");
            callback();
        } else {
            // Subsequent signal - force exit
            eprintln!("\nForce exit requested.");
            std::process::exit(130); // 128 + SIGINT (2)
        }
    })
}

/// Setup default signal handlers that just set the shutdown flag
pub fn setup_default_handlers() -> Result<(), ctrlc::Error> {
    setup_signal_handlers(|| {})
}

#[cfg(unix)]
mod unix {
    /// Check if running on Unix
    pub fn is_unix() -> bool {
        true
    }

    /// Get the signal name for display
    pub fn signal_name(sig: i32) -> &'static str {
        match sig {
            2 => "SIGINT",
            15 => "SIGTERM",
            _ => "UNKNOWN",
        }
    }
}

#[cfg(windows)]
mod windows {
    /// Check if running on Windows
    pub fn is_windows() -> bool {
        true
    }

    /// Get the signal name for display
    pub fn signal_name(_sig: i32) -> &'static str {
        "Ctrl+C"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_flag() {
        reset_shutdown();
        assert!(!is_shutdown_requested());
        assert_eq!(signal_count(), 0);

        request_shutdown();
        assert!(is_shutdown_requested());
        assert_eq!(signal_count(), 1);

        reset_shutdown();
        assert!(!is_shutdown_requested());
        assert_eq!(signal_count(), 0);
    }

    #[test]
    fn test_shutdown_counter_increments() {
        reset_shutdown();

        // First signal returns 0 (previous count)
        assert_eq!(request_shutdown(), 0);
        assert_eq!(signal_count(), 1);

        // Second signal returns 1 (previous count)
        assert_eq!(request_shutdown(), 1);
        assert_eq!(signal_count(), 2);

        // Third signal returns 2 (previous count)
        assert_eq!(request_shutdown(), 2);
        assert_eq!(signal_count(), 3);

        reset_shutdown();
    }

    #[test]
    fn test_shutdown_atomic() {
        reset_shutdown();

        // Simulate multiple requests
        for i in 0..10 {
            assert_eq!(request_shutdown(), i);
        }

        assert!(is_shutdown_requested());
        assert_eq!(signal_count(), 10);
        reset_shutdown();
    }

    #[test]
    fn test_first_signal_triggers_graceful_shutdown() {
        reset_shutdown();

        // First signal should return 0, indicating graceful shutdown should start
        let previous = request_shutdown();
        assert_eq!(previous, 0, "First signal should have previous count of 0");
        assert!(is_shutdown_requested());

        reset_shutdown();
    }

    #[test]
    fn test_subsequent_signals_indicate_force_exit() {
        reset_shutdown();

        // First signal
        let first = request_shutdown();
        assert_eq!(first, 0);

        // Subsequent signals should return > 0, indicating force exit
        let second = request_shutdown();
        assert!(second > 0, "Second signal should indicate force exit");

        let third = request_shutdown();
        assert!(third > 0, "Third signal should indicate force exit");

        reset_shutdown();
    }
}
