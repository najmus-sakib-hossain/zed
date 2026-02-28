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
    /// Media attachment
    Media {
        /// URL or path to media
        url: String,
        /// MIME type
        #[serde(default)]
        mime: Option<String>,
        /// Optional caption
        #[serde(default)]
        caption: Option<String>,
    },
    /// Reaction emoji
    Reaction(String),
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
    /// Message ID
    #[serde(default)]
    pub id: String,
    /// Channel type (e.g., "imessage_dm", "imessage_group")
    #[serde(default)]
    pub channel_type: String,
    /// Chat/conversation ID
    #[serde(default)]
    pub chat_id: String,
    /// Sender identifier
    pub from: String,
    /// Sender ID
    #[serde(default)]
    pub sender_id: String,
    /// Sender display name
    #[serde(default)]
    pub sender_name: Option<String>,
    /// Channel name
    pub channel: String,
    /// Message content
    pub content: MessageContent,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Reply to message ID
    #[serde(default)]
    pub reply_to: Option<String>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// Sent successfully (not yet confirmed delivered)
    Sent,
    /// Delivered successfully
    Delivered,
    /// Failed to deliver
    Failed(String),
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

    /// Display name for UI
    fn display_name(&self) -> &str {
        self.name()
    }

    /// Whether the channel is enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Whether the channel is connected
    fn is_connected(&self) -> bool {
        true
    }

    /// Connect to the channel
    async fn connect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Disconnect from the channel
    async fn disconnect(&mut self) -> anyhow::Result<()> {
        Ok(())
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

    /// Get channel registration info
    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: self.name().to_string(),
            display_name: self.display_name().to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            author: String::new(),
            capabilities: vec![],
        }
    }
}

/// Channel registration info
#[derive(Debug, Clone)]
pub struct ChannelRegistration {
    /// Channel name
    pub name: String,
    /// Display name for UI
    pub display_name: String,
    /// Description
    pub description: String,
    /// Version
    pub version: String,
    /// Author
    pub author: String,
    /// Capabilities
    pub capabilities: Vec<String>,
}
