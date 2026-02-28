//! WhatsApp channel integration.
//!
//! Uses Rust HTTP wrapper for WhatsApp Cloud API,
//! with optional Node.js fallback via Baileys for Web API.

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use dx_agent_protocol::framing::{Frame, FrameError, FrameType};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::message::*;
use crate::traits::*;

/// WhatsApp channel configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhatsAppConfig {
    /// WhatsApp Cloud API access token
    pub access_token: Option<String>,
    /// Phone number ID (from WhatsApp Business)
    pub phone_number_id: Option<String>,
    /// Verify token for webhook
    pub verify_token: Option<String>,
    /// WhatsApp Business Account ID
    pub business_account_id: Option<String>,
    /// Use Baileys (Node.js) for Web API instead of Cloud API
    #[serde(default)]
    pub use_baileys: bool,
}

/// WhatsApp Cloud API response
#[derive(Debug, serde::Deserialize)]
struct CloudApiResponse {
    messages: Option<Vec<CloudApiMessage>>,
    error: Option<CloudApiError>,
}

#[derive(Debug, serde::Deserialize)]
struct CloudApiMessage {
    id: String,
}

#[derive(Debug, serde::Deserialize)]
struct CloudApiError {
    message: String,
    code: i32,
}

struct BaileysBridge {
    child: Child,
    stdin: ChildStdin,
    responses: Arc<DashMap<u64, serde_json::Value>>,
    reader_task: tokio::task::JoinHandle<()>,
    next_id: u64,
    framed_ipc: bool,
}

/// WhatsApp channel implementation (Cloud API)
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
    http: reqwest::Client,
    connected: Arc<AtomicBool>,
    incoming: Arc<Mutex<Vec<IncomingMessage>>>,
    baileys: Arc<Mutex<Option<BaileysBridge>>>,
}

impl WhatsAppChannel {
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            connected: Arc::new(AtomicBool::new(false)),
            incoming: Arc::new(Mutex::new(Vec::new())),
            baileys: Arc::new(Mutex::new(None)),
        }
    }

    fn resolve_baileys_runner_path() -> PathBuf {
        if let Ok(path) = std::env::var("DX_WHATSAPP_RUNNER") {
            return PathBuf::from(path);
        }

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let preferred = manifest.join("../../cli/src/bridge/whatsapp/runner.mjs");
        if let Ok(path) = preferred.canonicalize() {
            return path;
        }

        let legacy = manifest.join("../../cli/src/bridge/whatsapp-runner.mjs");
        legacy.canonicalize().unwrap_or(legacy)
    }

    async fn connect_baileys(&self) -> Result<()> {
        let node_bin = std::env::var("DX_NODE_BIN").unwrap_or_else(|_| "node".to_string());
        let runner = Self::resolve_baileys_runner_path();

        if !runner.exists() {
            anyhow::bail!(
                "Baileys runner not found at {} (set DX_WHATSAPP_RUNNER)",
                runner.display()
            );
        }

        let mut cmd = Command::new(node_bin);
        let framed_ipc = std::env::var("DX_WHATSAPP_FALLBACK_IPC")
            .map(|v| v.eq_ignore_ascii_case("framed") || v == "1")
            .unwrap_or(true);

        cmd.arg(runner)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        if let Ok(auth_dir) = std::env::var("DX_WHATSAPP_AUTH_DIR") {
            cmd.env("AUTH_DIR", auth_dir);
        }
        if framed_ipc {
            cmd.env("DX_BRIDGE_FRAMED_IPC", "1");
        }

        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to acquire Baileys stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to acquire Baileys stdout"))?;

        let responses = Arc::new(DashMap::<u64, serde_json::Value>::new());
        let incoming = Arc::clone(&self.incoming);
        let connected = Arc::clone(&self.connected);
        let responses_for_task = Arc::clone(&responses);

        let reader_task = tokio::spawn(async move {
            if framed_ipc {
                let mut reader = BufReader::new(stdout);
                let mut stream_buffer = Vec::<u8>::new();
                let mut chunk = [0_u8; 4096];

                loop {
                    let Ok(read) = reader.read(&mut chunk).await else {
                        break;
                    };
                    if read == 0 {
                        break;
                    }

                    stream_buffer.extend_from_slice(&chunk[..read]);

                    loop {
                        let decoded = Frame::decode(&stream_buffer);
                        let (frame, consumed) = match decoded {
                            Ok(v) => v,
                            Err(FrameError::Incomplete(_)) => break,
                            Err(_) => {
                                stream_buffer.clear();
                                break;
                            }
                        };

                        stream_buffer.drain(0..consumed);
                        if frame.frame_type != FrameType::Binary {
                            continue;
                        }

                        let Ok(payload) =
                            serde_json::from_slice::<serde_json::Value>(&frame.payload)
                        else {
                            continue;
                        };

                        Self::handle_baileys_payload(
                            payload,
                            &responses_for_task,
                            &incoming,
                            &connected,
                        )
                        .await;
                    }
                }

                return;
            }

            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                let Ok(payload) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };

                Self::handle_baileys_payload(payload, &responses_for_task, &incoming, &connected)
                    .await;
            }
        });

        {
            let mut guard = self.baileys.lock().await;
            *guard = Some(BaileysBridge {
                child,
                stdin,
                responses,
                reader_task,
                next_id: 1,
                framed_ipc,
            });
        }

        let init = self.send_baileys_request("init", serde_json::json!({})).await?;
        let success = init.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !success {
            let error = init.get("error").and_then(|v| v.as_str()).unwrap_or("Baileys init failed");
            anyhow::bail!(error.to_string());
        }

        self.connected.store(true, Ordering::Relaxed);
        info!("WhatsApp Baileys bridge connected");
        Ok(())
    }

    async fn send_baileys_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (request_id, responses, framed_ipc) = {
            let mut guard = self.baileys.lock().await;
            let bridge =
                guard.as_mut().ok_or_else(|| anyhow::anyhow!("Baileys bridge not connected"))?;
            let request_id = bridge.next_id;
            bridge.next_id = bridge.next_id.saturating_add(1);

            let request = serde_json::json!({
                "id": request_id,
                "method": method,
                "params": params,
            });

            if bridge.framed_ipc {
                let payload = serde_json::to_vec(&request)?;
                let frame = Frame::binary(payload).encode();
                bridge.stdin.write_all(&frame).await?;
            } else {
                let line = serde_json::to_string(&request)?;
                bridge.stdin.write_all(line.as_bytes()).await?;
                bridge.stdin.write_all(b"\n").await?;
            }
            bridge.stdin.flush().await?;

            (request_id, Arc::clone(&bridge.responses), bridge.framed_ipc)
        };

        let timeout = tokio::time::Duration::from_secs(30);
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            if let Some((_, response)) = responses.remove(&request_id) {
                if let Some(error) = response.get("error") {
                    let message = error
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Baileys request failed");
                    anyhow::bail!(message.to_string());
                }

                return Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null));
            }

            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!("Baileys request timed out: {method} (framed_ipc={})", framed_ipc);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }
    }

    async fn disconnect_baileys(&self) -> Result<()> {
        let _ = self.send_baileys_request("shutdown", serde_json::json!({})).await;

        let mut guard = self.baileys.lock().await;
        if let Some(mut bridge) = guard.take() {
            let _ = bridge.child.kill().await;
            bridge.reader_task.abort();
        }
        Ok(())
    }

    fn normalize_e164(number: &str) -> Option<String> {
        let trimmed = number.trim();
        if trimmed.is_empty() {
            return None;
        }

        let without_prefix = trimmed
            .strip_prefix("whatsapp:")
            .or_else(|| trimmed.strip_prefix("WhatsApp:"))
            .unwrap_or(trimmed)
            .trim();

        let mut digits = String::with_capacity(without_prefix.len() + 1);
        let mut saw_plus = false;
        for ch in without_prefix.chars() {
            if ch == '+' && !saw_plus && digits.is_empty() {
                saw_plus = true;
                continue;
            }
            if ch.is_ascii_digit() {
                digits.push(ch);
            }
        }

        if digits.is_empty() {
            return None;
        }

        Some(format!("+{}", digits))
    }

    fn strip_target_prefixes(value: &str) -> String {
        let mut candidate = value.trim().to_string();
        loop {
            let before = candidate.clone();
            candidate = candidate
                .trim_start_matches("whatsapp:")
                .trim_start_matches("WhatsApp:")
                .trim()
                .to_string();
            if candidate == before {
                return candidate;
            }
        }
    }

    fn is_whatsapp_group_jid(value: &str) -> bool {
        let candidate = Self::strip_target_prefixes(value);
        let lower = candidate.to_ascii_lowercase();
        if !lower.ends_with("@g.us") {
            return false;
        }

        let local_part = &candidate[..candidate.len() - "@g.us".len()];
        if local_part.is_empty() || local_part.contains('@') {
            return false;
        }

        local_part
            .split('-')
            .all(|segment| !segment.is_empty() && segment.chars().all(|ch| ch.is_ascii_digit()))
    }

    fn extract_user_jid_phone(candidate: &str) -> Option<String> {
        let lower = candidate.to_ascii_lowercase();
        if let Some(local) = lower.strip_suffix("@s.whatsapp.net") {
            let base = local.split(':').next()?;
            if !base.is_empty() && base.chars().all(|ch| ch.is_ascii_digit()) {
                return Some(base.to_string());
            }
            return None;
        }

        if let Some(local) = lower.strip_suffix("@lid") {
            if !local.is_empty() && local.chars().all(|ch| ch.is_ascii_digit()) {
                return Some(local.to_string());
            }
            return None;
        }

        None
    }

    fn normalize_whatsapp_target(value: &str) -> Option<String> {
        let candidate = Self::strip_target_prefixes(value);
        if candidate.is_empty() {
            return None;
        }

        if Self::is_whatsapp_group_jid(&candidate) {
            let local = &candidate[..candidate.len() - "@g.us".len()];
            return Some(format!("{}@g.us", local));
        }

        if let Some(phone) = Self::extract_user_jid_phone(&candidate) {
            return Self::normalize_e164(&phone);
        }

        if candidate.contains('@') {
            return None;
        }

        Self::normalize_e164(&candidate)
    }

    async fn handle_baileys_payload(
        payload: serde_json::Value,
        responses: &DashMap<u64, serde_json::Value>,
        incoming: &Arc<Mutex<Vec<IncomingMessage>>>,
        connected: &Arc<AtomicBool>,
    ) {
        if let Some(id) = payload.get("id").and_then(|v| v.as_u64()) {
            responses.insert(id, payload);
            return;
        }

        let Some(event) = payload.get("event").and_then(|v| v.as_str()) else {
            return;
        };

        match event {
            "state" => {
                let state = payload
                    .get("payload")
                    .and_then(|p| p.get("state"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                connected.store(state == "connected", Ordering::Relaxed);
            }
            "message" => {
                let Some(message_payload) = payload.get("payload") else {
                    return;
                };

                let from = message_payload
                    .get("from")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let text = message_payload
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let message_id = message_payload
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                if from.is_empty() {
                    return;
                }

                let incoming_message = IncomingMessage {
                    id: message_id,
                    channel_type: "whatsapp".into(),
                    chat_id: from.clone(),
                    sender_id: from,
                    sender_name: None,
                    channel_name: "whatsapp".into(),
                    content: MessageContent::Text { text },
                    timestamp: chrono::Utc::now(),
                    reply_to: None,
                    metadata: std::collections::HashMap::new(),
                    is_group: false,
                    group_name: None,
                };

                let mut queue = incoming.lock().await;
                queue.push(incoming_message);
            }
            _ => {}
        }
    }

    fn normalize_send_targets(to: &str) -> Result<(String, String)> {
        let normalized = Self::normalize_whatsapp_target(to)
            .ok_or_else(|| anyhow::anyhow!("Invalid WhatsApp target: {}", to))?;

        if Self::is_whatsapp_group_jid(&normalized) {
            return Ok((normalized.clone(), normalized));
        }

        let cloud = normalized.clone();
        let baileys = normalized.trim_start_matches('+').to_string();
        Ok((cloud, baileys))
    }

    fn parse_allow_from(metadata: &std::collections::HashMap<String, String>) -> Vec<String> {
        metadata
            .get("allow_from")
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn metadata_target_mode(metadata: &std::collections::HashMap<String, String>) -> String {
        metadata
            .get("target_mode")
            .map(|m| m.to_ascii_lowercase())
            .unwrap_or_else(|| "explicit".to_string())
    }

    fn resolve_target_with_policy(
        to: &str,
        metadata: &std::collections::HashMap<String, String>,
    ) -> Result<String> {
        let mode = metadata
            .get("target_mode")
            .map(|m| m.to_ascii_lowercase())
            .unwrap_or_else(|| "explicit".to_string());

        let normalized_to = Self::normalize_whatsapp_target(to)
            .ok_or_else(|| anyhow::anyhow!("Invalid WhatsApp target: {}", to))?;

        if Self::is_whatsapp_group_jid(&normalized_to) {
            return Ok(normalized_to);
        }

        if mode == "implicit" || mode == "heartbeat" {
            let allow_raw = Self::parse_allow_from(metadata);
            let has_wildcard = allow_raw.iter().any(|entry| entry == "*");
            let allow_list = allow_raw
                .into_iter()
                .filter(|entry| entry != "*")
                .filter_map(|entry| Self::normalize_whatsapp_target(&entry))
                .collect::<Vec<_>>();

            if !has_wildcard && !allow_list.is_empty() && !allow_list.contains(&normalized_to) {
                anyhow::bail!(
                    "WhatsApp target {} is not allowed by allow_from policy",
                    normalized_to
                );
            }
        }

        Ok(normalized_to)
    }

    /// Send a text message via Cloud API
    async fn send_cloud_api_text(&self, to: &str, text: &str) -> Result<String> {
        let phone_id = self
            .config
            .phone_number_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Phone number ID not configured"))?;
        let token = self
            .config
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Access token not configured"))?;

        let url = format!("https://graph.facebook.com/v21.0/{}/messages", phone_id);

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": { "body": text }
        });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?
            .json::<CloudApiResponse>()
            .await?;

        if let Some(error) = resp.error {
            anyhow::bail!("WhatsApp API error ({}): {}", error.code, error.message);
        }

        let msg_id = resp
            .messages
            .and_then(|m| m.first().map(|msg| msg.id.clone()))
            .unwrap_or_default();

        Ok(msg_id)
    }

    /// Send media via Cloud API
    async fn send_cloud_api_media(
        &self,
        to: &str,
        media_type: &str,
        url: &str,
        caption: Option<&str>,
    ) -> Result<String> {
        let phone_id = self
            .config
            .phone_number_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Phone number ID not configured"))?;
        let token = self
            .config
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Access token not configured"))?;

        let api_url = format!("https://graph.facebook.com/v21.0/{}/messages", phone_id);

        let mut media_obj = serde_json::json!({ "link": url });
        if let Some(cap) = caption {
            media_obj["caption"] = serde_json::Value::String(cap.to_string());
        }

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": media_type,
            media_type: media_obj
        });

        let resp = self
            .http
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?
            .json::<CloudApiResponse>()
            .await?;

        if let Some(error) = resp.error {
            anyhow::bail!("WhatsApp API error: {}", error.message);
        }

        Ok(resp
            .messages
            .and_then(|m| m.first().map(|msg| msg.id.clone()))
            .unwrap_or_default())
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn display_name(&self) -> &str {
        "WhatsApp"
    }

    fn is_enabled(&self) -> bool {
        self.config.access_token.is_some() || self.config.use_baileys
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            text: true,
            markdown: false,
            images: true,
            audio: true,
            video: true,
            files: true,
            reactions: true,
            structured: true, // Templates, buttons
            edit: false,
            delete: true,
            typing: true,
            read_receipts: true,
            groups: true,
            voice: true,
            webhooks: true,
        }
    }

    fn registration(&self) -> ChannelRegistration {
        ChannelRegistration {
            name: "whatsapp".into(),
            display_name: "WhatsApp".into(),
            description: "WhatsApp Cloud API integration".into(),
            version: "0.1.0".into(),
            author: "DX Team".into(),
            icon: Some("whatsapp".into()),
            capabilities: self.capabilities(),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        if self.config.use_baileys {
            self.connect_baileys().await?;
            return Ok(());
        }

        // Verify token by calling the API
        if let Some(ref token) = self.config.access_token {
            if let Some(ref phone_id) = self.config.phone_number_id {
                let url = format!("https://graph.facebook.com/v21.0/{}", phone_id);
                let resp = self
                    .http
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await?;

                if resp.status().is_success() {
                    self.connected.store(true, Ordering::Relaxed);
                    info!("WhatsApp Cloud API connected");
                } else {
                    anyhow::bail!("WhatsApp API auth failed: {}", resp.status());
                }
            }
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.config.use_baileys {
            self.disconnect_baileys().await?;
        }
        self.connected.store(false, Ordering::Relaxed);
        info!("WhatsApp channel disconnected");
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<DeliveryStatus> {
        let allow_from = Self::parse_allow_from(&message.metadata);
        let target_mode = Self::metadata_target_mode(&message.metadata);
        let resolved_to = Self::resolve_target_with_policy(&message.to, &message.metadata)?;
        let (cloud_to, baileys_to) = Self::normalize_send_targets(&resolved_to)?;

        if self.config.use_baileys {
            let result = match &message.content {
                MessageContent::Text { text } => {
                    self.send_baileys_request(
                        "send",
                        serde_json::json!({
                            "to": baileys_to,
                            "message": text,
                            "mode": target_mode.clone(),
                            "allowFrom": allow_from.clone(),
                        }),
                    )
                    .await?
                }
                MessageContent::Media(media) => {
                    let media_url = media.url.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("Media URL is required for WhatsApp media send")
                    })?;
                    let media_type = match media.media_type {
                        MediaType::Image => "image",
                        MediaType::Audio | MediaType::Voice => "audio",
                        MediaType::Video => "video",
                        _ => "document",
                    };

                    self.send_baileys_request(
                        "sendMedia",
                        serde_json::json!({
                            "to": baileys_to,
                            "url": media_url,
                            "caption": media.caption,
                            "type": media_type,
                            "mode": target_mode.clone(),
                            "allowFrom": allow_from.clone(),
                        }),
                    )
                    .await?
                }
                MessageContent::Reaction { emoji } => {
                    let chat_jid = message
                        .metadata
                        .get("chat_jid")
                        .cloned()
                        .unwrap_or_else(|| baileys_to.clone());
                    let message_id = message
                        .metadata
                        .get("message_id")
                        .cloned()
                        .or_else(|| message.reply_to.clone())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "WhatsApp reaction requires message_id metadata or reply_to"
                            )
                        })?;

                    self.send_baileys_request(
                        "sendReaction",
                        serde_json::json!({
                            "chatJid": chat_jid,
                            "messageId": message_id,
                            "emoji": emoji,
                        }),
                    )
                    .await?
                }
                MessageContent::Structured { data } => {
                    let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("Poll");
                    let options = data
                        .get("options")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect::<Vec<_>>();

                    if options.is_empty() {
                        return Ok(DeliveryStatus::Failed(
                            "Structured WhatsApp poll requires non-empty options[]".into(),
                        ));
                    }

                    let selectable_count =
                        data.get("selectableCount").and_then(|v| v.as_u64()).unwrap_or(1);

                    self.send_baileys_request(
                        "sendPoll",
                        serde_json::json!({
                            "to": baileys_to,
                            "name": name,
                            "options": options,
                            "selectableCount": selectable_count,
                        }),
                    )
                    .await?
                }
                _ => {
                    return Ok(DeliveryStatus::Failed(
                        "Unsupported content for WhatsApp Baileys".into(),
                    ));
                }
            };

            let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            if success {
                return Ok(DeliveryStatus::Sent);
            }

            let error =
                result.get("error").and_then(|v| v.as_str()).unwrap_or("Baileys send failed");
            return Ok(DeliveryStatus::Failed(error.to_string()));
        }

        match &message.content {
            MessageContent::Text { text } => {
                self.send_cloud_api_text(&cloud_to, text).await?;
            }
            MessageContent::Media(media) => {
                if let Some(ref url) = media.url {
                    let media_type = match media.media_type {
                        MediaType::Image => "image",
                        MediaType::Audio | MediaType::Voice => "audio",
                        MediaType::Video => "video",
                        _ => "document",
                    };
                    self.send_cloud_api_media(&cloud_to, media_type, url, media.caption.as_deref())
                        .await?;
                }
            }
            _ => {
                warn!("Unsupported content type for WhatsApp");
                return Ok(DeliveryStatus::Failed("Unsupported content".into()));
            }
        }

        Ok(DeliveryStatus::Sent)
    }

    async fn receive(&self) -> Result<Vec<IncomingMessage>> {
        let mut queue = self.incoming.lock().await;
        Ok(queue.drain(..).collect())
    }

    async fn handle_webhook(&self, payload: serde_json::Value) -> Result<()> {
        // Parse WhatsApp Cloud API webhook
        if let Some(entry) = payload.get("entry").and_then(|e| e.as_array()) {
            for e in entry {
                if let Some(changes) = e.get("changes").and_then(|c| c.as_array()) {
                    for change in changes {
                        if let Some(value) = change.get("value") {
                            if let Some(messages) = value.get("messages").and_then(|m| m.as_array())
                            {
                                for msg in messages {
                                    let from = msg
                                        .get("from")
                                        .and_then(|f| f.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let msg_id = msg
                                        .get("id")
                                        .and_then(|i| i.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let text = msg
                                        .get("text")
                                        .and_then(|t| t.get("body"))
                                        .and_then(|b| b.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    let incoming = IncomingMessage {
                                        id: msg_id,
                                        channel_type: "whatsapp".into(),
                                        chat_id: from.clone(),
                                        sender_id: from,
                                        sender_name: None,
                                        channel_name: "whatsapp".into(),
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
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whatsapp_config() {
        let config = WhatsAppConfig {
            access_token: Some("token".into()),
            phone_number_id: Some("123".into()),
            verify_token: None,
            business_account_id: None,
            use_baileys: false,
        };
        let channel = WhatsAppChannel::new(config);
        assert_eq!(channel.name(), "whatsapp");
        assert!(channel.is_enabled());
    }

    #[test]
    fn test_runner_path_resolution() {
        let path = WhatsAppChannel::resolve_baileys_runner_path();
        let rendered = path.to_string_lossy();
        assert!(
            rendered.contains("whatsapp/runner.mjs")
                || rendered.contains("whatsapp\\runner.mjs")
                || rendered.contains("whatsapp-runner.mjs")
        );
    }

    #[test]
    fn test_normalize_whatsapp_target_user_jid() {
        assert_eq!(
            WhatsAppChannel::normalize_whatsapp_target("41796666864:0@s.whatsapp.net"),
            Some("+41796666864".into())
        );
        assert_eq!(
            WhatsAppChannel::normalize_whatsapp_target("123456789@lid"),
            Some("+123456789".into())
        );
    }

    #[test]
    fn test_normalize_whatsapp_target_group_jid() {
        assert_eq!(
            WhatsAppChannel::normalize_whatsapp_target("whatsapp:123456789-987654321@g.us"),
            Some("123456789-987654321@g.us".into())
        );
        assert_eq!(WhatsAppChannel::normalize_whatsapp_target("group:123@g.us"), None);
    }

    #[test]
    fn test_resolve_target_with_policy_implicit_allowlist() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("target_mode".into(), "implicit".into());
        metadata.insert("allow_from".into(), "+12345,+67890".into());

        let allowed = WhatsAppChannel::resolve_target_with_policy("+12345", &metadata)
            .expect("target should be allowed");
        assert_eq!(allowed, "+12345");

        let blocked = WhatsAppChannel::resolve_target_with_policy("+99999", &metadata);
        assert!(blocked.is_err());
    }
}
