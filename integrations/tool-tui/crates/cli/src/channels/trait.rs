//! Channel Trait System
//!
//! Defines the core `Channel` trait and message types for messaging channels.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Message content for channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text
    Text(String),
    /// Rich text (markdown)
    Markdown(String),
    /// Binary data (e.g., image, file)
    Binary {
        /// MIME type
        mime: String,
        /// Data bytes
        data: Vec<u8>,
        /// Optional filename
        filename: Option<String>,
    },
    /// Structured data payload
    Structured(serde_json::Value),
}

/// Message to send
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Target recipient (phone, user ID, channel, etc.)
    pub to: String,
    /// Message content
    pub content: MessageContent,
    /// Optional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Incoming message from a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Sender identifier
    pub from: String,
    /// Channel name
    pub channel: String,
    /// Message content
    pub content: MessageContent,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// Delivered successfully
    Delivered,
    /// Failed to deliver
    Failed { reason: String },
    /// Pending delivery
    Pending,
}

/// Channel configuration base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel name
    pub name: String,
    /// Enabled flag
    pub enabled: bool,
    /// Channel-specific settings
    #[serde(default)]
    pub settings: serde_json::Value,
}

impl ChannelConfig {
    /// Create a new config
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            settings: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// The core channel trait
#[async_trait]
pub trait Channel: Send + Sync {
    /// Channel name
    fn name(&self) -> &str;

    /// Whether the channel is enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Initialize the channel
    async fn initialize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Send a message
    async fn send(&self, message: ChannelMessage) -> anyhow::Result<DeliveryStatus>;

    /// Receive messages (if supported)
    async fn receive(&self) -> anyhow::Result<Vec<IncomingMessage>> {
        Ok(Vec::new())
    }

    /// Handle a webhook payload (if supported)
    async fn handle_webhook(&self, _payload: serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Channel registration info
#[derive(Debug, Clone)]
pub struct ChannelRegistration {
    /// Channel name
    pub name: String,
    /// Channel type
    pub channel_type: String,
    /// Configuration path
    pub config_path: Option<std::path::PathBuf>,
}
