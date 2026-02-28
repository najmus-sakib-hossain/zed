//! Background daemon for automatic updates

use super::{UpdateApplier, UpdateChecker, UpdateDownloader};
use crate::utils::error::DxError;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

/// Update daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Check interval in seconds
    pub check_interval: u64,
    /// Auto-apply updates without confirmation
    pub auto_apply: bool,
    /// Use DX API instead of GitHub
    pub use_dx_api: bool,
    /// State file path
    pub state_file: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let state_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx");
        std::fs::create_dir_all(&state_dir).ok();

        Self {
            check_interval: 3600, // 1 hour
            auto_apply: true,
            use_dx_api: true,
            state_file: state_dir.join("update_daemon.json"),
        }
    }
}

/// Update daemon state
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DaemonState {
    last_check: u64,
    last_update: Option<u64>,
    current_version: String,
}

/// Background update daemon
pub struct UpdateDaemon {
    config: DaemonConfig,
    checker: UpdateChecker,
}

impl UpdateDaemon {
    /// Create a new update daemon
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            checker: UpdateChecker::new(),
            config,
        }
    }

    /// Run the daemon (blocking)
    pub async fn run(&self) -> Result<(), DxError> {
        loop {
            if let Err(e) = self.check_and_update().await {
                eprintln!("Update check failed: {}", e);
            }
            sleep(Duration::from_secs(self.config.check_interval)).await;
        }
    }

    /// Check for updates and apply if available
    async fn check_and_update(&self) -> Result<(), DxError> {
        let state = self.load_state()?;
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        // Check if enough time has passed
        if now - state.last_check < self.config.check_interval {
            return Ok(());
        }

        // Check for updates
        let update_info = match self.checker.check().await? {
            Some(info) => info,
            None => {
                self.save_state(&DaemonState {
                    last_check: now,
                    last_update: state.last_update,
                    current_version: state.current_version,
                })?;
                return Ok(());
            }
        };

        // Auto-apply if enabled
        if self.config.auto_apply {
            self.apply_update(&update_info).await?;
            self.save_state(&DaemonState {
                last_check: now,
                last_update: Some(now),
                current_version: update_info.new_version.clone(),
            })?;
        }

        Ok(())
    }

    async fn apply_update(&self, info: &super::UpdateInfo) -> Result<(), DxError> {
        let downloader = UpdateDownloader::new()?;
        let binary = downloader.download(info)?;
        let signature = downloader.download_signature(&info.signature)?;

        // TODO: Load public key from embedded or config
        let public_key = vec![0u8; 32]; // Placeholder

        let applier = UpdateApplier::for_current_exe()?;
        applier.apply_update(&binary, &signature, &public_key)?;

        Ok(())
    }

    fn load_state(&self) -> Result<DaemonState, DxError> {
        if !self.config.state_file.exists() {
            return Ok(DaemonState {
                last_check: 0,
                last_update: None,
                current_version: self.checker.current_version().to_string(),
            });
        }

        let content =
            std::fs::read_to_string(&self.config.state_file).map_err(|e| DxError::Io {
                message: format!("Failed to read state file: {}", e),
            })?;

        serde_json::from_str(&content).map_err(|e| DxError::Io {
            message: format!("Failed to parse state file: {}", e),
        })
    }

    fn save_state(&self, state: &DaemonState) -> Result<(), DxError> {
        let content = serde_json::to_string_pretty(state).map_err(|e| DxError::Io {
            message: format!("Failed to serialize state: {}", e),
        })?;

        std::fs::write(&self.config.state_file, content).map_err(|e| DxError::Io {
            message: format!("Failed to write state file: {}", e),
        })
    }
}

/// Daemon manager for starting/stopping the update daemon
pub struct DaemonManager {
    config: DaemonConfig,
}

impl DaemonManager {
    /// Create a new daemon manager
    pub fn new(config: DaemonConfig) -> Self {
        Self { config }
    }

    /// Start the daemon in the background
    pub async fn start(&self) -> Result<(), DxError> {
        let daemon = UpdateDaemon::new(self.config.clone());

        // Spawn daemon in background
        tokio::spawn(async move {
            if let Err(e) = daemon.run().await {
                eprintln!("Daemon error: {}", e);
            }
        });

        Ok(())
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        // TODO: Implement proper daemon status check
        false
    }

    /// Stop the daemon
    pub fn stop(&self) -> Result<(), DxError> {
        // TODO: Implement daemon stop
        Ok(())
    }
}
