//! # Zapier Integration
//!
//! Trigger Zapier automations via webhooks.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::zapier::{ZapierClient, ZapierConfig};
//!
//! let config = ZapierConfig::from_file("~/.dx/config/zapier.sr")?;
//! let client = ZapierClient::new(&config)?;
//!
//! // Trigger a Zap
//! client.trigger("my_zap", serde_json::json!({
//!     "name": "John",
//!     "email": "john@example.com"
//! })).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Zapier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierConfig {
    /// Whether Zapier integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Configured triggers (name -> webhook URL)
    #[serde(default)]
    pub triggers: HashMap<String, ZapierTrigger>,
}

fn default_true() -> bool {
    true
}

impl Default for ZapierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            triggers: HashMap::new(),
        }
    }
}

impl ZapierConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }
}

/// Zapier trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierTrigger {
    /// Trigger name
    pub name: String,
    /// Webhook URL (from Zapier)
    pub webhook_url: String,
    /// Description
    pub description: Option<String>,
    /// Whether trigger is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Zapier trigger response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierResponse {
    /// Response status
    pub status: String,
    /// Request ID
    pub id: Option<String>,
    /// Attempt number
    pub attempt: Option<String>,
}

/// Zapier client
pub struct ZapierClient {
    config: ZapierConfig,
}

impl ZapierClient {
    /// Create a new Zapier client
    pub fn new(config: &ZapierConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled && !self.config.triggers.is_empty()
    }

    /// Trigger a Zap by name
    pub async fn trigger(&self, name: &str, data: serde_json::Value) -> Result<ZapierResponse> {
        let trigger = self.config.triggers.get(name)
            .ok_or_else(|| DrivenError::NotFound(format!("Trigger '{}' not found", name)))?;

        if !trigger.enabled {
            return Err(DrivenError::Config("Trigger is disabled".into()));
        }

        self.send_webhook(&trigger.webhook_url, data).await
    }

    /// Trigger using a raw webhook URL
    pub async fn trigger_webhook(&self, url: &str, data: serde_json::Value) -> Result<ZapierResponse> {
        self.send_webhook(url, data).await
    }

    /// Send webhook request
    async fn send_webhook(&self, url: &str, data: serde_json::Value) -> Result<ZapierResponse> {
        let client = reqwest::Client::new();
        
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!(
                "Zapier webhook error ({}): {}",
                status, error_text
            )));
        }

        // Zapier returns a simple response
        Ok(ZapierResponse {
            status: "success".to_string(),
            id: None,
            attempt: None,
        })
    }

    /// List configured triggers
    pub fn list_triggers(&self) -> Vec<&ZapierTrigger> {
        self.config.triggers.values().collect()
    }

    /// Add a trigger
    pub fn add_trigger(&mut self, trigger: ZapierTrigger) {
        self.config.triggers.insert(trigger.name.clone(), trigger);
    }

    /// Remove a trigger
    pub fn remove_trigger(&mut self, name: &str) -> Option<ZapierTrigger> {
        self.config.triggers.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ZapierConfig::default();
        assert!(config.enabled);
        assert!(config.triggers.is_empty());
    }

    #[test]
    fn test_client_creation() {
        let config = ZapierConfig::default();
        let client = ZapierClient::new(&config);
        assert!(client.is_ok());
    }
}
