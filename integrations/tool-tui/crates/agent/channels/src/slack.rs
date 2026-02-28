//! Native Rust Slack integration using HTTP API.
//!
//! Uses Slack Web API + Socket Mode for real-time events.

use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use crate::message::*;
use crate::traits::*;

/// Slack channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot token (xoxb-...)
    pub bot_token: String,
    /// App-level token for Socket Mode (xapp-...)
    pub app_token: Option<String>,
    /// Signing secret for webhook verification
    pub signing_secret: Option<String>,
    /// Default channel for messages
    pub default_channel: Option<String>,
}

/// Slack API response wrapper
#[derive(Debug, Deserialize)]
struct SlackResponse {
    ok: bool,
    error: Option<String>,
    #[allow(dead_code)]
    ts: Option<String>,
    #[allow(dead_code)]
    channel: Option<String>,
}

/// Slack event payload
#[derive(Debug, Deserialize)]
struct SlackEvent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    event_type: Option<String>,
    event: Option<SlackInnerEvent>,
    challenge: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SocketOpenResponse {
    ok: bool,
    url: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SocketEnvelope {
    envelope_id: Option<String>,
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SlackInnerEvent {
    #[serde(rename = "type")]
    event_type: String,
    user: Option<String>,
    text: Option<String>,
    channel: Option<String>,
    ts: Option<String>,
    thread_ts: Option<String>,
    bot_id: Option<String>,
}

/// Native Slack channel
pub struct SlackChannel {
    config: SlackConfig,
    http: HttpClient,
    connected: Arc<AtomicBool>,
    incoming: Arc<Mutex<Vec<IncomingMessage>>>,
    bot_user_id: Arc<Mutex<Option<String>>>,
    socket_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl SlackChannel {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            http: HttpClient::new(),
            connected: Arc::new(AtomicBool::new(false)),
            incoming: Arc::new(Mutex::new(Vec::new())),
            bot_user_id: Arc::new(Mutex::new(None)),
            socket_task: Arc::new(Mutex::new(None)),
        }
    }

    async fn start_socket_mode(&self, app_token: &str) -> Result<()> {
        let open = self
            .http
            .post("https://slack.com/api/apps.connections.open")
            .header("Authorization", format!("Bearer {}", app_token))
            .send()
            .await?
            .json::<SocketOpenResponse>()
            .await?;

        if !open.ok {
            anyhow::bail!(
                "Slack Socket Mode open failed: {}",
                open.error.unwrap_or_else(|| "unknown".to_string())
            );
        }

        let url = open
            .url
            .ok_or_else(|| anyhow::anyhow!("Slack Socket Mode response missing url"))?;

        let incoming = self.incoming.clone();
        let task = tokio::spawn(async move {
            let Ok((ws, _)) = connect_async(&url).await else {
                warn!("Slack Socket Mode websocket connection failed");
                return;
            };

            let (mut write, mut read) = ws.split();
            while let Some(next) = read.next().await {
                let msg = match next {
                    Ok(m) => m,
                    Err(err) => {
                        warn!("Slack Socket Mode read error: {}", err);
                        break;
                    }
                };

                let text = match msg {
                    Message::Text(text) => text,
                    Message::Binary(bytes) => match String::from_utf8(bytes.to_vec()) {
                        Ok(text) => text.into(),
                        Err(_) => continue,
                    },
                    Message::Close(_) => break,
                    _ => continue,
                };

                let envelope = match serde_json::from_str::<SocketEnvelope>(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if let Some(envelope_id) = envelope.envelope_id {
                    let ack = serde_json::json!({ "envelope_id": envelope_id });
                    if let Err(err) = write.send(Message::Text(ack.to_string().into())).await {
                        warn!("Slack Socket Mode ack send error: {}", err);
                        break;
                    }
                }

                if let Some(payload) = envelope.payload {
                    if let Ok(event) = serde_json::from_value::<SlackEvent>(payload) {
                        if let Err(err) = Self::enqueue_event(incoming.clone(), event).await {
                            warn!("Slack Socket Mode event handling error: {}", err);
                        }
                    }
                }
            }
        });

        *self.socket_task.lock().await = Some(task);
        info!("Slack Socket Mode connected");
        Ok(())
    }

    async fn enqueue_event(
        incoming_store: Arc<Mutex<Vec<IncomingMessage>>>,
        event: SlackEvent,
    ) -> Result<()> {
        if event.challenge.is_some() {
            return Ok(());
        }

        if let Some(inner) = event.event {
            if inner.bot_id.is_some() {
                return Ok(());
            }

            if inner.event_type == "message" {
                if let (Some(text), Some(channel), Some(user)) =
                    (inner.text, inner.channel, inner.user)
                {
                    let incoming = IncomingMessage {
                        id: inner.ts.unwrap_or_default(),
                        channel_type: "slack".into(),
                        chat_id: channel,
                        sender_id: user,
                        sender_name: None,
                        channel_name: "slack".into(),
                        content: MessageContent::Text { text },
                        timestamp: chrono::Utc::now(),
                        reply_to: inner.thread_ts,
                        metadata: std::collections::HashMap::new(),
                        is_group: true,
                        group_name: None,
                    };

                    let mut queue = incoming_store.lock().await;
                    queue.push(incoming);
                    if queue.len() > 10_000 {
                        queue.drain(..5_000);
                    }
                }
            }
        }

        Ok(())
    }

    /// Post a message to Slack
    async fn post_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> Result<SlackResponse> {
        let mut body = serde_json::json!({
            "channel": channel,
            "text": text,
        });

        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_string());
        }

        let resp = self
            .http
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .json(&body)
            .send()
            .await?
            .json::<SlackResponse>()
            .await?;

        if !resp.ok {
            anyhow::bail!("Slack API error: {}", resp.error.unwrap_or("unknown".into()));
        }

        Ok(resp)
    }

    /// Post a structured Block Kit message.
    async fn post_message_blocks(
        &self,
        channel: &str,
        text: Option<&str>,
        blocks: serde_json::Value,
        thread_ts: Option<&str>,
    ) -> Result<SlackResponse> {
        let mut body = serde_json::json!({
            "channel": channel,
            "text": text.unwrap_or("DX structured message"),
            "blocks": blocks,
        });

        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_string());
        }

        let resp = self
            .http
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .json(&body)
            .send()
            .await?
            .json::<SlackResponse>()
            .await?;

        if !resp.ok {
            anyhow::bail!("Slack API error: {}", resp.error.unwrap_or("unknown".into()));
        }

        Ok(resp)
    }

    /// Upload a file to Slack
    async fn upload_file(
        &self,
        channel: &str,
        data: &[u8],
        filename: &str,
        title: Option<&str>,
    ) -> Result<()> {
        let form = reqwest::multipart::Form::new()
            .text("channels", channel.to_string())
            .text("filename", filename.to_string())
            .text("title", title.unwrap_or(filename).to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(data.to_vec()).file_name(filename.to_string()),
            );

        let resp = self
            .http
            .post("https://slack.com/api/files.upload")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .multipart(form)
            .send()
            .await?
            .json::<SlackResponse>()
            .await?;

        if !resp.ok {
            anyhow::bail!("Slack file upload error: {}", resp.error.unwrap_or("unknown".into()));
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    fn display_name(&self) -> &str {
        "Slack"
    }

    fn is_enabled(&self) -> bool {
        !self.config.bot_token.is_empty()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            text: true,
            markdown: true, // Slack mrkdwn
            images: true,
            audio: true,
            video: true,
            files: true,
            reactions: true,
            structured: true, // Block Kit
            edit: true,
            delete: true,
            typing: true,
            read_receipts: false,
            groups: true,
            voice: false,
            webhooks: true,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "slack".into(),
            display_name: "Slack".into(),
            description: "Native Rust Slack integration via Web API".into(),
            version: "0.1.0".into(),
            author: "DX Team".into(),
            icon: Some("slack".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        // Verify bot token by calling auth.test
        let resp = self
            .http
            .post("https://slack.com/api/auth.test")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .send()
            .await?;

        #[derive(Deserialize)]
        struct AuthTest {
            ok: bool,
            user_id: Option<String>,
            user: Option<String>,
            team: Option<String>,
            error: Option<String>,
        }

        let auth: AuthTest = resp.json().await?;
        if !auth.ok {
            anyhow::bail!("Slack auth failed: {}", auth.error.unwrap_or("unknown".into()));
        }

        if let Some(ref user_id) = auth.user_id {
            *self.bot_user_id.lock().await = Some(user_id.clone());
        }

        info!(
            "Slack bot connected: {} in team {}",
            auth.user.unwrap_or_default(),
            auth.team.unwrap_or_default()
        );

        if let Some(app_token) = &self.config.app_token {
            self.start_socket_mode(app_token).await?;
        }

        self.connected.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(task) = self.socket_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        self.connected.store(false, Ordering::Relaxed);
        *self.bot_user_id.lock().await = None;
        info!("Slack channel disconnected");
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        match &message.content {
            MessageContent::Text { text } => {
                self.post_message(&message.to, text, message.reply_to.as_deref()).await?;
            }
            MessageContent::Markdown { text } => {
                // Slack uses mrkdwn format
                self.post_message(&message.to, text, message.reply_to.as_deref()).await?;
            }
            MessageContent::Media(media) => {
                if let Some(ref data) = media.data {
                    let filename = media.filename.as_deref().unwrap_or("file");
                    self.upload_file(&message.to, data, filename, media.caption.as_deref()).await?;
                } else if let Some(ref url) = media.url {
                    // Send URL as text
                    self.post_message(&message.to, url, None).await?;
                }
            }
            MessageContent::Structured { data } => {
                if let Some(blocks) = data.get("blocks") {
                    let text = data.get("text").and_then(|v| v.as_str());
                    self.post_message_blocks(
                        &message.to,
                        text,
                        blocks.clone(),
                        message.reply_to.as_deref(),
                    )
                    .await?;
                } else if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                    self.post_message(&message.to, text, message.reply_to.as_deref()).await?;
                } else {
                    self.post_message(&message.to, &data.to_string(), message.reply_to.as_deref())
                        .await?;
                }
            }
            MessageContent::Interactive { text, keyboard } => {
                // Convert buttons to simple Block Kit actions payload.
                let mut elements = Vec::new();
                for row in &keyboard.rows {
                    for button in row {
                        let element = match &button.action {
                            ButtonAction::Url { url } => serde_json::json!({
                                "type": "button",
                                "text": { "type": "plain_text", "text": button.text },
                                "url": url,
                            }),
                            ButtonAction::Callback { data } => serde_json::json!({
                                "type": "button",
                                "text": { "type": "plain_text", "text": button.text },
                                "value": data,
                                "action_id": format!("cb_{}", data),
                            }),
                            ButtonAction::SwitchInline { query } => serde_json::json!({
                                "type": "button",
                                "text": { "type": "plain_text", "text": button.text },
                                "value": query,
                                "action_id": "switch_inline",
                            }),
                            ButtonAction::Copy { text: value } => serde_json::json!({
                                "type": "button",
                                "text": { "type": "plain_text", "text": button.text },
                                "value": value,
                                "action_id": "copy_text",
                            }),
                        };
                        elements.push(element);
                    }
                }

                let blocks = serde_json::json!([
                    {
                        "type": "section",
                        "text": { "type": "mrkdwn", "text": text }
                    },
                    {
                        "type": "actions",
                        "elements": elements
                    }
                ]);

                self.post_message_blocks(
                    &message.to,
                    Some(text),
                    blocks,
                    message.reply_to.as_deref(),
                )
                .await?;
            }
            _ => {
                warn!("Unsupported content type for Slack");
                return Ok(DeliveryStatus::Failed("Unsupported content".into()));
            }
        }

        Ok(DeliveryStatus::Sent)
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        let mut queue = self.incoming.lock().await;
        let messages = queue.drain(..).collect();
        Ok(messages)
    }

    async fn handle_webhook(&self, payload: serde_json::Value) -> Result<()> {
        let event: SlackEvent = serde_json::from_value(payload)?;
        Self::enqueue_event(self.incoming.clone(), event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_config() {
        let config = SlackConfig {
            bot_token: "xoxb-test".into(),
            app_token: None,
            signing_secret: None,
            default_channel: Some("#general".into()),
        };
        let channel = SlackChannel::new(config);
        assert_eq!(channel.name(), "slack");
        assert!(channel.is_enabled());
    }

    #[test]
    fn test_slack_capabilities() {
        let config = SlackConfig {
            bot_token: "test".into(),
            app_token: None,
            signing_secret: None,
            default_channel: None,
        };
        let channel = SlackChannel::new(config);
        let caps = channel.capabilities();
        assert!(caps.text);
        assert!(caps.structured);
        assert!(caps.files);
        assert!(!caps.voice); // Slack doesn't support voice
    }
}
