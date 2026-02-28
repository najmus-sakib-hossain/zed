//! WhatsApp Channel implementation

use async_trait::async_trait;

use crate::whatsapp::{WhatsAppClient, WhatsAppConfig};

use super::trait_def::{Channel, ChannelMessage, DeliveryStatus, MessageContent};

/// WhatsApp messaging channel
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
}

impl Default for WhatsAppChannel {
    fn default() -> Self {
        Self {
            config: WhatsAppConfig::load_or_default(),
        }
    }
}

impl WhatsAppChannel {
    /// Create a new WhatsApp channel with config
    pub fn new(config: WhatsAppConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn is_enabled(&self) -> bool {
        true
    }

    async fn send(&self, message: ChannelMessage) -> anyhow::Result<DeliveryStatus> {
        let client = WhatsAppClient::new(self.config.clone()).await?;

        match message.content {
            MessageContent::Text(text) | MessageContent::Markdown(text) => {
                client.send_message(&message.to, &text).await?;
                Ok(DeliveryStatus::Delivered)
            }
            MessageContent::Binary { .. } => Ok(DeliveryStatus::Failed(
                "Binary messages not supported for WhatsApp in this implementation".to_string(),
            )),
            MessageContent::Structured(value) => {
                let text = value.to_string();
                client.send_message(&message.to, &text).await?;
                Ok(DeliveryStatus::Delivered)
            }
            MessageContent::Media { url, caption, .. } => {
                // TODO: Implement media upload for WhatsApp
                let _ = (url, caption);
                Ok(DeliveryStatus::Failed(
                    "Media messages not yet implemented for WhatsApp".to_string(),
                ))
            }
            MessageContent::Reaction(emoji) => {
                // TODO: Implement reactions for WhatsApp
                let _ = emoji;
                Ok(DeliveryStatus::Failed("Reactions not yet implemented for WhatsApp".to_string()))
            }
        }
    }
}
