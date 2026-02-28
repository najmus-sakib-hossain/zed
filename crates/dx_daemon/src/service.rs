//! Daemon service — manages the background agent lifecycle.

use serde::{Deserialize, Serialize};

/// State of the daemon process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Error,
}

/// Configuration for the daemon service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Whether to start on system boot.
    pub autostart: bool,
    /// Port for the local IPC socket.
    pub ipc_port: u16,
    /// Max memory budget in MB.
    pub max_memory_mb: u64,
    /// Log file path.
    pub log_path: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            autostart: false,
            ipc_port: 42069,
            max_memory_mb: 512,
            log_path: None,
        }
    }
}

/// The background daemon service.
pub struct DaemonService {
    state: DaemonState,
    config: DaemonConfig,
}

impl DaemonService {
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            state: DaemonState::Stopped,
            config,
        }
    }

    pub fn state(&self) -> DaemonState {
        self.state
    }

    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    /// Start the daemon.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.state == DaemonState::Running {
            return Ok(());
        }
        log::info!("Starting DX daemon on port {}", self.config.ipc_port);
        self.state = DaemonState::Starting;
        // Placeholder — real implementation spawns the background process
        self.state = DaemonState::Running;
        Ok(())
    }

    /// Stop the daemon.
    pub fn stop(&mut self) -> anyhow::Result<()> {
        log::info!("Stopping DX daemon");
        self.state = DaemonState::Stopping;
        // Placeholder
        self.state = DaemonState::Stopped;
        Ok(())
    }

    /// Pause the daemon (stop processing, keep alive).
    pub fn pause(&mut self) {
        if self.state == DaemonState::Running {
            self.state = DaemonState::Paused;
        }
    }

    /// Resume from paused state.
    pub fn resume(&mut self) {
        if self.state == DaemonState::Paused {
            self.state = DaemonState::Running;
        }
    }

    /// Install the daemon as a system service.
    pub fn install_service(&self) -> anyhow::Result<()> {
        log::info!("Installing DX daemon as system service");
        #[cfg(target_os = "linux")]
        {
            log::info!("Would create systemd unit file");
        }
        #[cfg(target_os = "macos")]
        {
            log::info!("Would create launchd plist");
        }
        #[cfg(target_os = "windows")]
        {
            log::info!("Would register Windows Service");
        }
        Ok(())
    }
}
