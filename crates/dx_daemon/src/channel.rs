//! Channel router — routes AI responses to external channels
//! (Telegram, Discord, Slack, Email, SMS, WhatsApp).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    Email,
    Sms,
    WhatsApp,
    Webhook,
}

impl ChannelType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Telegram => "Telegram",
            Self::Discord => "Discord",
            Self::Slack => "Slack",
            Self::Email => "Email",
            Self::Sms => "SMS",
            Self::WhatsApp => "WhatsApp",
            Self::Webhook => "Webhook",
        }
    }
}

/// Configuration for a single channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub channel_type: ChannelType,
    pub name: String,
    /// API token or credentials.
    pub token: Option<String>,
    /// Channel/chat ID for the target.
    pub target_id: Option<String>,
    /// Webhook URL.
    pub webhook_url: Option<String>,
    pub enabled: bool,
}

/// A message to send or received from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub channel_id: String,
    pub channel_type: ChannelType,
    pub text: String,
    pub sender: Option<String>,
    pub timestamp: Option<std::time::SystemTime>,
    /// Optional attachment URLs.
    pub attachments: Vec<String>,
}

/// Routes messages to the appropriate channel.
pub struct ChannelRouter {
    channels: HashMap<String, ChannelConfig>,
}

impl ChannelRouter {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Register a channel.
    pub fn register(&mut self, config: ChannelConfig) {
        self.channels.insert(config.id.clone(), config);
    }

    /// Remove a channel.
    pub fn unregister(&mut self, id: &str) -> Option<ChannelConfig> {
        self.channels.remove(id)
    }

    /// Get all registered channels.
    pub fn channels(&self) -> impl Iterator<Item = &ChannelConfig> {
        self.channels.values()
    }

    /// Send a message to a specific channel.
    pub async fn send(&self, channel_id: &str, text: &str) -> Result<()> {
        let config = self
            .channels
            .get(channel_id)
            .ok_or_else(|| anyhow::anyhow!("Channel not found: {}", channel_id))?;

        if !config.enabled {
            return Err(anyhow::anyhow!("Channel {} is disabled", channel_id));
        }

        match config.channel_type {
            ChannelType::Telegram => self.send_telegram(config, text).await,
            ChannelType::Discord => self.send_discord(config, text).await,
            ChannelType::Slack => self.send_slack(config, text).await,
            ChannelType::Email => self.send_email(config, text).await,
            ChannelType::Webhook => self.send_webhook(config, text).await,
            _ => {
                log::warn!(
                    "Channel type {:?} not yet implemented",
                    config.channel_type
                );
                Ok(())
            }
        }
    }

    /// Broadcast to all enabled channels.
    pub async fn broadcast(&self, text: &str) -> Vec<(String, Result<()>)> {
        let mut results = Vec::new();
        for (id, _config) in &self.channels {
            let result = self.send(id, text).await;
            results.push((id.clone(), result));
        }
        results
    }

    async fn send_telegram(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let _token = config
            .token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Telegram token"))?;
        let _chat_id = config
            .target_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Telegram chat_id"))?;
        log::info!("Telegram: would send {} chars to chat", text.len());
        Ok(())
    }

    async fn send_discord(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let _webhook = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Discord webhook URL"))?;
        log::info!("Discord: would send {} chars", text.len());
        Ok(())
    }

    async fn send_slack(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let _webhook = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Slack webhook URL"))?;
        log::info!("Slack: would send {} chars", text.len());
        Ok(())
    }

    async fn send_email(&self, _config: &ChannelConfig, text: &str) -> Result<()> {
        log::info!("Email: would send {} chars", text.len());
        Ok(())
    }

    async fn send_webhook(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let _url = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No webhook URL"))?;
        log::info!("Webhook: would POST {} chars", text.len());
        Ok(())
    }
}

impl Default for ChannelRouter {
    fn default() -> Self {
        Self::new()
    }
}
