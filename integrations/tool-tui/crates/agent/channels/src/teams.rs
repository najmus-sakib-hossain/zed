//! Microsoft Teams channel integration via REST API.
//!
//! Teams does not have a dedicated Rust crate.
//! This implementation uses the Microsoft Graph API directly
//! via reqwest, which is efficient and avoids Node.js.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::message::{ChannelMessage, DeliveryStatus, IncomingMessage};
use crate::traits::{Channel, ChannelCapabilities, ChannelRegistration};
use std::collections::HashMap;

/// Microsoft Teams channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Azure AD Tenant ID
    pub tenant_id: String,
    /// Azure AD Client ID (Application ID)
    pub client_id: String,
    /// Azure AD Client Secret
    pub client_secret: String,
    /// Bot webhook URL for incoming messages
    pub webhook_url: Option<String>,
    /// Team ID to monitor
    pub team_id: Option<String>,
    /// Channel ID within the team
    pub channel_id: Option<String>,
}

/// Microsoft Teams channel handler
pub struct TeamsHandler {
    config: TeamsConfig,
    connected: bool,
    enabled: bool,
    access_token: Option<String>,
}

impl TeamsHandler {
    pub fn new(config: TeamsConfig) -> Self {
        Self {
            config,
            connected: false,
            enabled: true,
            access_token: None,
        }
    }

    /// Authenticate with Azure AD to get an access token
    async fn authenticate(&mut self) -> Result<String> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.config.tenant_id
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("scope", "https://graph.microsoft.com/.default"),
        ];

        let client = reqwest::Client::new();
        let resp = client.post(&url).form(&params).send().await?;

        let body: serde_json::Value = resp.json().await?;
        let token = body["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No access_token in response"))?
            .to_string();

        self.access_token = Some(token.clone());
        Ok(token)
    }
}

#[async_trait]
impl Channel for TeamsHandler {
    fn name(&self) -> &str {
        "teams"
    }

    fn display_name(&self) -> &str {
        "Microsoft Teams"
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
            audio: false,
            video: false,
            files: true,
            reactions: true,
            structured: true, // Adaptive Cards
            edit: true,
            delete: true,
            typing: false,
            read_receipts: false,
            groups: true,
            voice: false,
            webhooks: true,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "teams".into(),
            display_name: "Microsoft Teams".into(),
            description: "Microsoft Teams via Graph API".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            author: "DX".into(),
            icon: Some("🟦".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Microsoft Teams...");
        self.authenticate().await?;
        self.connected = true;
        info!("Teams channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        self.access_token = None;
        info!("Teams channel disconnected");
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        if !self.connected {
            return Ok(DeliveryStatus::Failed("Not connected".into()));
        }

        let token =
            self.access_token.as_ref().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let team_id = self
            .config
            .team_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No team_id configured"))?;
        let target = &message.to;

        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages",
            team_id, target
        );

        let content_text = match &message.content {
            crate::message::MessageContent::Text { text } => text.clone(),
            _ => String::from("[unsupported content type]"),
        };

        let body = serde_json::json!({
            "body": {
                "contentType": "text",
                "content": content_text,
            }
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url).bearer_auth(token).json(&body).send().await?;

        if resp.status().is_success() {
            Ok(DeliveryStatus::Sent)
        } else {
            Ok(DeliveryStatus::Failed(format!("HTTP {}", resp.status())))
        }
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        // Teams uses webhooks for incoming messages
        Ok(vec![])
    }

    async fn handle_webhook(&self, payload: serde_json::Value) -> Result<()> {
        info!("Teams webhook received: {}", payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> TeamsConfig {
        TeamsConfig {
            tenant_id: "test-tenant".into(),
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
            webhook_url: None,
            team_id: Some("team-123".into()),
            channel_id: Some("channel-456".into()),
        }
    }

    #[test]
    fn test_teams_handler_creation() {
        let handler = TeamsHandler::new(test_config());
        assert_eq!(handler.name(), "teams");
        assert!(handler.is_enabled());
        assert!(!handler.is_connected());
    }

    #[test]
    fn test_teams_capabilities() {
        let handler = TeamsHandler::new(test_config());
        let caps = handler.capabilities();
        assert!(caps.text);
        assert!(caps.structured); // Adaptive Cards
        assert!(!caps.audio); // Not supported via Graph API
    }
}
