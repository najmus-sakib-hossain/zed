//! Channel trait definitions for platform integrations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::message::{ChannelMessage, DeliveryStatus, IncomingMessage};

/// Core trait for all messaging channel integrations
#[async_trait]
pub trait Channel: Send + Sync {
    /// Unique channel type name (e.g., "telegram", "discord")
    fn name(&self) -> &str;

    /// Human-readable display name
    fn display_name(&self) -> &str;

    /// Whether the channel is currently enabled
    fn is_enabled(&self) -> bool;

    /// Whether the channel is currently connected
    fn is_connected(&self) -> bool;

    /// Channel capabilities
    fn capabilities(&self) -> ChannelCapabilities;

    /// Channel registration metadata
    fn registration(&self) -> ChannelRegistration;

    /// Connect to the channel (start bot, authenticate, etc.)
    async fn connect(&mut self) -> Result<()>;

    /// Disconnect from the channel
    async fn disconnect(&mut self) -> Result<()>;

    /// Send a message through this channel
    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus>;

    /// Poll for new messages (for pull-based channels)
    async fn receive(&self) -> Result<Vec<IncomingMessage>>;

    /// Handle an incoming webhook payload (for push-based channels)
    async fn handle_webhook(&self, payload: serde_json::Value) -> Result<()>;
}

/// Channel capabilities descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    /// Can send text messages
    pub text: bool,
    /// Can send markdown-formatted messages
    pub markdown: bool,
    /// Can send images
    pub images: bool,
    /// Can send audio
    pub audio: bool,
    /// Can send video
    pub video: bool,
    /// Can send files/documents
    pub files: bool,
    /// Can send reactions
    pub reactions: bool,
    /// Can send structured messages (buttons, cards)
    pub structured: bool,
    /// Can edit sent messages
    pub edit: bool,
    /// Can delete messages
    pub delete: bool,
    /// Supports typing indicators
    pub typing: bool,
    /// Supports read receipts
    pub read_receipts: bool,
    /// Supports group conversations
    pub groups: bool,
    /// Supports voice messages
    pub voice: bool,
    /// Supports webhooks
    pub webhooks: bool,
}

impl Default for ChannelCapabilities {
    fn default() -> Self {
        Self {
            text: true,
            markdown: false,
            images: false,
            audio: false,
            video: false,
            files: false,
            reactions: false,
            structured: false,
            edit: false,
            delete: false,
            typing: false,
            read_receipts: false,
            groups: false,
            voice: false,
            webhooks: false,
        }
    }
}

/// Channel registration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRegistration {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub icon: Option<String>,
    pub capabilities: ChannelCapabilities,
}
