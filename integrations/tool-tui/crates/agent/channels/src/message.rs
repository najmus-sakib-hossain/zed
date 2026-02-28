//! Core message types for cross-platform messaging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Content types that can be sent/received across channels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    /// Plain text message
    Text { text: String },
    /// Markdown-formatted message
    Markdown { text: String },
    /// Media attachment (image, audio, video, file)
    Media(MediaAttachment),
    /// Emoji reaction
    Reaction { emoji: String },
    /// Structured data (buttons, cards, etc.)
    Structured { data: serde_json::Value },
    /// Interactive message with inline keyboard/buttons
    Interactive {
        text: String,
        keyboard: InlineKeyboard,
    },
    /// Location sharing
    Location {
        latitude: f64,
        longitude: f64,
        label: Option<String>,
    },
    /// Contact card
    Contact {
        name: String,
        phone: Option<String>,
        email: Option<String>,
    },
}

/// Media attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    /// Media type
    pub media_type: MediaType,
    /// URL or base64 data
    pub url: Option<String>,
    /// Raw binary data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<u8>>,
    /// MIME type
    pub mime_type: String,
    /// Original filename
    pub filename: Option<String>,
    /// Caption
    pub caption: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
    /// Duration in seconds (for audio/video)
    pub duration_secs: Option<u32>,
    /// Thumbnail URL
    pub thumbnail_url: Option<String>,
}

/// Media types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
    Voice,
    Sticker,
    Animation,
}

/// Inline keyboard / button layout for interactive messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineKeyboard {
    /// Rows of buttons
    pub rows: Vec<Vec<InlineButton>>,
}

/// Button in an inline keyboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineButton {
    /// Display text on the button
    pub text: String,
    /// Action when clicked
    pub action: ButtonAction,
}

/// Button action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ButtonAction {
    /// Send callback data back to the bot
    Callback { data: String },
    /// Open a URL
    Url { url: String },
    /// Switch to inline query in same chat
    SwitchInline { query: String },
    /// Copy text to clipboard
    Copy { text: String },
}

impl InlineKeyboard {
    /// Create a new empty keyboard
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Add a row of buttons
    pub fn row(mut self, buttons: Vec<InlineButton>) -> Self {
        self.rows.push(buttons);
        self
    }

    /// Add a single callback button as its own row
    pub fn button(self, text: impl Into<String>, callback_data: impl Into<String>) -> Self {
        self.row(vec![InlineButton {
            text: text.into(),
            action: ButtonAction::Callback {
                data: callback_data.into(),
            },
        }])
    }

    /// Add a URL button as its own row
    pub fn url_button(self, text: impl Into<String>, url: impl Into<String>) -> Self {
        self.row(vec![InlineButton {
            text: text.into(),
            action: ButtonAction::Url { url: url.into() },
        }])
    }
}

impl Default for InlineKeyboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Outgoing message to send through a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Target chat/user/channel ID
    pub to: String,
    /// Message content
    pub content: MessageContent,
    /// Reply to a specific message
    pub reply_to: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Incoming message received from a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Unique message ID
    pub id: String,
    /// Channel type (telegram, discord, slack, etc.)
    pub channel_type: String,
    /// Chat/conversation ID
    pub chat_id: String,
    /// Sender user ID
    pub sender_id: String,
    /// Sender display name
    pub sender_name: Option<String>,
    /// Channel instance name
    pub channel_name: String,
    /// Message content
    pub content: MessageContent,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Reply-to message ID
    pub reply_to: Option<String>,
    /// Additional metadata (platform-specific)
    pub metadata: HashMap<String, String>,
    /// Whether this is a group message
    pub is_group: bool,
    /// Group/chat title if applicable
    pub group_name: Option<String>,
}

/// Message delivery status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
    Sent,
    Delivered,
    Read,
    Failed(String),
    Pending,
}

impl ChannelMessage {
    /// Create a simple text message
    pub fn text(to: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            content: MessageContent::Text { text: text.into() },
            reply_to: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a markdown message
    pub fn markdown(to: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            content: MessageContent::Markdown { text: text.into() },
            reply_to: None,
            metadata: HashMap::new(),
        }
    }

    /// Set reply-to
    pub fn with_reply(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_message_text() {
        let msg = ChannelMessage::text("user123", "Hello!");
        assert_eq!(msg.to, "user123");
        if let MessageContent::Text { text } = &msg.content {
            assert_eq!(text, "Hello!");
        } else {
            panic!("Expected Text content");
        }
    }

    #[test]
    fn test_message_serialization() {
        let msg = IncomingMessage {
            id: "msg-1".into(),
            channel_type: "telegram".into(),
            chat_id: "chat-1".into(),
            sender_id: "user-1".into(),
            sender_name: Some("Alice".into()),
            channel_name: "my-telegram".into(),
            content: MessageContent::Text {
                text: "Hello!".into(),
            },
            timestamp: Utc::now(),
            reply_to: None,
            metadata: HashMap::new(),
            is_group: false,
            group_name: None,
        };

        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("telegram"));
        assert!(json.contains("Hello!"));
    }
}
