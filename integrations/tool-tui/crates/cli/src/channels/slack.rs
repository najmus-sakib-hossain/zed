//! Slack Channel implementation

use async_trait::async_trait;

use super::trait_def::{Channel, ChannelMessage, DeliveryStatus, MessageContent};

/// Slack messaging channel (webhook-based)
pub struct SlackChannel {
    webhook_url: Option<String>,
}

impl Default for SlackChannel {
    fn default() -> Self {
        let webhook_url = std::env::var("SLACK_WEBHOOK_URL").ok();
        Self { webhook_url }
    }
}

impl SlackChannel {
    /// Create a new Slack channel with webhook URL
    pub fn new(webhook_url: Option<String>) -> Self {
        Self { webhook_url }
    }

    fn webhook(&self) -> anyhow::Result<&str> {
        self.webhook_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("SLACK_WEBHOOK_URL not set"))
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, message: ChannelMessage) -> anyhow::Result<DeliveryStatus> {
        let url = self.webhook()?;

        let text = match message.content {
            MessageContent::Text(t) | MessageContent::Markdown(t) => t,
            MessageContent::Media { caption, .. } => {
                caption.unwrap_or_else(|| "[media attachment]".to_string())
            }
            MessageContent::Reaction(emoji) => emoji,
            MessageContent::Binary { .. } => {
                return Ok(DeliveryStatus::Failed(
                    "Binary messages not supported in this implementation".to_string(),
                ));
            }
            MessageContent::Structured(v) => v.to_string(),
        };

        let body = serde_json::json!({
            "text": text
        });

        let response = reqwest::Client::new().post(url).json(&body).send().await?;

        if response.status().is_success() {
            Ok(DeliveryStatus::Delivered)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Ok(DeliveryStatus::Failed(format!("Slack error {}: {}", status, body)))
        }
    }
}
