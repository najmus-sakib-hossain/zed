//! Generic Webhook Channel implementation

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

use super::trait_def::{Channel, ChannelMessage, DeliveryStatus, MessageContent};

/// Webhook channel for generic HTTP notifications
pub struct WebhookChannel {
    default_url: Option<String>,
    auth_header: Option<(String, String)>,
}

impl Default for WebhookChannel {
    fn default() -> Self {
        let default_url = std::env::var("WEBHOOK_URL").ok();
        let auth_key = std::env::var("WEBHOOK_AUTH_HEADER").ok();
        let auth_value = std::env::var("WEBHOOK_AUTH_VALUE").ok();
        let auth_header = match (auth_key, auth_value) {
            (Some(k), Some(v)) => Some((k, v)),
            _ => None,
        };

        Self {
            default_url,
            auth_header,
        }
    }
}

impl WebhookChannel {
    /// Create a new webhook channel
    pub fn new(default_url: Option<String>) -> Self {
        Self {
            default_url,
            auth_header: None,
        }
    }

    /// Set auth header
    pub fn with_auth_header(mut self, key: String, value: String) -> Self {
        self.auth_header = Some((key, value));
        self
    }

    fn resolve_url(&self, to: &str) -> anyhow::Result<String> {
        if !to.is_empty() {
            Ok(to.to_string())
        } else if let Some(url) = &self.default_url {
            Ok(url.clone())
        } else {
            Err(anyhow::anyhow!("Webhook URL not provided"))
        }
    }
}

#[async_trait]
impl Channel for WebhookChannel {
    fn name(&self) -> &str {
        "webhook"
    }

    async fn send(&self, message: ChannelMessage) -> anyhow::Result<DeliveryStatus> {
        let url = self.resolve_url(&message.to)?;

        let mut req = reqwest::Client::new().post(&url);

        if let Some((key, value)) = &self.auth_header {
            req = req.header(key, value);
        }

        match message.content {
            MessageContent::Text(t) | MessageContent::Markdown(t) => {
                req = req.json(&serde_json::json!({ "text": t }));
            }
            MessageContent::Media { url, mime, caption } => {
                req = req.json(&serde_json::json!({
                    "media_url": url,
                    "mime": mime,
                    "caption": caption
                }));
            }
            MessageContent::Reaction(emoji) => {
                req = req.json(&serde_json::json!({ "reaction": emoji }));
            }
            MessageContent::Binary {
                mime,
                data,
                filename,
            } => {
                // Send as base64 payload
                req = req.json(&serde_json::json!({
                    "mime": mime,
                    "data": BASE64_STANDARD.encode(data),
                    "filename": filename
                }));
            }
            MessageContent::Structured(v) => {
                req = req.json(&v);
            }
        }

        let response = req.send().await?;

        if response.status().is_success() {
            Ok(DeliveryStatus::Delivered)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Ok(DeliveryStatus::Failed(format!("Webhook error {}: {}", status, body)))
        }
    }
}
