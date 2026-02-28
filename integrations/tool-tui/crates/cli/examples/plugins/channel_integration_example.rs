//! Example Channel Integration for DX
//!
//! This demonstrates how to create a custom messaging channel
//! that integrates with the DX agent system.
//!
//! # Overview
//!
//! Channels provide a unified interface for the DX agent to communicate
//! through various messaging platforms (Slack, Discord, Telegram, etc.).
//!
//! # Usage
//!
//! 1. Implement the `Channel` trait
//! 2. Create a channel.sr configuration file
//! 3. Place in `.dx/channels/` for auto-discovery

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Channel Trait
// ============================================================================

/// Message content that can be sent through channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text message
    Text(String),
    /// Markdown-formatted text
    Markdown(String),
    /// Code block with optional language
    Code { language: Option<String>, content: String },
    /// Image with optional caption
    Image { url: String, caption: Option<String> },
    /// File attachment
    File { name: String, data: Vec<u8>, mime_type: String },
    /// Interactive buttons
    Buttons { text: String, buttons: Vec<Button> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub style: ButtonStyle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
    Link,
}

/// Incoming message from a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: MessageContent,
    pub timestamp: u64,
    pub thread_id: Option<String>,
    pub reply_to: Option<String>,
}

/// Outgoing message to a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub channel_id: String,
    pub content: MessageContent,
    pub thread_id: Option<String>,
    pub reply_to: Option<String>,
}

/// Channel error type
#[derive(Debug, Clone)]
pub struct ChannelError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ChannelError {}

/// Result type for channel operations
pub type ChannelResult<T> = Result<T, ChannelError>;

/// The main Channel trait that all integrations must implement
#[async_trait]
pub trait Channel: Send + Sync {
    /// Returns the channel type identifier (e.g., "slack", "discord")
    fn channel_type(&self) -> &str;

    /// Returns the channel's display name
    fn name(&self) -> &str;

    /// Send a message to the channel
    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String>;

    /// Edit an existing message
    async fn edit(&self, message_id: &str, content: MessageContent) -> ChannelResult<()>;

    /// Delete a message
    async fn delete(&self, message_id: &str) -> ChannelResult<()>;

    /// React to a message with an emoji
    async fn react(&self, message_id: &str, emoji: &str) -> ChannelResult<()>;

    /// Start typing indicator
    async fn start_typing(&self, channel_id: &str) -> ChannelResult<()>;

    /// Subscribe to incoming messages (returns a receiver)
    async fn subscribe(&self) -> ChannelResult<tokio::sync::mpsc::Receiver<IncomingMessage>>;

    /// Check if the channel is connected
    fn is_connected(&self) -> bool;

    /// Reconnect to the channel
    async fn reconnect(&self) -> ChannelResult<()>;
}

// ============================================================================
// Example: Matrix Channel Implementation
// ============================================================================

/// Example Matrix channel implementation
pub struct MatrixChannel {
    homeserver: String,
    user_id: String,
    access_token: String,
    room_id: String,
    client: reqwest::Client,
    connected: std::sync::atomic::AtomicBool,
}

impl MatrixChannel {
    /// Create a new Matrix channel
    pub fn new(
        homeserver: &str,
        user_id: &str,
        access_token: &str,
        room_id: &str,
    ) -> Self {
        Self {
            homeserver: homeserver.to_string(),
            user_id: user_id.to_string(),
            access_token: access_token.to_string(),
            room_id: room_id.to_string(),
            client: reqwest::Client::new(),
            connected: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Load from channel.sr configuration
    pub fn from_config(config: &HashMap<String, String>) -> ChannelResult<Self> {
        let homeserver = config.get("homeserver").ok_or_else(|| ChannelError {
            code: "CONFIG_ERROR".into(),
            message: "Missing homeserver".into(),
            retryable: false,
        })?;

        let user_id = config.get("user_id").ok_or_else(|| ChannelError {
            code: "CONFIG_ERROR".into(),
            message: "Missing user_id".into(),
            retryable: false,
        })?;

        let access_token = config.get("access_token").ok_or_else(|| ChannelError {
            code: "CONFIG_ERROR".into(),
            message: "Missing access_token".into(),
            retryable: false,
        })?;

        let room_id = config.get("room_id").ok_or_else(|| ChannelError {
            code: "CONFIG_ERROR".into(),
            message: "Missing room_id".into(),
            retryable: false,
        })?;

        Ok(Self::new(homeserver, user_id, access_token, room_id))
    }

    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/_matrix/client/r0{}?access_token={}",
            self.homeserver, path, self.access_token
        )
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn channel_type(&self) -> &str {
        "matrix"
    }

    fn name(&self) -> &str {
        "Matrix"
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        let txn_id = format!("m{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());

        let body = match message.content {
            MessageContent::Text(text) => serde_json::json!({
                "msgtype": "m.text",
                "body": text,
            }),
            MessageContent::Markdown(md) => serde_json::json!({
                "msgtype": "m.text",
                "body": md,
                "format": "org.matrix.custom.html",
                "formatted_body": markdown_to_html(&md),
            }),
            MessageContent::Code { language, content } => serde_json::json!({
                "msgtype": "m.text",
                "body": format!("```{}\n{}\n```", language.unwrap_or_default(), content),
                "format": "org.matrix.custom.html",
                "formatted_body": format!("<pre><code class=\"language-{}\">{}</code></pre>", 
                    language.unwrap_or_default(), html_escape(&content)),
            }),
            MessageContent::Image { url, caption } => serde_json::json!({
                "msgtype": "m.image",
                "body": caption.unwrap_or_else(|| "image".to_string()),
                "url": url,
            }),
            _ => serde_json::json!({
                "msgtype": "m.text",
                "body": "Unsupported message type",
            }),
        };

        let url = self.build_url(&format!(
            "/rooms/{}/send/m.room.message/{}",
            urlencoding::encode(&self.room_id),
            txn_id
        ));

        let response = self.client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError {
                code: "NETWORK_ERROR".into(),
                message: e.to_string(),
                retryable: true,
            })?;

        if !response.status().is_success() {
            return Err(ChannelError {
                code: "API_ERROR".into(),
                message: format!("HTTP {}", response.status()),
                retryable: response.status().is_server_error(),
            });
        }

        let result: serde_json::Value = response.json().await.map_err(|e| ChannelError {
            code: "PARSE_ERROR".into(),
            message: e.to_string(),
            retryable: false,
        })?;

        Ok(result["event_id"].as_str().unwrap_or("unknown").to_string())
    }

    async fn edit(&self, message_id: &str, content: MessageContent) -> ChannelResult<()> {
        let text = match content {
            MessageContent::Text(t) => t,
            MessageContent::Markdown(m) => m,
            _ => return Err(ChannelError {
                code: "UNSUPPORTED".into(),
                message: "Can only edit text messages".into(),
                retryable: false,
            }),
        };

        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": format!("* {}", text),
            "m.new_content": {
                "msgtype": "m.text",
                "body": text,
            },
            "m.relates_to": {
                "rel_type": "m.replace",
                "event_id": message_id,
            }
        });

        let txn_id = format!("e{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());

        let url = self.build_url(&format!(
            "/rooms/{}/send/m.room.message/{}",
            urlencoding::encode(&self.room_id),
            txn_id
        ));

        self.client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError {
                code: "NETWORK_ERROR".into(),
                message: e.to_string(),
                retryable: true,
            })?;

        Ok(())
    }

    async fn delete(&self, message_id: &str) -> ChannelResult<()> {
        let body = serde_json::json!({
            "reason": "Message deleted"
        });

        let url = self.build_url(&format!(
            "/rooms/{}/redact/{}/{}",
            urlencoding::encode(&self.room_id),
            urlencoding::encode(message_id),
            format!("d{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis())
        ));

        self.client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError {
                code: "NETWORK_ERROR".into(),
                message: e.to_string(),
                retryable: true,
            })?;

        Ok(())
    }

    async fn react(&self, message_id: &str, emoji: &str) -> ChannelResult<()> {
        let body = serde_json::json!({
            "m.relates_to": {
                "rel_type": "m.annotation",
                "event_id": message_id,
                "key": emoji,
            }
        });

        let txn_id = format!("r{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());

        let url = self.build_url(&format!(
            "/rooms/{}/send/m.reaction/{}",
            urlencoding::encode(&self.room_id),
            txn_id
        ));

        self.client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError {
                code: "NETWORK_ERROR".into(),
                message: e.to_string(),
                retryable: true,
            })?;

        Ok(())
    }

    async fn start_typing(&self, _channel_id: &str) -> ChannelResult<()> {
        let body = serde_json::json!({
            "typing": true,
            "timeout": 10000,
        });

        let url = self.build_url(&format!(
            "/rooms/{}/typing/{}",
            urlencoding::encode(&self.room_id),
            urlencoding::encode(&self.user_id)
        ));

        self.client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError {
                code: "NETWORK_ERROR".into(),
                message: e.to_string(),
                retryable: true,
            })?;

        Ok(())
    }

    async fn subscribe(&self) -> ChannelResult<tokio::sync::mpsc::Receiver<IncomingMessage>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // In a real implementation, this would start a long-polling or
        // WebSocket connection to receive events
        
        // Simulated - spawn a task that would poll /sync
        let _homeserver = self.homeserver.clone();
        let _access_token = self.access_token.clone();
        let _room_id = self.room_id.clone();

        tokio::spawn(async move {
            // Poll /sync endpoint and forward messages to tx
            // This is a placeholder - real implementation would use Matrix SDK
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        self.connected.store(true, std::sync::atomic::Ordering::SeqCst);

        Ok(rx)
    }

    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }

    async fn reconnect(&self) -> ChannelResult<()> {
        // Re-establish sync connection
        self.connected.store(false, std::sync::atomic::Ordering::SeqCst);
        self.subscribe().await?;
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn markdown_to_html(md: &str) -> String {
    // Simple markdown conversion (use pulldown-cmark for full support)
    md.replace("**", "<strong>")
        .replace("*", "<em>")
        .replace("`", "<code>")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ============================================================================
// Channel.sr Configuration Example
// ============================================================================

// ```sr
// # .dx/channels/matrix.sr
// 
// [channel]
// type = "matrix"
// name = "My Matrix Room"
// enabled = true
// 
// [channel.config]
// homeserver = "https://matrix.org"
// user_id = "@dx-agent:matrix.org"
// access_token = "${MATRIX_ACCESS_TOKEN}"  # From environment
// room_id = "!abc123:matrix.org"
// 
// [channel.options]
// auto_reconnect = true
// reconnect_delay_ms = 5000
// max_retries = 3
// 
// [channel.filters]
// # Only process messages from these users
// allowed_senders = ["@alice:matrix.org", "@bob:matrix.org"]
// # Ignore messages starting with these prefixes
// ignore_prefixes = ["!", "/"]
// # Only respond to messages mentioning the bot
// require_mention = false
// ```

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_content_serialization() {
        let content = MessageContent::Text("Hello".to_string());
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_button_style() {
        let button = Button {
            id: "btn1".to_string(),
            label: "Click me".to_string(),
            style: ButtonStyle::Primary,
        };
        let json = serde_json::to_string(&button).unwrap();
        assert!(json.contains("Primary"));
    }

    #[test]
    fn test_channel_error_display() {
        let err = ChannelError {
            code: "TEST".to_string(),
            message: "Test error".to_string(),
            retryable: true,
        };
        assert_eq!(format!("{}", err), "[TEST] Test error");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }
}
