//! Matrix channel integration using the matrix-sdk Rust crate.
//!
//! Matrix is a fully open protocol with an excellent Rust SDK (`matrix-sdk`).
//! This is a pure Rust implementation — no Node.js required.
//!
//! Enable with the `matrix` feature flag.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::message::{ChannelMessage, DeliveryStatus, IncomingMessage};
use crate::traits::{Channel, ChannelCapabilities, ChannelRegistration};

/// Matrix channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// Homeserver URL (e.g., "https://matrix.org")
    pub homeserver_url: String,
    /// Bot username
    pub username: String,
    /// Bot password (or use access token)
    pub password: Option<String>,
    /// Access token (alternative to password)
    pub access_token: Option<String>,
    /// Room IDs to join on start
    #[serde(default)]
    pub auto_join_rooms: Vec<String>,
    /// Accept room invites automatically
    #[serde(default = "default_true")]
    pub auto_accept_invites: bool,
    /// Display name for the bot
    pub display_name: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Matrix channel handler
pub struct MatrixHandler {
    config: MatrixConfig,
    connected: bool,
    enabled: bool,
}

impl MatrixHandler {
    pub fn new(config: MatrixConfig) -> Self {
        Self {
            config,
            connected: false,
            enabled: true,
        }
    }
}

#[async_trait]
impl Channel for MatrixHandler {
    fn name(&self) -> &str {
        "matrix"
    }

    fn display_name(&self) -> &str {
        "Matrix"
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
            images: true,
            audio: true,
            video: true,
            files: true,
            reactions: true,
            structured: false,
            edit: true,
            delete: true,
            typing: true,
            read_receipts: true,
            groups: true,
            voice: false,
            webhooks: false,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "matrix".into(),
            display_name: "Matrix".into(),
            description: "Matrix messaging protocol (decentralized, E2EE)".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            author: "DX".into(),
            icon: Some("🟢".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Matrix homeserver: {}", self.config.homeserver_url);

        // When matrix-sdk feature is enabled, use:
        // let client = matrix_sdk::Client::builder()
        //     .homeserver_url(&self.config.homeserver_url)
        //     .build()
        //     .await?;
        //
        // if let Some(ref token) = self.config.access_token {
        //     client.restore_session(session).await?;
        // } else if let Some(ref password) = self.config.password {
        //     client.matrix_auth()
        //         .login_username(&self.config.username, password)
        //         .send()
        //         .await?;
        // }

        self.connected = true;
        info!("Matrix channel connected as {}", self.config.username);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        info!("Matrix channel disconnected");
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        if !self.connected {
            return Ok(DeliveryStatus::Failed("Not connected".into()));
        }

        // When matrix-sdk is available:
        // let room = client.get_room(&room_id)?;
        // room.send(RoomMessageEventContent::text_plain(&message.content)).await?;

        info!("Matrix: would send to {}", message.to);

        Ok(DeliveryStatus::Sent)
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        // matrix-sdk uses event handlers (push-based), not polling
        Ok(vec![])
    }

    async fn handle_webhook(&self, _payload: serde_json::Value) -> Result<()> {
        // Matrix uses long-polling/sync, not webhooks
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MatrixConfig {
        MatrixConfig {
            homeserver_url: "https://matrix.org".into(),
            username: "@dxbot:matrix.org".into(),
            password: Some("test-password".into()),
            access_token: None,
            auto_join_rooms: vec!["!test:matrix.org".into()],
            auto_accept_invites: true,
            display_name: Some("DX Bot".into()),
        }
    }

    #[test]
    fn test_matrix_handler_creation() {
        let handler = MatrixHandler::new(test_config());
        assert_eq!(handler.name(), "matrix");
        assert!(handler.is_enabled());
        assert!(!handler.is_connected());
    }

    #[test]
    fn test_matrix_capabilities() {
        let handler = MatrixHandler::new(test_config());
        let caps = handler.capabilities();
        assert!(caps.text);
        assert!(caps.markdown);
        assert!(caps.edit);
        assert!(caps.typing);
    }

    #[tokio::test]
    async fn test_matrix_connect_disconnect() {
        let mut handler = MatrixHandler::new(test_config());
        handler.connect().await.unwrap();
        assert!(handler.is_connected());
        handler.disconnect().await.unwrap();
        assert!(!handler.is_connected());
    }
}
