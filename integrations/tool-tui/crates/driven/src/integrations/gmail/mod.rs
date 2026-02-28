//! # Gmail Integration
//!
//! Gmail Pub/Sub listener and email operations.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::gmail::{GmailClient, GmailConfig};
//!
//! let config = GmailConfig::from_file("~/.dx/config/gmail.sr")?;
//! let client = GmailClient::new(&config).await?;
//!
//! // Watch for new emails
//! client.watch(&["INBOX"], |email| {
//!     println!("New email: {}", email.subject);
//! }).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gmail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailConfig {
    /// Whether Gmail integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// OAuth2 client ID
    #[serde(default)]
    pub client_id: String,
    /// OAuth2 client secret
    #[serde(default)]
    pub client_secret: String,
    /// OAuth2 refresh token
    #[serde(default)]
    pub refresh_token: String,
    /// Pub/Sub topic for push notifications
    pub pubsub_topic: Option<String>,
    /// Email filters
    #[serde(default)]
    pub filters: Vec<GmailFilter>,
}

fn default_true() -> bool {
    true
}

impl Default for GmailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: String::new(),
            client_secret: String::new(),
            refresh_token: String::new(),
            pubsub_topic: None,
            filters: Vec::new(),
        }
    }
}

impl GmailConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.client_id.is_empty() || self.client_id.starts_with('$') {
            self.client_id = std::env::var("GMAIL_CLIENT_ID").unwrap_or_default();
        }
        if self.client_secret.is_empty() || self.client_secret.starts_with('$') {
            self.client_secret = std::env::var("GMAIL_CLIENT_SECRET").unwrap_or_default();
        }
        if self.refresh_token.is_empty() || self.refresh_token.starts_with('$') {
            self.refresh_token = std::env::var("GMAIL_REFRESH_TOKEN").unwrap_or_default();
        }
    }
}

/// Gmail filter for incoming emails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailFilter {
    /// Filter name
    pub name: String,
    /// From address pattern
    pub from: Option<String>,
    /// Subject pattern
    pub subject: Option<String>,
    /// Label to match
    pub label: Option<String>,
    /// Action to perform
    pub action: GmailAction,
    /// Whether filter is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Gmail filter action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GmailAction {
    /// Forward to another email
    Forward { to: String },
    /// Run a command
    Command { cmd: String },
    /// Trigger a webhook
    Webhook { url: String },
    /// Archive the email
    Archive,
    /// Mark as read
    MarkRead,
}

/// Gmail message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessage {
    /// Message ID
    pub id: String,
    /// Thread ID
    pub thread_id: String,
    /// Subject
    pub subject: String,
    /// From address
    pub from: String,
    /// To addresses
    pub to: Vec<String>,
    /// CC addresses
    pub cc: Vec<String>,
    /// Date
    pub date: String,
    /// Snippet (preview)
    pub snippet: String,
    /// Labels
    pub labels: Vec<String>,
    /// Body (plain text)
    pub body_plain: Option<String>,
    /// Body (HTML)
    pub body_html: Option<String>,
    /// Attachments
    pub attachments: Vec<GmailAttachment>,
}

/// Gmail attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailAttachment {
    /// Attachment ID
    pub id: String,
    /// Filename
    pub filename: String,
    /// MIME type
    pub mime_type: String,
    /// Size in bytes
    pub size: u64,
}

/// Gmail label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailLabel {
    /// Label ID
    pub id: String,
    /// Label name
    pub name: String,
    /// Label type (system/user)
    pub label_type: String,
}

/// Gmail client
pub struct GmailClient {
    config: GmailConfig,
    access_token: Option<String>,
    base_url: String,
}

impl GmailClient {
    /// API base URL
    const API_BASE: &'static str = "https://gmail.googleapis.com/gmail/v1";
    /// OAuth token URL
    const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";

    /// Create a new Gmail client
    pub async fn new(config: &GmailConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        let mut client = Self {
            config,
            access_token: None,
            base_url: Self::API_BASE.to_string(),
        };

        // Get initial access token
        if client.is_configured() {
            client.refresh_access_token().await?;
        }

        Ok(client)
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.config.client_id.is_empty()
            && !self.config.client_secret.is_empty()
            && !self.config.refresh_token.is_empty()
    }

    /// Refresh access token
    async fn refresh_access_token(&mut self) -> Result<()> {
        let client = reqwest::Client::new();

        let mut params = HashMap::new();
        params.insert("client_id", self.config.client_id.as_str());
        params.insert("client_secret", self.config.client_secret.as_str());
        params.insert("refresh_token", self.config.refresh_token.as_str());
        params.insert("grant_type", "refresh_token");

        let response = client
            .post(Self::TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to refresh token".into()));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.access_token = Some(tokens.access_token);
        Ok(())
    }

    /// List messages
    pub async fn list_messages(&self, query: Option<&str>, max_results: u32) -> Result<Vec<GmailMessage>> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let mut url = format!(
            "{}/users/me/messages?maxResults={}",
            self.base_url, max_results
        );

        if let Some(q) = query {
            url.push_str(&format!("&q={}", urlencoding::encode(q)));
        }

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list messages".into()));
        }

        #[derive(Deserialize)]
        struct ListResponse {
            messages: Option<Vec<MessageId>>,
        }

        #[derive(Deserialize)]
        struct MessageId {
            id: String,
        }

        let list: ListResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        let mut messages = Vec::new();
        if let Some(msg_ids) = list.messages {
            for msg_id in msg_ids.into_iter().take(max_results as usize) {
                if let Ok(msg) = self.get_message(&msg_id.id).await {
                    messages.push(msg);
                }
            }
        }

        Ok(messages)
    }

    /// Get a specific message
    pub async fn get_message(&self, id: &str) -> Result<GmailMessage> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let url = format!(
            "{}/users/me/messages/{}?format=full",
            self.base_url, id
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to get message".into()));
        }

        let raw: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.parse_message(raw)
    }

    /// Send an email
    pub async fn send(&self, to: &[&str], subject: &str, body: &str) -> Result<String> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        // Build RFC 2822 message
        let message = format!(
            "To: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
            to.join(", "),
            subject,
            body
        );

        let encoded = base64::encode_config(message.as_bytes(), base64::URL_SAFE_NO_PAD);

        let url = format!("{}/users/me/messages/send", self.base_url);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "raw": encoded }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to send message".into()));
        }

        #[derive(Deserialize)]
        struct SendResponse {
            id: String,
        }

        let result: SendResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(result.id)
    }

    /// Archive a message
    pub async fn archive(&self, id: &str) -> Result<()> {
        self.modify_labels(id, &[], &["INBOX"]).await
    }

    /// Mark message as read
    pub async fn mark_read(&self, id: &str) -> Result<()> {
        self.modify_labels(id, &[], &["UNREAD"]).await
    }

    /// Mark message as unread
    pub async fn mark_unread(&self, id: &str) -> Result<()> {
        self.modify_labels(id, &["UNREAD"], &[]).await
    }

    /// Modify message labels
    pub async fn modify_labels(&self, id: &str, add: &[&str], remove: &[&str]) -> Result<()> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let url = format!("{}/users/me/messages/{}/modify", self.base_url, id);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "addLabelIds": add,
                "removeLabelIds": remove
            }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to modify labels".into()));
        }

        Ok(())
    }

    /// List labels
    pub async fn list_labels(&self) -> Result<Vec<GmailLabel>> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let url = format!("{}/users/me/labels", self.base_url);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list labels".into()));
        }

        #[derive(Deserialize)]
        struct LabelsResponse {
            labels: Vec<LabelItem>,
        }

        #[derive(Deserialize)]
        struct LabelItem {
            id: String,
            name: String,
            #[serde(rename = "type")]
            label_type: Option<String>,
        }

        let result: LabelsResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(result
            .labels
            .into_iter()
            .map(|l| GmailLabel {
                id: l.id,
                name: l.name,
                label_type: l.label_type.unwrap_or_else(|| "user".to_string()),
            })
            .collect())
    }

    /// Setup push notifications via Pub/Sub
    pub async fn watch(&self, labels: &[&str]) -> Result<WatchResponse> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let topic = self.config.pubsub_topic.as_ref()
            .ok_or_else(|| DrivenError::Config("Pub/Sub topic not configured".into()))?;

        let url = format!("{}/users/me/watch", self.base_url);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "topicName": topic,
                "labelIds": labels
            }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to setup watch".into()));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Parse raw message response
    fn parse_message(&self, raw: serde_json::Value) -> Result<GmailMessage> {
        let id = raw["id"].as_str().unwrap_or_default().to_string();
        let thread_id = raw["threadId"].as_str().unwrap_or_default().to_string();
        let snippet = raw["snippet"].as_str().unwrap_or_default().to_string();

        let labels: Vec<String> = raw["labelIds"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Parse headers
        let headers = raw["payload"]["headers"].as_array();
        let mut subject = String::new();
        let mut from = String::new();
        let mut to = Vec::new();
        let mut cc = Vec::new();
        let mut date = String::new();

        if let Some(hdrs) = headers {
            for h in hdrs {
                let name = h["name"].as_str().unwrap_or_default().to_lowercase();
                let value = h["value"].as_str().unwrap_or_default();
                match name.as_str() {
                    "subject" => subject = value.to_string(),
                    "from" => from = value.to_string(),
                    "to" => to = value.split(',').map(|s| s.trim().to_string()).collect(),
                    "cc" => cc = value.split(',').map(|s| s.trim().to_string()).collect(),
                    "date" => date = value.to_string(),
                    _ => {}
                }
            }
        }

        Ok(GmailMessage {
            id,
            thread_id,
            subject,
            from,
            to,
            cc,
            date,
            snippet,
            labels,
            body_plain: None,
            body_html: None,
            attachments: Vec::new(),
        })
    }
}

/// Watch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchResponse {
    /// History ID
    #[serde(rename = "historyId")]
    pub history_id: String,
    /// Expiration time
    pub expiration: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GmailConfig::default();
        assert!(config.enabled);
        assert!(config.filters.is_empty());
    }
}
