//! WhatsApp Business API configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// WhatsApp account type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountType {
    /// Official Business API (requires Meta approval)
    Business,
    /// Personal account via gateway (requires local gateway service)
    Personal,
}

/// WhatsApp configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Account type
    pub account_type: AccountType,

    // Business API fields
    /// WhatsApp Business API access token
    pub access_token: Option<String>,
    /// Phone number ID (from Meta Business)
    pub phone_number_id: Option<String>,
    /// Business account ID
    pub business_account_id: Option<String>,
    /// API version (default: v21.0)
    pub api_version: String,

    // Personal account fields
    /// Gateway URL (e.g., http://localhost:3000)
    pub gateway_url: Option<String>,
    /// Gateway API key (if required)
    pub gateway_api_key: Option<String>,
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            account_type: AccountType::Business,
            access_token: None,
            phone_number_id: None,
            business_account_id: None,
            api_version: "v21.0".to_string(),
            gateway_url: None,
            gateway_api_key: None,
        }
    }
}

impl WhatsAppConfig {
    /// Load configuration from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get config path
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("whatsapp.toml")
    }

    /// Load from default location or create new
    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        Self::load(&path).unwrap_or_default()
    }
}
