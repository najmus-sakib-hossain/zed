//! Telegram Channel implementation

use async_trait::async_trait;

use super::trait_def::{Channel, ChannelMessage, DeliveryStatus, MessageContent};

/// Telegram messaging channel
pub struct TelegramChannel {
    bot_token: Option<String>,
}

impl Default for TelegramChannel {
    fn default() -> Self {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
        Self { bot_token }
    }
}

impl TelegramChannel {
    /// Create a new Telegram channel with bot token
    pub fn new(bot_token: Option<String>) -> Self {
        Self { bot_token }
    }

    fn token(&self) -> anyhow::Result<&str> {
        self.bot_token
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("TELEGRAM_BOT_TOKEN not set"))
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, message: ChannelMessage) -> anyhow::Result<DeliveryStatus> {
        let token = self.token()?;
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

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
            "chat_id": message.to,
            "text": text,
            "parse_mode": "Markdown"
        });

        let response = reqwest::Client::new().post(&url).json(&body).send().await?;

        if response.status().is_success() {
            Ok(DeliveryStatus::Delivered)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Ok(DeliveryStatus::Failed(format!("Telegram error {}: {}", status, body)))
        }
    }
}
