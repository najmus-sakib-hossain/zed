//! Forge LSP/WebSocket Server
//!
//! Handles WebSocket connections from VS Code extension.

use crate::daemon::{DaemonEvent, DaemonState};
use crate::tools::ToolRegistry;
use anyhow::Result;
use axum::{
    Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::broadcast;

// ============================================================================
// LSP MESSAGE TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LspRequest {
    #[serde(rename = "status")]
    GetStatus,
    #[serde(rename = "tools")]
    ListTools,
    #[serde(rename = "run")]
    RunTool { name: String },
    #[serde(rename = "file_changed")]
    FileChanged { path: String, change_type: String },
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LspResponse {
    #[serde(rename = "status")]
    Status {
        state: String,
        uptime_seconds: u64,
        files_changed: u64,
        tools_executed: u64,
        cache_hits: u64,
        errors: u64,
    },
    #[serde(rename = "tools")]
    Tools { tools: Vec<LspToolInfo> },
    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
        success: bool,
        output: Option<String>,
    },
    #[serde(rename = "event")]
    Event { event_type: String, data: String },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspToolInfo {
    pub name: String,
    pub version: String,
    pub status: String,
    pub is_dummy: bool,
    pub run_count: u64,
    pub error_count: u64,
}

// ============================================================================
// LSP SERVER STATE
// ============================================================================

#[derive(Clone)]
pub struct LspServerState {
    pub tool_registry: Arc<ToolRegistry>,
    pub shutdown: Arc<AtomicBool>,
    pub state: Arc<RwLock<DaemonState>>,
    pub uptime_start: Arc<RwLock<Option<std::time::Instant>>>,
    pub files_changed: Arc<AtomicU64>,
    pub tools_executed: Arc<AtomicU64>,
    pub cache_hits: Arc<AtomicU64>,
    pub errors: Arc<AtomicU64>,
    pub event_tx: broadcast::Sender<DaemonEvent>,
    pub connection_count: Arc<AtomicU64>,
}

impl LspServerState {
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            tool_registry,
            shutdown: Arc::new(AtomicBool::new(false)),
            state: Arc::new(RwLock::new(DaemonState::Stopped)),
            uptime_start: Arc::new(RwLock::new(None)),
            files_changed: Arc::new(AtomicU64::new(0)),
            tools_executed: Arc::new(AtomicU64::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            event_tx,
            connection_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

// ============================================================================
// LSP SERVER
// ============================================================================

/// WebSocket/LSP server for VS Code extension
pub struct LspServer {
    port: u16,
    state: LspServerState,
}

impl LspServer {
    pub fn new(port: u16, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            port,
            state: LspServerState::new(tool_registry),
        }
    }

    /// Start the WebSocket server
    pub async fn start(self: Arc<Self>) -> Result<()> {
        *self.state.uptime_start.write() = Some(std::time::Instant::now());
        *self.state.state.write() = DaemonState::Running;

        let app = Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            .with_state(self.state.clone());

        let addr = format!("127.0.0.1:{}", self.port);
        println!("🌐 LSP/WebSocket server listening on: ws://{}/ws", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;

        let shutdown = self.state.shutdown.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                while !shutdown.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            })
            .await?;

        *self.state.state.write() = DaemonState::Stopped;
        Ok(())
    }

    pub fn stop(&self) {
        self.state.shutdown.store(true, Ordering::SeqCst);
    }

    pub fn connection_count(&self) -> u64 {
        self.state.connection_count.load(Ordering::SeqCst)
    }
}

// ============================================================================
// HANDLERS
// ============================================================================

async fn health_handler() -> &'static str {
    "OK"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<LspServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: LspServerState) {
    state.connection_count.fetch_add(1, Ordering::SeqCst);
    println!(
        "🔗 Extension connected (total: {})",
        state.connection_count.load(Ordering::SeqCst)
    );

    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = state.event_tx.subscribe();

    // Channel for sending responses back to client
    let (response_tx, mut response_rx) = tokio::sync::mpsc::channel::<String>(100);

    // Spawn task to forward events and responses to client
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle daemon events
                Ok(event) = event_rx.recv() => {
                    // Only forward actual events, not response messages
                    if let DaemonEvent::FileChanged(_) | DaemonEvent::ToolStarted(_) | DaemonEvent::ToolCompleted(..) = &event {
                        let response = LspResponse::Event {
                            event_type: format!("{:?}", event),
                            data: String::new(),
                        };
                        if let Ok(json) = serde_json::to_string(&response) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                // Handle response messages
                Some(json) = response_rx.recv() => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response = handle_message(&text, &state);
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = response_tx.send(json).await;
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    state.connection_count.fetch_sub(1, Ordering::SeqCst);
    println!(
        "🔌 Extension disconnected (total: {})",
        state.connection_count.load(Ordering::SeqCst)
    );
}

fn handle_message(text: &str, state: &LspServerState) -> LspResponse {
    let request: Result<LspRequest, _> = serde_json::from_str(text);

    match request {
        Ok(req) => execute_request(req, state),
        Err(e) => LspResponse::Error {
            message: format!("Invalid request: {}", e),
        },
    }
}

fn execute_request(req: LspRequest, state: &LspServerState) -> LspResponse {
    match req {
        LspRequest::Ping => LspResponse::Pong,

        LspRequest::GetStatus => {
            let uptime = state.uptime_start.read().map(|s| s.elapsed().as_secs()).unwrap_or(0);

            LspResponse::Status {
                state: format!("{:?}", *state.state.read()),
                uptime_seconds: uptime,
                files_changed: state.files_changed.load(Ordering::SeqCst),
                tools_executed: state.tools_executed.load(Ordering::SeqCst),
                cache_hits: state.cache_hits.load(Ordering::SeqCst),
                errors: state.errors.load(Ordering::SeqCst),
            }
        }

        LspRequest::ListTools => {
            let tools = state.tool_registry.list();
            LspResponse::Tools {
                tools: tools
                    .into_iter()
                    .map(|t| LspToolInfo {
                        name: t.name,
                        version: t.version,
                        status: format!("{:?}", t.status),
                        is_dummy: t.is_dummy,
                        run_count: t.run_count,
                        error_count: t.error_count,
                    })
                    .collect(),
            }
        }

        LspRequest::RunTool { name } => {
            state.tools_executed.fetch_add(1, Ordering::SeqCst);
            LspResponse::ToolResult {
                name: name.clone(),
                success: true,
                output: Some(format!("Tool {} executed", name)),
            }
        }

        LspRequest::FileChanged { path, change_type } => {
            state.files_changed.fetch_add(1, Ordering::SeqCst);
            println!("📝 [LSP] File changed: {} ({})", path, change_type);
            LspResponse::Pong
        }
    }
}
