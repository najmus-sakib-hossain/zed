//! Signal channel integration (Rust-first wrapper around signal-cli).
//!
//! This implementation uses local `signal-cli` for transport.
//! It keeps DX in Rust while avoiding Node.js for Signal.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::info;

use crate::message::{ChannelMessage, DeliveryStatus, IncomingMessage, MessageContent};
use crate::traits::{Channel, ChannelCapabilities, ChannelRegistration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Registered Signal phone number used by signal-cli
    pub account: String,
    /// Optional explicit signal-cli binary path
    pub signal_cli_path: Option<String>,
}

pub struct SignalHandler {
    config: SignalConfig,
    connected: bool,
    enabled: bool,
}

impl SignalHandler {
    pub fn new(config: SignalConfig) -> Self {
        Self {
            config,
            connected: false,
            enabled: true,
        }
    }

    fn signal_cli_bin(&self) -> &str {
        self.config.signal_cli_path.as_deref().unwrap_or("signal-cli")
    }
}

#[async_trait]
impl Channel for SignalHandler {
    fn name(&self) -> &str {
        "signal"
    }

    fn display_name(&self) -> &str {
        "Signal"
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
            markdown: false,
            images: true,
            audio: true,
            video: true,
            files: true,
            reactions: false,
            structured: false,
            edit: false,
            delete: false,
            typing: false,
            read_receipts: false,
            groups: true,
            voice: false,
            webhooks: false,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "signal".into(),
            display_name: "Signal".into(),
            description: "Signal via signal-cli bridge".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            author: "DX".into(),
            icon: Some("🔐".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        let output = Command::new(self.signal_cli_bin())
            .arg("-a")
            .arg(&self.config.account)
            .arg("-v")
            .arg("version")
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                self.connected = true;
                info!("Signal channel connected for account {}", self.config.account);
                Ok(())
            }
            Ok(out) => {
                anyhow::bail!("signal-cli check failed: {}", String::from_utf8_lossy(&out.stderr))
            }
            Err(e) => anyhow::bail!("failed to execute signal-cli: {}", e),
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

        let text = match &message.content {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Markdown { text } => text.clone(),
            _ => "[unsupported content type]".to_string(),
        };

        let output = Command::new(self.signal_cli_bin())
            .arg("-a")
            .arg(&self.config.account)
            .arg("send")
            .arg("-m")
            .arg(text)
            .arg(&message.to)
            .output()
            .await?;

        if output.status.success() {
            Ok(DeliveryStatus::Sent)
        } else {
            Ok(DeliveryStatus::Failed(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        Ok(vec![])
    }

    async fn handle_webhook(&self, _payload: serde_json::Value) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_registration_and_capabilities() {
        let handler = SignalHandler::new(SignalConfig {
            account: "+10000000000".into(),
            signal_cli_path: None,
        });
        assert_eq!(handler.name(), "signal");
        assert!(handler.capabilities().text);
        assert!(handler.capabilities().groups);
    }
}
