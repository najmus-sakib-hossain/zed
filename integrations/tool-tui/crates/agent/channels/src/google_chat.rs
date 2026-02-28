//! Google Chat integration via Google Chat REST API.
//!
//! Uses service-account OAuth token + Space messages endpoint.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::message::{ChannelMessage, DeliveryStatus, IncomingMessage, MessageContent};
use crate::traits::{Channel, ChannelCapabilities, ChannelRegistration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatConfig {
    /// OAuth bearer token for Chat API
    pub access_token: String,
    /// Optional default space: spaces/AAAA...
    pub default_space: Option<String>,
}

pub struct GoogleChatHandler {
    config: GoogleChatConfig,
    connected: bool,
    enabled: bool,
    client: reqwest::Client,
}

impl GoogleChatHandler {
    pub fn new(config: GoogleChatConfig) -> Self {
        Self {
            config,
            connected: false,
            enabled: true,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Channel for GoogleChatHandler {
    fn name(&self) -> &str {
        "google_chat"
    }

    fn display_name(&self) -> &str {
        "Google Chat"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            text: true,
            markdown: true,
            images: false,
            audio: false,
            video: false,
            files: false,
            reactions: false,
            structured: true,
            edit: false,
            delete: false,
            typing: false,
            read_receipts: false,
            groups: true,
            voice: false,
            webhooks: true,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "google_chat".into(),
            display_name: "Google Chat".into(),
            description: "Google Chat API integration".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            author: "DX".into(),
            icon: Some("💬".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        self.connected = !self.config.access_token.trim().is_empty();
        if self.connected {
            info!("Google Chat channel connected");
            Ok(())
        } else {
            anyhow::bail!("missing Google Chat access token")
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        if !self.connected {
            return Ok(DeliveryStatus::Failed("Not connected".into()));
        }

        let space = if message.to.starts_with("spaces/") {
            message.to
        } else {
            self.config
                .default_space
                .clone()
                .ok_or_else(|| anyhow::anyhow!("target space missing"))?
        };

        let text = match &message.content {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Markdown { text } => text.clone(),
            _ => "[unsupported content type]".to_string(),
        };

        let url = format!("https://chat.googleapis.com/v1/{}/messages", space);
        let payload = serde_json::json!({ "text": text });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.access_token)
            .json(&payload)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(DeliveryStatus::Sent)
        } else {
            Ok(DeliveryStatus::Failed(format!("HTTP {}", resp.status())))
        }
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        Ok(vec![])
    }

    async fn handle_webhook(&self, payload: serde_json::Value) -> Result<()> {
        info!("Google Chat webhook event: {}", payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_chat_registration() {
        let handler = GoogleChatHandler::new(GoogleChatConfig {
            access_token: "test-token".into(),
            default_space: Some("spaces/AAA".into()),
        });
        assert_eq!(handler.name(), "google_chat");
        assert!(handler.capabilities().structured);
    }
}
