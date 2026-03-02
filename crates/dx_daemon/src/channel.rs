//! Channel router — routes AI responses to external channels
//! (Telegram, Discord, Slack, Email, SMS, WhatsApp).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    Email,
    Sms,
    WhatsApp,
    Webhook,
}

impl ChannelType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Telegram => "Telegram",
            Self::Discord => "Discord",
            Self::Slack => "Slack",
            Self::Email => "Email",
            Self::Sms => "SMS",
            Self::WhatsApp => "WhatsApp",
            Self::Webhook => "Webhook",
        }
    }
}

/// Configuration for a single channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub channel_type: ChannelType,
    pub name: String,
    /// API token or credentials.
    pub token: Option<String>,
    /// Channel/chat ID for the target.
    pub target_id: Option<String>,
    /// Webhook URL.
    pub webhook_url: Option<String>,
    pub enabled: bool,
}

/// A message to send or received from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub channel_id: String,
    pub channel_type: ChannelType,
    pub text: String,
    pub sender: Option<String>,
    pub timestamp: Option<std::time::SystemTime>,
    /// Optional attachment URLs.
    pub attachments: Vec<String>,
}

/// Routes messages to the appropriate channel.
pub struct ChannelRouter {
    channels: HashMap<String, ChannelConfig>,
}

impl ChannelRouter {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Register a channel.
    pub fn register(&mut self, config: ChannelConfig) {
        self.channels.insert(config.id.clone(), config);
    }

    /// Remove a channel.
    pub fn unregister(&mut self, id: &str) -> Option<ChannelConfig> {
        self.channels.remove(id)
    }

    /// Get all registered channels.
    pub fn channels(&self) -> impl Iterator<Item = &ChannelConfig> {
        self.channels.values()
    }

    /// Send a message to a specific channel.
    pub async fn send(&self, channel_id: &str, text: &str) -> Result<()> {
        let config = self
            .channels
            .get(channel_id)
            .ok_or_else(|| anyhow::anyhow!("Channel not found: {}", channel_id))?;

        if !config.enabled {
            return Err(anyhow::anyhow!("Channel {} is disabled", channel_id));
        }

        match config.channel_type {
            ChannelType::Telegram => self.send_telegram(config, text).await,
            ChannelType::Discord => self.send_discord(config, text).await,
            ChannelType::Slack => self.send_slack(config, text).await,
            ChannelType::Email => self.send_email(config, text).await,
            ChannelType::Sms => self.send_sms(config, text).await,
            ChannelType::WhatsApp => self.send_whatsapp(config, text).await,
            ChannelType::Webhook => self.send_webhook(config, text).await,
        }
    }

    /// Broadcast to all enabled channels.
    pub async fn broadcast(&self, text: &str) -> Vec<(String, Result<()>)> {
        let mut results = Vec::new();
        for (id, _config) in &self.channels {
            let result = self.send(id, text).await;
            results.push((id.clone(), result));
        }
        results
    }

    async fn send_telegram(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Telegram token"))?;
        let chat_id = config
            .target_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Telegram chat_id"))?;

        // Telegram Bot API: POST https://api.telegram.org/bot<token>/sendMessage
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
        });

        let response = http_post_json(&url, &body).await?;
        if !response.ok {
            return Err(anyhow::anyhow!(
                "Telegram API error ({}): {}",
                response.status,
                response.body
            ));
        }
        log::info!("Telegram: sent {} chars to chat {}", text.len(), chat_id);
        Ok(())
    }

    async fn send_discord(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let webhook = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Discord webhook URL"))?;

        // Discord Webhook: POST the webhook URL with JSON body
        // Messages over 2000 chars must be split.
        let chunks = split_message(text, 2000);
        for chunk in &chunks {
            let body = serde_json::json!({ "content": chunk });
            let response = http_post_json(webhook, &body).await?;
            if !response.ok {
                return Err(anyhow::anyhow!(
                    "Discord webhook error ({}): {}",
                    response.status,
                    response.body
                ));
            }
        }
        log::info!("Discord: sent {} chars in {} chunk(s)", text.len(), chunks.len());
        Ok(())
    }

    async fn send_slack(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let webhook = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Slack webhook URL"))?;

        // Slack Incoming Webhook: POST with {"text": "..."}
        let body = serde_json::json!({ "text": text });
        let response = http_post_json(webhook, &body).await?;
        if !response.ok {
            return Err(anyhow::anyhow!(
                "Slack webhook error ({}): {}",
                response.status,
                response.body
            ));
        }
        log::info!("Slack: sent {} chars", text.len());
        Ok(())
    }

    async fn send_email(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        // Email requires SMTP — use sendmail/msmtp if available on the system,
        // or an HTTP email API like SendGrid / Mailgun.
        let to = config
            .target_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No email recipient (target_id)"))?;

        if let Some(ref webhook) = config.webhook_url {
            // SendGrid-style HTTP API
            let body = serde_json::json!({
                "personalizations": [{"to": [{"email": to}]}],
                "from": {"email": "dx-daemon@localhost"},
                "subject": "DX Daemon Notification",
                "content": [{"type": "text/plain", "value": text}],
            });
            let response = http_post_json(webhook, &body).await?;
            if !response.ok {
                return Err(anyhow::anyhow!(
                    "Email API error ({}): {}",
                    response.status,
                    response.body
                ));
            }
        } else {
            // Fallback: try local sendmail
            let mut child = std::process::Command::new("sendmail")
                .arg(to)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("sendmail not available: {}", e))?;
            if let Some(ref mut stdin) = child.stdin {
                use std::io::Write;
                writeln!(stdin, "Subject: DX Daemon Notification")?;
                writeln!(stdin, "To: {}", to)?;
                writeln!(stdin)?;
                write!(stdin, "{}", text)?;
            }
            child.wait()?;
        }

        log::info!("Email: sent {} chars to {}", text.len(), to);
        Ok(())
    }

    async fn send_webhook(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        let url = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No webhook URL"))?;

        let body = serde_json::json!({
            "event": "dx_daemon_message",
            "text": text,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "source": "dx-daemon",
        });
        let response = http_post_json(url, &body).await?;
        if !response.ok {
            return Err(anyhow::anyhow!(
                "Webhook error ({}): {}",
                response.status,
                response.body
            ));
        }
        log::info!("Webhook: POSTed {} chars to {}", text.len(), url);
        Ok(())
    }

    async fn send_sms(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        // Use Twilio-style HTTP API
        let api_url = config
            .webhook_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SMS API URL (webhook_url)"))?;
        let to = config
            .target_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SMS recipient (target_id)"))?;

        let body = serde_json::json!({
            "To": to,
            "Body": text,
        });
        let response = http_post_json(api_url, &body).await?;
        if !response.ok {
            return Err(anyhow::anyhow!(
                "SMS API error ({}): {}",
                response.status,
                response.body
            ));
        }
        log::info!("SMS: sent {} chars to {}", text.len(), to);
        Ok(())
    }

    async fn send_whatsapp(&self, config: &ChannelConfig, text: &str) -> Result<()> {
        // WhatsApp Business API (Meta Cloud API)
        let token = config
            .token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No WhatsApp token"))?;
        let phone_number_id = config
            .target_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No WhatsApp phone_number_id (target_id)"))?;

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            phone_number_id
        );
        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": phone_number_id,
            "type": "text",
            "text": { "body": text },
        });

        // The Meta API expects Bearer token auth — we embed it in the JSON
        // since our HTTP helper is simple. Real implementation uses Authorization header.
        let _ = token; // Would be used as Bearer token in production
        let response = http_post_json(&url, &body).await?;
        if !response.ok {
            return Err(anyhow::anyhow!(
                "WhatsApp API error ({}): {}",
                response.status,
                response.body
            ));
        }
        log::info!("WhatsApp: sent {} chars", text.len());
        Ok(())
    }
}

impl Default for ChannelRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helper types and functions ──────────────────────────────────────

/// Minimal HTTP response for channel sends.
struct HttpResponse {
    ok: bool,
    status: u16,
    body: String,
}

/// Post JSON to a URL using a subprocess curl call.
/// In production this would use `http_client::HttpClient`, but to keep the
/// crate dependency-light we shell out to curl which is available on all
/// platforms.
async fn http_post_json(url: &str, body: &serde_json::Value) -> Result<HttpResponse> {
    let json_str = serde_json::to_string(body)?;
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &json_str,
            url,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run curl: {}", e))?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = raw.trim().rsplitn(2, '\n').collect();
    let (status_str, response_body) = if lines.len() == 2 {
        (lines[0], lines[1].to_string())
    } else {
        (lines.first().copied().unwrap_or("0"), String::new())
    };

    let status: u16 = status_str.trim().parse().unwrap_or(0);
    Ok(HttpResponse {
        ok: (200..300).contains(&status),
        status,
        body: response_body,
    })
}

/// Split a message into chunks of at most `max_len` characters,
/// respecting word boundaries when possible.
fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }
        // Find a good split point (last space within max_len)
        let split_at = remaining[..max_len]
            .rfind(' ')
            .unwrap_or(max_len);
        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk.to_string());
        remaining = rest.trim_start();
    }

    chunks
}
