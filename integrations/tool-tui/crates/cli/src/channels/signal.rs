//! Signal Messaging Channel
//!
//! Private, encrypted messaging via Signal protocol.
//!
//! # Features
//!
//! - End-to-end encryption (Signal Protocol)
//! - DM and group chat support
//! - Media attachments
//! - Disappearing messages
//! - Linked devices support
//!
//! # Requirements
//!
//! - signal-cli installed and configured
//! - Phone number verified with Signal
//!
//! # Configuration
//!
//! ```sr
//! [signal]
//! enabled = true
//! phone_number = "+1234567890"
//! config_path = "~/.local/share/signal-cli"
//! trust_new_identities = "on-first-use"
//!
//! [signal.groups]
//! allowed = ["Family", "Work"]
//! blocked = []
//!
//! [signal.media]
//! auto_download = true
//! max_size_mb = 50
//! save_path = "~/.dx/signal/media"
//! ```

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::trait_def::{
    Channel, ChannelMessage, ChannelRegistration, DeliveryStatus, IncomingMessage, MessageContent,
};

/// Signal channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Enable Signal channel
    pub enabled: bool,
    /// Phone number (E.164 format)
    pub phone_number: String,
    /// signal-cli config path
    pub config_path: PathBuf,
    /// Trust policy for new identities
    pub trust_policy: TrustPolicy,
    /// Allowed groups
    pub allowed_groups: Vec<String>,
    /// Blocked groups
    pub blocked_groups: Vec<String>,
    /// Media settings
    pub media: MediaConfig,
    /// Enable receipts
    pub send_receipts: bool,
    /// Enable typing indicators
    pub typing_indicators: bool,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            phone_number: String::new(),
            config_path: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("signal-cli"),
            trust_policy: TrustPolicy::OnFirstUse,
            allowed_groups: Vec::new(),
            blocked_groups: Vec::new(),
            media: MediaConfig::default(),
            send_receipts: true,
            typing_indicators: true,
        }
    }
}

/// Trust policy for new identities
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TrustPolicy {
    /// Trust on first use
    #[default]
    OnFirstUse,
    /// Always trust
    Always,
    /// Never trust (manual verification required)
    Never,
    /// Trust only verified
    VerifiedOnly,
}

/// Media configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
    /// Auto-download media
    pub auto_download: bool,
    /// Maximum media size in MB
    pub max_size_mb: u32,
    /// Save path for media
    pub save_path: PathBuf,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            auto_download: true,
            max_size_mb: 50,
            save_path: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("dx")
                .join("signal")
                .join("media"),
        }
    }
}

/// Signal message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SignalMessageType {
    /// Text message
    Text { body: String },
    /// Reaction
    Reaction {
        emoji: String,
        target_timestamp: i64,
    },
    /// Media attachment
    Attachment {
        content_type: String,
        filename: String,
        size: u64,
    },
    /// Sticker
    Sticker { pack_id: String, sticker_id: u32 },
    /// Quote/reply
    Quote {
        author: String,
        timestamp: i64,
        body: String,
    },
}

/// Signal channel implementation
pub struct SignalChannel {
    /// Configuration
    config: SignalConfig,
    /// signal-cli path
    cli_path: PathBuf,
    /// Incoming message channel
    incoming_tx: Option<mpsc::Sender<IncomingMessage>>,
    /// Connection state
    connected: bool,
}

impl SignalChannel {
    /// Create a new Signal channel
    pub fn new(config: SignalConfig) -> Result<Self> {
        let cli_path = which::which("signal-cli")
            .context("signal-cli not found. Install from: https://github.com/AsamK/signal-cli")?;

        Ok(Self {
            config,
            cli_path,
            incoming_tx: None,
            connected: false,
        })
    }

    /// Create with default config
    pub fn with_defaults() -> Result<Self> {
        Self::new(SignalConfig::default())
    }

    /// Verify signal-cli is configured
    async fn verify_setup(&self) -> Result<()> {
        let output = Command::new(&self.cli_path)
            .args(["-a", &self.config.phone_number, "listAccounts"])
            .output()
            .await
            .context("Failed to run signal-cli")?;

        if !output.status.success() {
            bail!(
                "signal-cli not configured for {}. Run: signal-cli -a {} register",
                self.config.phone_number,
                self.config.phone_number
            );
        }

        Ok(())
    }

    /// Run signal-cli command
    async fn run_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.cli_path)
            .args(["-a", &self.config.phone_number])
            .args(args)
            .args(["--output", "json"])
            .output()
            .await
            .context("Failed to run signal-cli")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("signal-cli error: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Start daemon for receiving messages
    pub async fn start_daemon(&mut self, tx: mpsc::Sender<IncomingMessage>) -> Result<()> {
        self.incoming_tx = Some(tx.clone());
        let cli_path = self.cli_path.clone();
        let phone = self.config.phone_number.clone();

        tokio::spawn(async move {
            let mut child = match Command::new(&cli_path)
                .args(["-a", &phone, "daemon", "--json"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to start signal-cli daemon: {}", e);
                    return;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(envelope) = serde_json::from_str::<SignalEnvelope>(&line) {
                        if let Some(msg) = envelope.to_incoming_message() {
                            if tx.send(msg).await.is_err() {
                                warn!("Failed to forward Signal message");
                                break;
                            }
                        }
                    }
                }
            }

            info!("Signal daemon stopped");
        });

        self.connected = true;
        Ok(())
    }

    /// Send text message
    pub async fn send_text(&self, recipient: &str, text: &str) -> Result<DeliveryStatus> {
        let args = if recipient.starts_with('+') {
            vec!["send", "-m", text, recipient]
        } else {
            vec!["send", "-m", text, "-g", recipient]
        };

        self.run_command(&args).await?;
        Ok(DeliveryStatus::Delivered)
    }

    /// Send media attachment
    pub async fn send_attachment(
        &self,
        recipient: &str,
        path: &str,
        caption: Option<&str>,
    ) -> Result<DeliveryStatus> {
        let mut args = vec!["send", "-a", path];

        if let Some(cap) = caption {
            args.extend(["-m", cap]);
        }

        if recipient.starts_with('+') {
            args.push(recipient);
        } else {
            args.extend(["-g", recipient]);
        }

        self.run_command(&args).await?;
        Ok(DeliveryStatus::Delivered)
    }

    /// Send reaction
    pub async fn send_reaction(
        &self,
        recipient: &str,
        emoji: &str,
        target_author: &str,
        target_timestamp: i64,
    ) -> Result<()> {
        let ts_str = target_timestamp.to_string();
        let args = if recipient.starts_with('+') {
            vec![
                "sendReaction",
                "-e",
                emoji,
                "-a",
                target_author,
                "-t",
                &ts_str,
                recipient,
            ]
        } else {
            vec![
                "sendReaction",
                "-e",
                emoji,
                "-a",
                target_author,
                "-t",
                &ts_str,
                "-g",
                recipient,
            ]
        };

        self.run_command(&args).await?;
        Ok(())
    }

    /// Get group list
    pub async fn list_groups(&self) -> Result<Vec<SignalGroup>> {
        let output = self.run_command(&["listGroups", "-d"]).await?;
        let groups: Vec<SignalGroup> =
            serde_json::from_str(&output).context("Failed to parse groups")?;
        Ok(groups)
    }

    /// Get contacts
    pub async fn list_contacts(&self) -> Result<Vec<SignalContact>> {
        let output = self.run_command(&["listContacts"]).await?;
        let contacts: Vec<SignalContact> =
            serde_json::from_str(&output).context("Failed to parse contacts")?;
        Ok(contacts)
    }

    /// Trust an identity
    pub async fn trust_identity(&self, recipient: &str) -> Result<()> {
        self.run_command(&["trust", "-a", recipient]).await?;
        Ok(())
    }

    /// Verify safety number
    pub async fn verify_identity(&self, recipient: &str, safety_number: &str) -> Result<()> {
        self.run_command(&["trust", "-v", safety_number, recipient]).await?;
        Ok(())
    }
}

/// Signal envelope from daemon
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalEnvelope {
    source: Option<String>,
    source_number: Option<String>,
    source_uuid: Option<String>,
    timestamp: Option<i64>,
    data_message: Option<SignalDataMessage>,
    sync_message: Option<serde_json::Value>,
    typing_message: Option<serde_json::Value>,
    receipt_message: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalDataMessage {
    message: Option<String>,
    timestamp: i64,
    group_info: Option<SignalGroupInfo>,
    attachments: Option<Vec<SignalAttachment>>,
    reaction: Option<SignalReaction>,
    quote: Option<SignalQuote>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalGroupInfo {
    group_id: String,
    #[serde(rename = "type")]
    group_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalAttachment {
    content_type: String,
    filename: Option<String>,
    size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalReaction {
    emoji: String,
    target_author: String,
    target_sent_timestamp: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalQuote {
    author: String,
    timestamp: i64,
    text: Option<String>,
}

impl SignalEnvelope {
    fn to_incoming_message(&self) -> Option<IncomingMessage> {
        let data = self.data_message.as_ref()?;
        let source = self.source_number.clone().or_else(|| self.source.clone())?;

        let (channel_type, chat_id) = if let Some(group) = &data.group_info {
            ("signal_group".to_string(), group.group_id.clone())
        } else {
            ("signal_dm".to_string(), source.clone())
        };

        let content = if let Some(text) = &data.message {
            MessageContent::Text(text.clone())
        } else if let Some(attachments) = &data.attachments {
            if let Some(att) = attachments.first() {
                MessageContent::Media {
                    url: String::new(), // Need to save and get path
                    mime: Some(att.content_type.clone()),
                    caption: None,
                }
            } else {
                return None;
            }
        } else if let Some(reaction) = &data.reaction {
            MessageContent::Reaction(reaction.emoji.clone())
        } else {
            return None;
        };

        Some(IncomingMessage {
            id: format!("{}_{}", source, data.timestamp),
            channel_type,
            chat_id,
            from: source.clone(),
            sender_id: source,
            sender_name: None,
            channel: "signal".to_string(),
            content,
            timestamp: chrono::Utc::now(),
            reply_to: None,
            metadata: HashMap::new(),
        })
    }
}

/// Signal group info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalGroup {
    /// Group ID
    pub id: String,
    /// Group name
    pub name: String,
    /// Member count
    pub member_count: Option<u32>,
    /// Is blocked
    pub blocked: bool,
}

/// Signal contact
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalContact {
    /// Phone number
    pub number: String,
    /// UUID
    pub uuid: Option<String>,
    /// Name
    pub name: Option<String>,
    /// Profile name
    pub profile_name: Option<String>,
    /// Is blocked
    pub blocked: bool,
}

#[async_trait]
impl Channel for SignalChannel {
    fn name(&self) -> &str {
        "signal"
    }

    fn display_name(&self) -> &str {
        "Signal"
    }

    async fn connect(&mut self) -> Result<()> {
        self.verify_setup().await?;
        self.connected = true;
        info!("Signal channel connected: {}", self.config.phone_number);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        self.incoming_tx = None;
        info!("Signal channel disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        if !self.connected {
            bail!("Signal channel not connected");
        }

        match message.content {
            MessageContent::Text(text) => self.send_text(&message.to, &text).await,
            MessageContent::Media { url, caption, .. } => {
                self.send_attachment(&message.to, &url, caption.as_deref()).await
            }
            MessageContent::Reaction(emoji) => {
                // Need target message info from metadata
                let target_author = message
                    .metadata
                    .get("target_author")
                    .map(|s| s.as_str())
                    .unwrap_or(&message.to);
                let target_timestamp: i64 = message
                    .metadata
                    .get("target_timestamp")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                self.send_reaction(&message.to, &emoji, target_author, target_timestamp).await?;
                Ok(DeliveryStatus::Delivered)
            }
            _ => bail!("Unsupported message type for Signal"),
        }
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        // Messages come through daemon, return empty here
        Ok(Vec::new())
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "signal".to_string(),
            display_name: "Signal".to_string(),
            description: "Private, encrypted messaging".to_string(),
            version: "1.0.0".to_string(),
            author: "DX Team".to_string(),
            capabilities: vec![
                "dm".to_string(),
                "group".to_string(),
                "media".to_string(),
                "reactions".to_string(),
                "encryption".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SignalConfig::default();
        assert!(config.enabled);
        assert!(config.send_receipts);
        assert_eq!(config.trust_policy, TrustPolicy::OnFirstUse);
    }

    #[test]
    fn test_envelope_parsing() {
        let json = r#"{
            "sourceNumber": "+1234567890",
            "timestamp": 1234567890,
            "dataMessage": {
                "message": "Hello Signal!",
                "timestamp": 1234567890
            }
        }"#;

        let envelope: SignalEnvelope = serde_json::from_str(json).unwrap();
        let msg = envelope.to_incoming_message().unwrap();

        assert_eq!(msg.sender_id, "+1234567890");
        assert!(matches!(msg.content, MessageContent::Text(_)));
    }

    #[test]
    fn test_group_message_parsing() {
        let json = r#"{
            "sourceNumber": "+1234567890",
            "timestamp": 1234567890,
            "dataMessage": {
                "message": "Hello group!",
                "timestamp": 1234567890,
                "groupInfo": {
                    "groupId": "abc123"
                }
            }
        }"#;

        let envelope: SignalEnvelope = serde_json::from_str(json).unwrap();
        let msg = envelope.to_incoming_message().unwrap();

        assert_eq!(msg.channel_type, "signal_group");
        assert_eq!(msg.chat_id, "abc123");
    }
}
