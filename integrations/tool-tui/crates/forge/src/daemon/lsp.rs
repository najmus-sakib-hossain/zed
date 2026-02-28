//! LSP Bridge - VS Code Extension Integration
//!
//! Provides Language Server Protocol integration for:
//! - Real-time file change notifications from editors
//! - Semantic code analysis
//! - Pattern detection before files hit disk
//! - VS Code extension communication

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, broadcast, mpsc};

// ============================================================================
// LSP MESSAGE TYPES
// ============================================================================

/// LSP JSON-RPC message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspMessage {
    Request(LspRequest),
    Response(LspResponse),
    Notification(LspNotification),
}

/// LSP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// LSP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    pub jsonrpc: String,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LspError>,
}

/// LSP error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// LSP notification (no response required)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl LspNotification {
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }
}

// ============================================================================
// DX-SPECIFIC NOTIFICATIONS
// ============================================================================

/// Text document change notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentChange {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    #[serde(rename = "contentChanges")]
    pub content_changes: Vec<TextDocumentContentChange>,
}

/// Text document identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
    #[serde(default)]
    pub version: Option<i64>,
}

/// Content change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentContentChange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Range in a text document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Position in a text document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

// ============================================================================
// DX FORGE NOTIFICATIONS (Custom)
// ============================================================================

/// DX tool status notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxToolStatus {
    pub tool: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub output: Option<String>,
}

/// DX pattern detected notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxPatternDetected {
    pub uri: String,
    pub patterns: Vec<DxPattern>,
}

/// Detected DX pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxPattern {
    pub kind: String,
    pub name: String,
    pub range: Range,
}

/// DX cache status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxCacheStatus {
    pub tool: String,
    pub entries: usize,
    pub size_bytes: u64,
    pub warm: bool,
}

// ============================================================================
// LSP BRIDGE
// ============================================================================

/// LSP Bridge configuration
#[derive(Debug, Clone)]
pub struct LspBridgeConfig {
    /// Port for TCP connections (VS Code extension)
    pub port: u16,
    /// Enable Unix socket (for local editors)
    pub enable_unix_socket: bool,
    /// Unix socket path
    pub socket_path: Option<PathBuf>,
}

impl Default for LspBridgeConfig {
    fn default() -> Self {
        Self {
            port: 9527, // DX_FORGE_PORT
            enable_unix_socket: cfg!(unix),
            socket_path: Some(PathBuf::from("/tmp/dx-forge.sock")),
        }
    }
}

/// Type alias for incoming message receiver
type IncomingMessageReceiver = Arc<RwLock<Option<mpsc::Receiver<(u64, LspMessage)>>>>;

/// LSP Bridge for VS Code integration
pub struct LspBridge {
    config: LspBridgeConfig,
    /// Connected clients
    clients: Arc<RwLock<HashMap<u64, ClientConnection>>>,
    /// Next client ID
    next_client_id: Arc<RwLock<u64>>,
    /// Notification broadcaster
    notification_tx: broadcast::Sender<LspNotification>,
    /// Incoming messages channel
    incoming_tx: mpsc::Sender<(u64, LspMessage)>,
    incoming_rx: IncomingMessageReceiver,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

/// Client connection
struct ClientConnection {
    tx: mpsc::Sender<String>,
}

impl LspBridge {
    /// Create a new LSP bridge
    pub fn new(config: LspBridgeConfig) -> Self {
        let (notification_tx, _) = broadcast::channel(1000);
        let (incoming_tx, incoming_rx) = mpsc::channel(1000);

        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: Arc::new(RwLock::new(0)),
            notification_tx,
            incoming_tx,
            incoming_rx: Arc::new(RwLock::new(Some(incoming_rx))),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Subscribe to outgoing notifications
    pub fn subscribe(&self) -> broadcast::Receiver<LspNotification> {
        self.notification_tx.subscribe()
    }

    /// Get incoming message receiver (one-time)
    pub async fn take_incoming_receiver(&self) -> Option<mpsc::Receiver<(u64, LspMessage)>> {
        self.incoming_rx.write().await.take()
    }

    /// Start the LSP bridge server
    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;

        println!("🔌 LSP Bridge starting on port {}...", self.config.port);

        // Start TCP listener
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port)).await?;
        println!("📡 LSP Bridge listening on 127.0.0.1:{}", self.config.port);

        let clients = self.clients.clone();
        let next_id = self.next_client_id.clone();
        let incoming_tx = self.incoming_tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let client_id = {
                            let mut id = next_id.write().await;
                            *id += 1;
                            *id
                        };

                        println!("🔗 Client {} connected from {}", client_id, addr);

                        let (tx, rx) = mpsc::channel(100);
                        let client = ClientConnection { tx };

                        clients.write().await.insert(client_id, client);

                        // Handle client
                        let clients_clone = clients.clone();
                        let incoming_tx_clone = incoming_tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                Self::handle_client(client_id, stream, rx, incoming_tx_clone).await
                            {
                                eprintln!("Client {} error: {}", client_id, e);
                            }

                            clients_clone.write().await.remove(&client_id);
                            println!("🔌 Client {} disconnected", client_id);
                        });
                    }
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle a client connection
    async fn handle_client(
        client_id: u64,
        stream: TcpStream,
        mut outgoing_rx: mpsc::Receiver<String>,
        incoming_tx: mpsc::Sender<(u64, LspMessage)>,
    ) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            tokio::select! {
                // Read from client
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            // Parse LSP message
                            if let Ok(msg) = serde_json::from_str::<LspMessage>(&line) {
                                let _ = incoming_tx.send((client_id, msg)).await;
                            }
                            line.clear();
                        }
                        Err(e) => {
                            eprintln!("Read error: {}", e);
                            break;
                        }
                    }
                }

                // Write to client
                Some(msg) = outgoing_rx.recv() => {
                    if let Err(e) = writer.write_all(msg.as_bytes()).await {
                        eprintln!("Write error: {}", e);
                        break;
                    }
                    if let Err(e) = writer.write_all(b"\n").await {
                        eprintln!("Write error: {}", e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Stop the LSP bridge
    pub async fn stop(&self) {
        *self.running.write().await = false;
        self.clients.write().await.clear();
        println!("🔌 LSP Bridge stopped");
    }

    /// Broadcast notification to all clients
    pub async fn broadcast(&self, notification: LspNotification) {
        let msg = serde_json::to_string(&notification).unwrap_or_default();

        for client in self.clients.read().await.values() {
            let _ = client.tx.send(msg.clone()).await;
        }

        let _ = self.notification_tx.send(notification);
    }

    /// Send notification to specific client
    pub async fn send_to(&self, client_id: u64, notification: LspNotification) {
        let msg = serde_json::to_string(&notification).unwrap_or_default();

        if let Some(client) = self.clients.read().await.get(&client_id) {
            let _ = client.tx.send(msg).await;
        }
    }

    /// Notify tool started
    pub async fn notify_tool_started(&self, tool: &str) {
        self.broadcast(LspNotification::new("dx/toolStarted", serde_json::json!({ "tool": tool })))
            .await;
    }

    /// Notify tool completed
    pub async fn notify_tool_completed(&self, tool: &str, duration_ms: u64, success: bool) {
        self.broadcast(LspNotification::new(
            "dx/toolCompleted",
            serde_json::json!({
                "tool": tool,
                "duration_ms": duration_ms,
                "success": success
            }),
        ))
        .await;
    }

    /// Notify pattern detected
    pub async fn notify_pattern(&self, uri: &str, patterns: Vec<DxPattern>) {
        self.broadcast(LspNotification::new(
            "dx/patternDetected",
            serde_json::json!({
                "uri": uri,
                "patterns": patterns
            }),
        ))
        .await;
    }

    /// Notify cache status
    pub async fn notify_cache_status(&self, status: DxCacheStatus) {
        self.broadcast(LspNotification::new(
            "dx/cacheStatus",
            serde_json::to_value(status).unwrap_or_default(),
        ))
        .await;
    }

    /// Get connected client count
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

impl Default for LspBridge {
    fn default() -> Self {
        Self::new(LspBridgeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_notification_new() {
        let notification = LspNotification::new("test/method", serde_json::json!({"key": "value"}));
        assert_eq!(notification.method, "test/method");
        assert_eq!(notification.jsonrpc, "2.0");
    }

    #[test]
    fn test_lsp_bridge_config_default() {
        let config = LspBridgeConfig::default();
        assert_eq!(config.port, 9527);
    }

    #[tokio::test]
    async fn test_lsp_bridge_new() {
        let bridge = LspBridge::new(LspBridgeConfig::default());
        assert_eq!(bridge.client_count().await, 0);
    }
}
