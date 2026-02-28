//! Generic webhook channel for custom integrations.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tracing::info;

use crate::message::*;
use crate::traits::*;

/// Webhook channel configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookConfig {
    /// Name for this webhook channel
    pub name: String,
    /// Webhook URL to send messages to
    pub outgoing_url: Option<String>,
    /// Secret for webhook signature verification
    pub secret: Option<String>,
}

/// Generic webhook channel
pub struct WebhookChannel {
    config: WebhookConfig,
    http: reqwest::Client,
    connected: Arc<AtomicBool>,
    incoming: Arc<Mutex<Vec<IncomingMessage>>>,
}

impl WebhookChannel {
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            connected: Arc::new(AtomicBool::new(false)),
            incoming: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Channel for WebhookChannel {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn display_name(&self) -> &str {
        &self.config.name
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            text: true,
            webhooks: true,
            ..Default::default()
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: self.config.name.clone(),
            display_name: self.config.name.clone(),
            description: "Custom webhook channel".into(),
            version: "0.1.0".into(),
            author: "DX Team".into(),
            icon: None,
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        self.connected.store(true, Ordering::Relaxed);
        info!("Webhook channel '{}' connected", self.config.name);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        if let Some(ref url) = self.config.outgoing_url {
            let body = serde_json::json!({
                "to": message.to,
                "content": message.content,
                "metadata": message.metadata,
            });

            let resp = self.http.post(url).json(&body).send().await?;
            if resp.status().is_success() {
                Ok(DeliveryStatus::Sent)
            } else {
                Ok(DeliveryStatus::Failed(format!("HTTP {}", resp.status())))
            }
        } else {
            Ok(DeliveryStatus::Failed("No outgoing URL configured".into()))
        }
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        let mut queue = self.incoming.lock().await;
        Ok(queue.drain(..).collect())
    }

    async fn handle_webhook(&self, payload: serde_json::Value) -> Result<()> {
        // Generic webhook - try to extract a message
        let text = payload
            .get("text")
            .or_else(|| payload.get("message"))
            .or_else(|| payload.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let from = payload
            .get("from")
            .or_else(|| payload.get("sender"))
            .or_else(|| payload.get("user"))
            .and_then(|v| v.as_str())
            .unwrap_or("webhook")
            .to_string();

        if !text.is_empty() {
            let incoming = IncomingMessage {
                id: uuid::Uuid::new_v4().to_string(),
                channel_type: "webhook".into(),
                chat_id: from.clone(),
                sender_id: from,
                sender_name: None,
                channel_name: self.config.name.clone(),
                content: MessageContent::Text { text },
                timestamp: chrono::Utc::now(),
                reply_to: None,
                metadata: std::collections::HashMap::new(),
                is_group: false,
                group_name: None,
            };

            let mut queue = self.incoming.lock().await;
            queue.push(incoming);
        }

        Ok(())
    }
}
