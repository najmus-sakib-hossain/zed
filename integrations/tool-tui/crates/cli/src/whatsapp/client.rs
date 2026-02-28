//! WhatsApp client (supports both Business API and Personal via gateway)

use anyhow::{Context, Result};
use whatsapp_business_rs::Client as BusinessClient;

use super::config::{AccountType, WhatsAppConfig};

/// WhatsApp client for sending messages
pub struct WhatsAppClient {
    config: WhatsAppConfig,
    business_client: Option<BusinessClient>,
}

impl WhatsAppClient {
    /// Create a new WhatsApp client
    pub async fn new(config: WhatsAppConfig) -> Result<Self> {
        let business_client = if config.account_type == AccountType::Business {
            let token =
                config.access_token.as_ref().context("Business account requires access_token")?;
            Some(BusinessClient::new(token).await?)
        } else {
            None
        };

        Ok(Self {
            config,
            business_client,
        })
    }

    /// Send a text message
    pub async fn send_message(&self, to: &str, message: &str) -> Result<()> {
        match self.config.account_type {
            AccountType::Business => self.send_business_message(to, message).await,
            AccountType::Personal => self.send_personal_message(to, message).await,
        }
    }

    /// Send via Business API
    async fn send_business_message(&self, to: &str, message: &str) -> Result<()> {
        let client = self.business_client.as_ref().context("Business client not initialized")?;

        let phone_id = self.config.phone_number_id.as_ref().context("Phone number ID required")?;

        client
            .message(phone_id.as_str())
            .send(to, message)
            .await
            .context("Failed to send WhatsApp message")?;

        Ok(())
    }

    /// Send via Personal gateway
    async fn send_personal_message(&self, to: &str, message: &str) -> Result<()> {
        let gateway_url = self
            .config
            .gateway_url
            .as_ref()
            .context("Gateway URL required for personal account")?;

        let url = format!("{}/send", gateway_url);

        let mut request = reqwest::Client::new().post(&url).json(&serde_json::json!({
            "phone": to,
            "message": message
        }));

        // Add API key if configured
        if let Some(api_key) = &self.config.gateway_api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await.context("Failed to connect to WhatsApp gateway")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Gateway returned error {}: {}", status, body));
        }

        Ok(())
    }
}
