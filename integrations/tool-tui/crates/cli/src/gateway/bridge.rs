//! Node.js Channel Bridge
//!
//! Subprocess-based bridge to Node.js for messaging channel integrations.
//! This enables using mature Node.js SDKs like:
//! - @whiskeysockets/baileys (WhatsApp)
//! - grammy (Telegram)
//! - discord.js (Discord)
//! - @slack/web-api (Slack)
//!
//! Communication happens via JSON-RPC over stdin/stdout.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{RwLock, mpsc, oneshot};

/// Supported channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    WhatsApp,
    Telegram,
    Discord,
    Slack,
    Signal,
    IMessage,
}

impl ChannelType {
    /// Get the Node.js runner script name
    pub fn runner_script(&self) -> &'static str {
        match self {
            ChannelType::WhatsApp => "whatsapp-runner.mjs",
            ChannelType::Telegram => "telegram-runner.mjs",
            ChannelType::Discord => "discord-runner.mjs",
            ChannelType::Slack => "slack-runner.mjs",
            ChannelType::Signal => "signal-runner.mjs",
            ChannelType::IMessage => "imessage-runner.mjs",
        }
    }

    /// Get required npm packages
    pub fn npm_packages(&self) -> Vec<&'static str> {
        match self {
            ChannelType::WhatsApp => vec!["@whiskeysockets/baileys", "pino", "qrcode-terminal"],
            ChannelType::Telegram => vec!["grammy"],
            ChannelType::Discord => vec!["discord.js"],
            ChannelType::Slack => vec!["@slack/web-api", "@slack/bolt"],
            ChannelType::Signal => vec![], // Uses signal-cli subprocess
            ChannelType::IMessage => vec![], // Uses AppleScript on macOS
        }
    }
}

/// JSON-RPC request to the bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRequest {
    /// Request ID for correlation
    pub id: String,
    /// Method name
    pub method: String,
    /// Parameters
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC response from the bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    /// Request ID (matches request)
    pub id: String,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BridgeError>,
}

/// Bridge error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Event from the bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEvent {
    /// Event type
    pub event: String,
    /// Event payload
    pub payload: Value,
}

/// Bridge connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Channel bridge instance
pub struct ChannelBridge {
    /// Channel type
    channel_type: ChannelType,
    /// Child process
    process: Option<Child>,
    /// Bridge state
    state: Arc<RwLock<BridgeState>>,
    /// Pending requests
    pending: Arc<RwLock<HashMap<String, oneshot::Sender<BridgeResponse>>>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<BridgeEvent>,
    /// Bridge directory
    bridge_dir: PathBuf,
}

impl ChannelBridge {
    /// Create a new channel bridge
    pub fn new(channel_type: ChannelType, event_tx: mpsc::UnboundedSender<BridgeEvent>) -> Self {
        let bridge_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("bridge");

        Self {
            channel_type,
            process: None,
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            pending: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            bridge_dir,
        }
    }

    /// Start the bridge process
    pub async fn start(&mut self) -> Result<()> {
        // Ensure bridge directory exists
        std::fs::create_dir_all(&self.bridge_dir)?;

        // Check if Node.js is available
        let node_path =
            which::which("node").context("Node.js not found. Please install Node.js >= 20")?;

        // Get runner script path
        let script_path = self.bridge_dir.join(self.channel_type.runner_script());

        // Create runner script if it doesn't exist
        if !script_path.exists() {
            self.create_runner_script(&script_path)?;
        }

        // Install npm packages if needed
        self.ensure_packages().await?;

        *self.state.write().await = BridgeState::Connecting;

        // Start the process
        let mut child = Command::new(&node_path)
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.bridge_dir)
            .spawn()
            .context("Failed to start bridge process")?;

        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let stderr = child.stderr.take().context("Failed to get stderr")?;

        self.process = Some(child);

        // Spawn stdout reader
        let state = Arc::clone(&self.state);
        let pending = Arc::clone(&self.pending);
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) if line.trim().is_empty() => continue,
                    Ok(line) => {
                        // Try to parse as response
                        if let Ok(response) = serde_json::from_str::<BridgeResponse>(&line) {
                            let mut pending = pending.write().await;
                            if let Some(tx) = pending.remove(&response.id) {
                                let _ = tx.send(response);
                            }
                            continue;
                        }

                        // Try to parse as event
                        if let Ok(event) = serde_json::from_str::<BridgeEvent>(&line) {
                            // Handle state change events
                            if event.event == "state" {
                                if let Some(new_state) =
                                    event.payload.get("state").and_then(|s| s.as_str())
                                {
                                    let mut state = state.write().await;
                                    *state = match new_state {
                                        "connected" => BridgeState::Connected,
                                        "disconnected" => BridgeState::Disconnected,
                                        "error" => BridgeState::Error,
                                        _ => BridgeState::Connecting,
                                    };
                                }
                            }

                            let _ = event_tx.send(event);
                        }
                    }
                    Err(_) => break,
                }
            }

            *state.write().await = BridgeState::Disconnected;
        });

        // Spawn stderr reader for logging
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if !line.trim().is_empty() {
                        tracing::warn!("Bridge stderr: {}", line);
                    }
                }
            }
        });

        // Wait for connection
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Send init request
        let init_response = self.call("init", json!({})).await?;

        if init_response.error.is_some() {
            *self.state.write().await = BridgeState::Error;
            anyhow::bail!("Bridge init failed: {:?}", init_response.error);
        }

        *self.state.write().await = BridgeState::Connected;
        tracing::info!("Bridge started for {:?}", self.channel_type);

        Ok(())
    }

    /// Stop the bridge process
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            // Send shutdown request
            let _ = self.call("shutdown", json!({})).await;

            // Wait a bit for graceful shutdown
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Force kill if still running
            let _ = process.kill();
            let _ = process.wait();
        }

        *self.state.write().await = BridgeState::Disconnected;
        Ok(())
    }

    /// Call a method on the bridge
    pub async fn call(&mut self, method: &str, params: Value) -> Result<BridgeResponse> {
        let process = self.process.as_mut().context("Bridge not running")?;
        let stdin = process.stdin.as_mut().context("No stdin")?;

        let id = uuid::Uuid::new_v4().to_string();
        let request = BridgeRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
        };

        // Create response channel
        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id.clone(), tx);

        // Send request
        let json = serde_json::to_string(&request)?;
        writeln!(stdin, "{}", json)?;
        stdin.flush()?;

        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                self.pending.write().await.remove(&id);
                anyhow::bail!("Response channel closed")
            }
            Err(_) => {
                self.pending.write().await.remove(&id);
                anyhow::bail!("Request timed out")
            }
        }
    }

    /// Get current state
    pub async fn state(&self) -> BridgeState {
        *self.state.read().await
    }

    /// Ensure npm packages are installed
    async fn ensure_packages(&self) -> Result<()> {
        let packages = self.channel_type.npm_packages();
        if packages.is_empty() {
            return Ok(());
        }

        let package_json = self.bridge_dir.join("package.json");

        // Create package.json if it doesn't exist
        if !package_json.exists() {
            let content = json!({
                "name": "dx-bridge",
                "version": "1.0.0",
                "type": "module",
                "private": true,
                "dependencies": {}
            });
            std::fs::write(&package_json, serde_json::to_string_pretty(&content)?)?;
        }

        // Check if packages are installed
        let node_modules = self.bridge_dir.join("node_modules");
        let mut needs_install = !node_modules.exists();

        if !needs_install {
            for pkg in &packages {
                let pkg_name = pkg.split('@').next().unwrap_or(pkg).trim_start_matches('@');
                let pkg_path = if pkg.starts_with('@') {
                    node_modules.join(pkg.split('/').next().unwrap_or(pkg))
                } else {
                    node_modules.join(pkg_name)
                };
                if !pkg_path.exists() {
                    needs_install = true;
                    break;
                }
            }
        }

        if needs_install {
            tracing::info!("Installing npm packages: {:?}", packages);

            let npm_path = which::which("npm").context("npm not found")?;

            let status = Command::new(&npm_path)
                .args(["install", "--save"])
                .args(&packages)
                .current_dir(&self.bridge_dir)
                .status()
                .context("Failed to run npm install")?;

            if !status.success() {
                anyhow::bail!("npm install failed");
            }
        }

        Ok(())
    }

    /// Create the runner script
    fn create_runner_script(&self, path: &PathBuf) -> Result<()> {
        let content = match self.channel_type {
            ChannelType::WhatsApp => include_str!("../bridge/whatsapp-runner.mjs"),
            ChannelType::Telegram => include_str!("../bridge/telegram-runner.mjs"),
            ChannelType::Discord => include_str!("../bridge/discord-runner.mjs"),
            ChannelType::Slack => include_str!("../bridge/slack-runner.mjs"),
            ChannelType::Signal => include_str!("../bridge/signal-runner.mjs"),
            ChannelType::IMessage => include_str!("../bridge/imessage-runner.mjs"),
        };

        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Drop for ChannelBridge {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

/// Bridge manager for multiple channels
pub struct BridgeManager {
    /// Active bridges
    bridges: Arc<RwLock<HashMap<ChannelType, ChannelBridge>>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<BridgeEvent>,
    /// Event receiver (for consumers)
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<BridgeEvent>>>>,
}

impl BridgeManager {
    /// Create a new bridge manager
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            bridges: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        }
    }

    /// Take the event receiver (can only be called once)
    pub async fn take_event_receiver(&self) -> Option<mpsc::UnboundedReceiver<BridgeEvent>> {
        self.event_rx.write().await.take()
    }

    /// Start a channel bridge
    pub async fn start_channel(&self, channel_type: ChannelType) -> Result<()> {
        let mut bridges = self.bridges.write().await;

        if bridges.contains_key(&channel_type) {
            anyhow::bail!("Channel {:?} already started", channel_type);
        }

        let mut bridge = ChannelBridge::new(channel_type, self.event_tx.clone());
        bridge.start().await?;

        bridges.insert(channel_type, bridge);
        Ok(())
    }

    /// Stop a channel bridge
    pub async fn stop_channel(&self, channel_type: ChannelType) -> Result<()> {
        let mut bridges = self.bridges.write().await;

        if let Some(mut bridge) = bridges.remove(&channel_type) {
            bridge.stop().await?;
        }

        Ok(())
    }

    /// Get channel state
    pub async fn channel_state(&self, channel_type: ChannelType) -> BridgeState {
        let bridges = self.bridges.read().await;

        match bridges.get(&channel_type) {
            Some(bridge) => bridge.state().await,
            None => BridgeState::Disconnected,
        }
    }

    /// Call a method on a channel
    pub async fn call(
        &self,
        channel_type: ChannelType,
        method: &str,
        params: Value,
    ) -> Result<BridgeResponse> {
        let mut bridges = self.bridges.write().await;

        let bridge = bridges
            .get_mut(&channel_type)
            .context(format!("Channel {:?} not started", channel_type))?;

        bridge.call(method, params).await
    }

    /// Send a message via a channel
    pub async fn send_message(
        &self,
        channel_type: ChannelType,
        to: &str,
        message: &str,
    ) -> Result<BridgeResponse> {
        self.call(
            channel_type,
            "send",
            json!({
                "to": to,
                "message": message
            }),
        )
        .await
    }

    /// List active channels
    pub async fn list_channels(&self) -> Vec<(ChannelType, BridgeState)> {
        let bridges = self.bridges.read().await;
        let mut result = Vec::new();

        for (channel_type, bridge) in bridges.iter() {
            result.push((*channel_type, bridge.state().await));
        }

        result
    }

    /// Stop all channels
    pub async fn stop_all(&self) -> Result<()> {
        let mut bridges = self.bridges.write().await;

        for (_, mut bridge) in bridges.drain() {
            let _ = bridge.stop().await;
        }

        Ok(())
    }
}

impl Default for BridgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_runner_script() {
        assert_eq!(ChannelType::WhatsApp.runner_script(), "whatsapp-runner.mjs");
        assert_eq!(ChannelType::Telegram.runner_script(), "telegram-runner.mjs");
    }

    #[test]
    fn test_channel_type_npm_packages() {
        let packages = ChannelType::WhatsApp.npm_packages();
        assert!(packages.contains(&"@whiskeysockets/baileys"));

        let packages = ChannelType::Telegram.npm_packages();
        assert!(packages.contains(&"grammy"));
    }

    #[test]
    fn test_bridge_request_serialization() {
        let request = BridgeRequest {
            id: "test-1".to_string(),
            method: "send".to_string(),
            params: json!({"to": "123", "message": "hello"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-1"));
        assert!(json.contains("send"));
    }

    #[tokio::test]
    async fn test_bridge_manager_creation() {
        let manager = BridgeManager::new();
        let channels = manager.list_channels().await;
        assert!(channels.is_empty());
    }
}
