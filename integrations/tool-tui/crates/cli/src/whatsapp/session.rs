//! WhatsApp session management (for Business API, this is just config management)

use anyhow::Result;
use std::path::PathBuf;

use super::config::WhatsAppConfig;

/// Session manager for WhatsApp Business API
pub struct SessionManager {
    config_path: PathBuf,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            config_path: WhatsAppConfig::config_path(),
        }
    }

    /// Check if configuration exists
    pub fn has_config(&self) -> bool {
        self.config_path.exists()
    }

    /// Load configuration
    pub fn load_config(&self) -> Result<WhatsAppConfig> {
        WhatsAppConfig::load(&self.config_path)
    }

    /// Save configuration
    pub fn save_config(&self, config: &WhatsAppConfig) -> Result<()> {
        config.save(&self.config_path)
    }

    /// Clear configuration
    pub fn clear_config(&self) -> Result<()> {
        if self.config_path.exists() {
            std::fs::remove_file(&self.config_path)?;
        }
        Ok(())
    }

    /// Get config path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
