//! Signal handling for graceful shutdown.
//!
//! Provides cross-platform signal handling for SIGTERM, SIGINT, and SIGHUP.

use std::sync::Arc;
use tokio::sync::broadcast;

use super::ShutdownCoordinator;

/// Signal types that can be handled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// SIGTERM - graceful shutdown
    Term,
    /// SIGINT - interrupt (Ctrl+C)
    Int,
    /// SIGHUP - reload configuration
    Hup,
}

/// Signal handler configuration
pub struct SignalHandler {
    /// Shutdown coordinator
    coordinator: Arc<ShutdownCoordinator>,
    /// Config reload channel
    reload_tx: broadcast::Sender<()>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new(coordinator: Arc<ShutdownCoordinator>) -> Self {
        let (reload_tx, _) = broadcast::channel(16);
        Self {
            coordinator,
            reload_tx,
        }
    }

    /// Subscribe to config reload notifications
    pub fn subscribe_reload(&self) -> broadcast::Receiver<()> {
        self.reload_tx.subscribe()
    }

    /// Trigger a config reload
    pub fn trigger_reload(&self) {
        let _ = self.reload_tx.send(());
    }

    /// Get the shutdown coordinator
    pub fn coordinator(&self) -> &Arc<ShutdownCoordinator> {
        &self.coordinator
    }

    /// Setup signal handlers (Unix)
    #[cfg(unix)]
    pub async fn setup_handlers(self: Arc<Self>) {
        use tokio::signal::unix::{signal, SignalKind};

        let handler = Arc::clone(&self);

        // SIGTERM handler
        let term_handler = Arc::clone(&handler);
        tokio::spawn(async move {
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");
            loop {
                sigterm.recv().await;
                log_signal(Signal::Term);
                term_handler.coordinator.shutdown();
            }
        });

        // SIGINT handler (Ctrl+C)
        let int_handler = Arc::clone(&handler);
        tokio::spawn(async move {
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");
            loop {
                sigint.recv().await;
                log_signal(Signal::Int);
                int_handler.coordinator.shutdown();
            }
        });

        // SIGHUP handler (config reload)
        let hup_handler = Arc::clone(&handler);
        tokio::spawn(async move {
            let mut sighup = signal(SignalKind::hangup()).expect("Failed to setup SIGHUP handler");
            loop {
                sighup.recv().await;
                log_signal(Signal::Hup);
                hup_handler.trigger_reload();
            }
        });
    }

    /// Setup signal handlers (Windows)
    #[cfg(windows)]
    pub async fn setup_handlers(self: Arc<Self>) {
        use tokio::signal::ctrl_c;

        let handler = Arc::clone(&self);

        // Ctrl+C handler (equivalent to SIGINT)
        tokio::spawn(async move {
            loop {
                if ctrl_c().await.is_ok() {
                    log_signal(Signal::Int);
                    handler.coordinator.shutdown();
                }
            }
        });
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.coordinator.subscribe();
        let _ = rx.recv().await;
    }
}

/// Log signal receipt
fn log_signal(signal: Signal) {
    match signal {
        Signal::Term => eprintln!("Received SIGTERM, initiating graceful shutdown..."),
        Signal::Int => eprintln!("Received SIGINT, initiating graceful shutdown..."),
        Signal::Hup => eprintln!("Received SIGHUP, reloading configuration..."),
    }
}

/// Setup default signal handlers
pub async fn setup_default_handlers(coordinator: Arc<ShutdownCoordinator>) -> Arc<SignalHandler> {
    let handler = Arc::new(SignalHandler::new(coordinator));
    handler.clone().setup_handlers().await;
    handler
}

/// Convenience function to wait for Ctrl+C
pub async fn wait_for_ctrl_c() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");

        tokio::select! {
            _ = sigint.recv() => {
                log_signal(Signal::Int);
            }
            _ = sigterm.recv() => {
                log_signal(Signal::Term);
            }
        }
    }

    #[cfg(windows)]
    {
        use tokio::signal::ctrl_c;
        let _ = ctrl_c().await;
        log_signal(Signal::Int);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_signal_handler_creation() {
        let coord = Arc::new(ShutdownCoordinator::default());
        let handler = SignalHandler::new(coord);

        assert!(!handler.coordinator().is_shutdown());
    }

    #[tokio::test]
    async fn test_reload_subscription() {
        let coord = Arc::new(ShutdownCoordinator::default());
        let handler = SignalHandler::new(coord);

        let mut rx = handler.subscribe_reload();

        // Trigger reload
        handler.trigger_reload();

        // Should receive notification
        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_shutdown() {
        let coord = Arc::new(ShutdownCoordinator::default());
        let handler = Arc::new(SignalHandler::new(Arc::clone(&coord)));

        let handler_clone = Arc::clone(&handler);

        // Spawn task to trigger shutdown
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            handler_clone.coordinator().shutdown();
        });

        // Wait for shutdown
        let result =
            tokio::time::timeout(Duration::from_millis(100), handler.wait_for_shutdown()).await;
        assert!(result.is_ok());
    }
}
