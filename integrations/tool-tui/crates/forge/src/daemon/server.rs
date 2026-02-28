//! Forge Daemon IPC Server
//!
//! Handles CLI and extension connections via Unix sockets (or TCP on Windows).
//! Also provides WebSocket server for VS Code extension integration.

use crate::daemon::{DaemonEvent, DaemonState};
use crate::dx_cache::DxToolId;
use crate::tools::{ToolRegistry, ToolStatus};
use anyhow::Result;
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite;

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

use tokio::net::{TcpListener, TcpStream};

// ============================================================================
// IPC PROTOCOL
// ============================================================================

/// Commands from CLI/Extension to Daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum IpcCommand {
    GetStatus,
    ListTools,
    RunTool { name: String, args: Vec<String> },
    EnableTool { name: String },
    DisableTool { name: String },
    GetBranchStatus,
    ApproveChange { id: String },
    ApproveAllPending,
    RejectChange { id: String },
    RejectAllPending,
    GetBranchHistory { limit: usize },
    GetFileChanges { limit: Option<usize> },
    GetGitStatus { workspace_path: Option<String> },
    SyncWithGit { workspace_path: Option<String> },
    ClearFileChanges,
    Shutdown { force: bool },
    Ping,
    // Extension-specific
    FileChanged { path: String, change_type: String },
}

/// Responses from Daemon to CLI/Extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcResponse {
    Status(StatusResponse),
    ToolList { tools: Vec<ToolInfoResponse> },
    ToolResult(ToolResultResponse),
    BranchStatus(BranchStatusResponse),
    BranchHistory { entries: Vec<BranchHistoryEntry> },
    FileChanges { changes: Vec<TrackedFileChange> },
    FileChangeEvent(TrackedFileChange),
    GitStatus(GitStatusResponse),
    Count { count: usize },
    Success,
    Error { message: String },
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusResponse {
    pub is_clean: bool,
    pub branch: String,
    pub staged: Vec<GitFileStatus>,
    pub unstaged: Vec<GitFileStatus>,
    pub untracked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
    pub diff: Option<FileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub state: String,
    pub uptime_seconds: u64,
    pub files_changed: u64,
    pub tools_executed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub errors: u64,
    pub lsp_events: u64,
    pub fs_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfoResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub is_dummy: bool,
    pub last_run: Option<String>,
    pub run_count: u64,
    pub error_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultResponse {
    pub success: bool,
    pub warm_start: bool,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchStatusResponse {
    pub current_color: String,
    pub pending_changes: Vec<PendingChangeResponse>,
    pub auto_approved: u64,
    pub manual_approved: u64,
    pub rejected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChangeResponse {
    pub id: String,
    pub path: String,
    pub change_type: String,
    pub color: String,
    pub timestamp: String,
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchHistoryEntry {
    pub timestamp: String,
    pub path: String,
    pub color: String,
    pub action: String,
}

// ============================================================================
// DAEMON SERVER
// ============================================================================

/// The IPC server for the Forge daemon
pub struct DaemonServer {
    tool_registry: Arc<ToolRegistry>,
    shutdown: Arc<AtomicBool>,
    state: Arc<RwLock<DaemonState>>,
    stats: Arc<RwLock<ServerStats>>,
    event_tx: broadcast::Sender<DaemonEvent>,
    branch_state: Arc<RwLock<BranchState>>,
    file_tracker: Arc<RwLock<FileChangeTracker>>,
}

#[derive(Debug, Default)]
struct ServerStats {
    uptime_start: Option<std::time::Instant>,
    files_changed: u64,
    tools_executed: u64,
    cache_hits: u64,
    cache_misses: u64,
    errors: u64,
    lsp_events: u64,
    fs_events: u64,
}

/// Tracked file change with diff information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFileChange {
    pub path: String,
    pub change_type: String,
    pub timestamp: String,
    pub diff: Option<FileDiff>,
}

/// File diff information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub additions: u32,
    pub deletions: u32,
    pub hunks: Vec<DiffHunk>,
}

/// A diff hunk showing changed lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub content: String,
}

#[derive(Debug, Default)]
struct FileChangeTracker {
    /// Recent file changes with diffs
    changes: Vec<TrackedFileChange>,
    /// Previous file contents for diff computation
    file_cache: std::collections::HashMap<String, String>,
    /// Maximum number of changes to track
    max_changes: usize,
}

impl FileChangeTracker {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
            file_cache: std::collections::HashMap::new(),
            max_changes: 100,
        }
    }

    /// Track a file change and compute diff
    fn track_change(&mut self, path: &str, change_type: &str) -> TrackedFileChange {
        let diff = self.compute_diff(path, change_type);

        let change = TrackedFileChange {
            path: path.to_string(),
            change_type: change_type.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            diff,
        };

        // Add to front of list
        self.changes.insert(0, change.clone());

        // Trim to max size
        if self.changes.len() > self.max_changes {
            self.changes.truncate(self.max_changes);
        }

        // Update cache with new content
        if change_type != "deleted" {
            if let Ok(content) = std::fs::read_to_string(path) {
                self.file_cache.insert(path.to_string(), content);
            }
        } else {
            self.file_cache.remove(path);
        }

        change
    }

    /// Compute diff between cached and current file content
    fn compute_diff(&self, path: &str, change_type: &str) -> Option<FileDiff> {
        use similar::{ChangeTag, TextDiff};

        if change_type == "deleted" {
            // For deleted files, show all lines as deletions
            if let Some(old_content) = self.file_cache.get(path) {
                let lines = old_content.lines().count() as u32;
                return Some(FileDiff {
                    additions: 0,
                    deletions: lines,
                    hunks: vec![DiffHunk {
                        old_start: 1,
                        old_lines: lines,
                        new_start: 0,
                        new_lines: 0,
                        content: format!(
                            "-{}",
                            old_content.lines().take(10).collect::<Vec<_>>().join("\n-")
                        ),
                    }],
                });
            }
            return None;
        }

        // Read current file content
        let new_content = std::fs::read_to_string(path).ok()?;

        if change_type == "created" {
            // For new files, show all lines as additions
            let lines = new_content.lines().count() as u32;
            return Some(FileDiff {
                additions: lines,
                deletions: 0,
                hunks: vec![DiffHunk {
                    old_start: 0,
                    old_lines: 0,
                    new_start: 1,
                    new_lines: lines,
                    content: format!(
                        "+{}",
                        new_content.lines().take(10).collect::<Vec<_>>().join("\n+")
                    ),
                }],
            });
        }

        // For modified files, compute actual diff
        let old_content = self.file_cache.get(path).map(|s| s.as_str()).unwrap_or("");
        let diff = TextDiff::from_lines(old_content, &new_content);

        let mut additions = 0u32;
        let mut deletions = 0u32;
        let mut hunks = Vec::new();
        let mut current_hunk_content = String::new();
        let mut hunk_old_start = 0u32;
        let mut hunk_new_start = 0u32;
        let mut hunk_old_lines = 0u32;
        let mut hunk_new_lines = 0u32;
        let mut in_hunk = false;

        for (idx, change) in diff.iter_all_changes().enumerate() {
            match change.tag() {
                ChangeTag::Delete => {
                    deletions += 1;
                    if !in_hunk {
                        in_hunk = true;
                        hunk_old_start = idx as u32 + 1;
                        hunk_new_start = idx as u32 + 1;
                    }
                    hunk_old_lines += 1;
                    current_hunk_content.push_str(&format!("-{}", change));
                }
                ChangeTag::Insert => {
                    additions += 1;
                    if !in_hunk {
                        in_hunk = true;
                        hunk_old_start = idx as u32 + 1;
                        hunk_new_start = idx as u32 + 1;
                    }
                    hunk_new_lines += 1;
                    current_hunk_content.push_str(&format!("+{}", change));
                }
                ChangeTag::Equal => {
                    if in_hunk {
                        // End current hunk
                        hunks.push(DiffHunk {
                            old_start: hunk_old_start,
                            old_lines: hunk_old_lines,
                            new_start: hunk_new_start,
                            new_lines: hunk_new_lines,
                            content: current_hunk_content.clone(),
                        });
                        current_hunk_content.clear();
                        hunk_old_lines = 0;
                        hunk_new_lines = 0;
                        in_hunk = false;
                    }
                }
            }
        }

        // Don't forget the last hunk
        if in_hunk && !current_hunk_content.is_empty() {
            hunks.push(DiffHunk {
                old_start: hunk_old_start,
                old_lines: hunk_old_lines,
                new_start: hunk_new_start,
                new_lines: hunk_new_lines,
                content: current_hunk_content,
            });
        }

        // Limit hunks to first 5 for performance
        hunks.truncate(5);

        Some(FileDiff {
            additions,
            deletions,
            hunks,
        })
    }

    /// Get recent changes
    fn get_changes(&self) -> &[TrackedFileChange] {
        &self.changes
    }

    /// Clear all tracked changes
    fn clear(&mut self) {
        self.changes.clear();
    }
}

#[derive(Debug, Default)]
struct BranchState {
    current_color: BranchColor,
    pending_changes: Vec<PendingChange>,
    auto_approved: u64,
    manual_approved: u64,
    rejected: u64,
    history: Vec<BranchHistoryEntry>,
}

// Reserved for future branching color implementation - Yellow and Red will be used
// when the branching decision engine is fully implemented to indicate warning and
// danger states for file changes. Timeline: v0.3.0
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
enum BranchColor {
    #[default]
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone)]
struct PendingChange {
    id: String,
    path: String,
    change_type: String,
    color: BranchColor,
    timestamp: chrono::DateTime<chrono::Utc>,
    tool: Option<String>,
}

impl Default for DaemonServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonServer {
    /// Create a new daemon server
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            tool_registry: Arc::new(ToolRegistry::with_dummy_tools()),
            shutdown: Arc::new(AtomicBool::new(false)),
            state: Arc::new(RwLock::new(DaemonState::Stopped)),
            stats: Arc::new(RwLock::new(ServerStats::default())),
            event_tx,
            branch_state: Arc::new(RwLock::new(BranchState::default())),
            file_tracker: Arc::new(RwLock::new(FileChangeTracker::new())),
        }
    }

    /// Get the socket path
    #[cfg(unix)]
    pub fn socket_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        runtime_dir.join("dx-forge.sock")
    }

    /// Get the IPC port (Windows)
    #[cfg(windows)]
    pub fn ipc_port() -> u16 {
        std::env::var("DX_FORGE_IPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9877)
    }

    /// Get the WebSocket port for VS Code extension
    pub fn ws_port() -> u16 {
        std::env::var("DX_FORGE_WS_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9876)
    }

    /// Get the PID file path
    pub fn pid_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        runtime_dir.join("dx-forge.pid")
    }

    /// Write PID file
    fn write_pid_file() -> Result<()> {
        let pid = std::process::id();
        std::fs::write(Self::pid_path(), pid.to_string())?;
        Ok(())
    }

    /// Remove PID file
    fn remove_pid_file() {
        let _ = std::fs::remove_file(Self::pid_path());
    }

    /// Start the IPC server
    pub async fn start(&self) -> Result<()> {
        *self.state.write() = DaemonState::Starting;
        self.stats.write().uptime_start = Some(std::time::Instant::now());

        // Write PID file
        Self::write_pid_file()?;

        let ws_port = Self::ws_port();

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║        ⚔️  FORGE DAEMON SERVER - Binary Dawn Edition          ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║  Tools Registered: {}                                         ║",
            self.tool_registry.count()
        );
        println!("║  WebSocket Port: {}                                        ║", ws_port);
        println!("╚══════════════════════════════════════════════════════════════╝");

        // Start WebSocket server in background
        let ws_handler = self.clone_for_handler();
        let ws_shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = ws_handler.start_websocket_server(ws_port, ws_shutdown).await {
                eprintln!("WebSocket server error: {}", e);
            }
        });

        #[cfg(unix)]
        {
            self.start_unix_server().await
        }

        #[cfg(windows)]
        {
            self.start_tcp_server().await
        }
    }

    #[cfg(unix)]
    async fn start_unix_server(&self) -> Result<()> {
        let socket_path = Self::socket_path();

        // Remove existing socket
        let _ = std::fs::remove_file(&socket_path);

        let listener = UnixListener::bind(&socket_path)?;
        println!("🔌 IPC Server listening on: {}", socket_path.display());

        *self.state.write() = DaemonState::Running;

        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let server = self.clone_for_handler();
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_unix_connection(stream).await {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Accept error: {}", e);
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Check shutdown flag periodically
                }
            }
        }

        // Cleanup
        let _ = std::fs::remove_file(&socket_path);
        Self::remove_pid_file();
        *self.state.write() = DaemonState::Stopped;

        Ok(())
    }

    #[cfg(windows)]
    async fn start_tcp_server(&self) -> Result<()> {
        let port = Self::ipc_port();
        let addr = format!("127.0.0.1:{}", port);

        let listener = TcpListener::bind(&addr).await?;
        println!("🔌 IPC Server listening on: {}", addr);

        *self.state.write() = DaemonState::Running;

        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let server = self.clone_for_handler();
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_tcp_connection(stream).await {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Accept error: {}", e);
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Check shutdown flag periodically
                }
            }
        }

        Self::remove_pid_file();
        *self.state.write() = DaemonState::Stopped;

        Ok(())
    }

    fn clone_for_handler(&self) -> DaemonServerHandler {
        DaemonServerHandler {
            tool_registry: self.tool_registry.clone(),
            shutdown: self.shutdown.clone(),
            state: self.state.clone(),
            stats: self.stats.clone(),
            event_tx: self.event_tx.clone(),
            branch_state: self.branch_state.clone(),
            file_tracker: self.file_tracker.clone(),
        }
    }

    /// Stop the server
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        *self.state.write() = DaemonState::ShuttingDown;
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DaemonEvent> {
        self.event_tx.subscribe()
    }
}

// ============================================================================
// CONNECTION HANDLER
// ============================================================================

struct DaemonServerHandler {
    tool_registry: Arc<ToolRegistry>,
    shutdown: Arc<AtomicBool>,
    state: Arc<RwLock<DaemonState>>,
    stats: Arc<RwLock<ServerStats>>,
    event_tx: broadcast::Sender<DaemonEvent>,
    branch_state: Arc<RwLock<BranchState>>,
    file_tracker: Arc<RwLock<FileChangeTracker>>,
}

impl DaemonServerHandler {
    /// Start WebSocket server for VS Code extension
    async fn start_websocket_server(&self, port: u16, shutdown: Arc<AtomicBool>) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        println!("🌐 WebSocket Server listening on: ws://{}", addr);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }

            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            println!("📡 WebSocket connection from: {}", addr);
                            let handler = DaemonServerHandler {
                                tool_registry: self.tool_registry.clone(),
                                shutdown: self.shutdown.clone(),
                                state: self.state.clone(),
                                stats: self.stats.clone(),
                                event_tx: self.event_tx.clone(),
                                branch_state: self.branch_state.clone(),
                                file_tracker: self.file_tracker.clone(),
                            };
                            tokio::spawn(async move {
                                if let Err(e) = handler.handle_websocket_connection(stream).await {
                                    eprintln!("WebSocket connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("WebSocket accept error: {}", e);
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Check shutdown flag periodically
                }
            }
        }

        Ok(())
    }

    /// Handle a WebSocket connection
    async fn handle_websocket_connection(&self, stream: TcpStream) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    let response = self.handle_command(&text);
                    let response_json = serde_json::to_string(&response)?;
                    write.send(tungstenite::Message::Text(response_json.into())).await?;
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    write.send(tungstenite::Message::Pong(data)).await?;
                }
                Ok(tungstenite::Message::Close(_)) => {
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket message error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[cfg(unix)]
    async fn handle_unix_connection(&self, stream: UnixStream) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // Connection closed
            }

            let response = self.handle_command(&line);
            let response_json = serde_json::to_string(&response)? + "\n";
            writer.write_all(response_json.as_bytes()).await?;
            writer.flush().await?;

            // Check for shutdown command
            if matches!(response, IpcResponse::Success) && line.contains("Shutdown") {
                break;
            }
        }

        Ok(())
    }

    #[cfg(windows)]
    async fn handle_tcp_connection(&self, stream: TcpStream) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break;
            }

            let response = self.handle_command(&line);
            let response_json = serde_json::to_string(&response)? + "\n";
            writer.write_all(response_json.as_bytes()).await?;
            writer.flush().await?;

            if matches!(response, IpcResponse::Success) && line.contains("Shutdown") {
                break;
            }
        }

        Ok(())
    }

    fn handle_command(&self, line: &str) -> IpcResponse {
        let cmd: Result<IpcCommand, _> = serde_json::from_str(line.trim());

        match cmd {
            Ok(command) => self.execute_command(command),
            Err(e) => IpcResponse::Error {
                message: format!("Invalid command: {}", e),
            },
        }
    }

    fn execute_command(&self, cmd: IpcCommand) -> IpcResponse {
        match cmd {
            IpcCommand::Ping => IpcResponse::Pong,

            IpcCommand::GetStatus => {
                let stats = self.stats.read();
                let uptime = stats.uptime_start.map(|s| s.elapsed().as_secs()).unwrap_or(0);

                IpcResponse::Status(StatusResponse {
                    state: format!("{:?}", *self.state.read()),
                    uptime_seconds: uptime,
                    files_changed: stats.files_changed,
                    tools_executed: stats.tools_executed,
                    cache_hits: stats.cache_hits,
                    cache_misses: stats.cache_misses,
                    errors: stats.errors,
                    lsp_events: stats.lsp_events,
                    fs_events: stats.fs_events,
                })
            }

            IpcCommand::ListTools => {
                let tools = self.tool_registry.list();
                let tool_responses: Vec<ToolInfoResponse> = tools
                    .into_iter()
                    .map(|t| ToolInfoResponse {
                        id: t.id,
                        name: t.name,
                        version: t.version,
                        status: format!("{:?}", t.status),
                        is_dummy: t.is_dummy,
                        last_run: t.last_run.map(|dt| dt.to_rfc3339()),
                        run_count: t.run_count,
                        error_count: t.error_count,
                    })
                    .collect();

                IpcResponse::ToolList {
                    tools: tool_responses,
                }
            }

            IpcCommand::RunTool { name, args: _ } => {
                // Find tool by name
                let tool_id = match name.to_lowercase().as_str() {
                    "bundler" | "dx-bundler" => Some(DxToolId::Bundler),
                    "style" | "dx-style" => Some(DxToolId::Style),
                    "test-runner" | "dx-test-runner" | "test" => Some(DxToolId::Test),
                    "package-manager" | "dx-package-manager" | "node-modules" => {
                        Some(DxToolId::NodeModules)
                    }
                    "serializer" | "dx-serializer" => Some(DxToolId::Serializer),
                    "www" | "dx-www" => Some(DxToolId::Www),
                    _ => None,
                };

                match tool_id {
                    Some(id) => {
                        self.tool_registry.set_status(id, ToolStatus::Running);

                        // Simulate execution (dummy tools)
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        self.tool_registry.record_execution(id, true);
                        self.tool_registry.set_status(id, ToolStatus::Ready);
                        self.stats.write().tools_executed += 1;

                        IpcResponse::ToolResult(ToolResultResponse {
                            success: true,
                            warm_start: true,
                            cache_hits: 1,
                            cache_misses: 0,
                            output: Some(format!("Tool {} executed successfully", name)),
                            error: None,
                        })
                    }
                    None => IpcResponse::Error {
                        message: format!("Unknown tool: {}", name),
                    },
                }
            }

            IpcCommand::EnableTool { name } => {
                let tool_id = self.parse_tool_name(&name);
                match tool_id {
                    Some(id) => {
                        self.tool_registry.enable(id);
                        IpcResponse::Success
                    }
                    None => IpcResponse::Error {
                        message: format!("Unknown tool: {}", name),
                    },
                }
            }

            IpcCommand::DisableTool { name } => {
                let tool_id = self.parse_tool_name(&name);
                match tool_id {
                    Some(id) => {
                        self.tool_registry.disable(id);
                        IpcResponse::Success
                    }
                    None => IpcResponse::Error {
                        message: format!("Unknown tool: {}", name),
                    },
                }
            }

            IpcCommand::GetBranchStatus => {
                let state = self.branch_state.read();
                IpcResponse::BranchStatus(BranchStatusResponse {
                    current_color: format!("{:?}", state.current_color),
                    pending_changes: state
                        .pending_changes
                        .iter()
                        .map(|c| PendingChangeResponse {
                            id: c.id.clone(),
                            path: c.path.clone(),
                            change_type: c.change_type.clone(),
                            color: format!("{:?}", c.color),
                            timestamp: c.timestamp.to_rfc3339(),
                            tool: c.tool.clone(),
                        })
                        .collect(),
                    auto_approved: state.auto_approved,
                    manual_approved: state.manual_approved,
                    rejected: state.rejected,
                })
            }

            IpcCommand::ApproveChange { id } => {
                let mut state = self.branch_state.write();
                if let Some(pos) = state.pending_changes.iter().position(|c| c.id == id) {
                    let change = state.pending_changes.remove(pos);
                    state.manual_approved += 1;
                    state.history.push(BranchHistoryEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        path: change.path,
                        color: format!("{:?}", change.color),
                        action: "approved".to_string(),
                    });
                    IpcResponse::Success
                } else {
                    IpcResponse::Error {
                        message: format!("Change not found: {}", id),
                    }
                }
            }

            IpcCommand::ApproveAllPending => {
                let mut state = self.branch_state.write();
                let count = state.pending_changes.len();
                let changes: Vec<_> = state.pending_changes.drain(..).collect();
                for change in changes {
                    state.history.push(BranchHistoryEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        path: change.path,
                        color: format!("{:?}", change.color),
                        action: "approved".to_string(),
                    });
                }
                state.manual_approved += count as u64;
                IpcResponse::Count { count }
            }

            IpcCommand::RejectChange { id } => {
                let mut state = self.branch_state.write();
                if let Some(pos) = state.pending_changes.iter().position(|c| c.id == id) {
                    let change = state.pending_changes.remove(pos);
                    state.rejected += 1;
                    state.history.push(BranchHistoryEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        path: change.path,
                        color: format!("{:?}", change.color),
                        action: "rejected".to_string(),
                    });
                    IpcResponse::Success
                } else {
                    IpcResponse::Error {
                        message: format!("Change not found: {}", id),
                    }
                }
            }

            IpcCommand::RejectAllPending => {
                let mut state = self.branch_state.write();
                let count = state.pending_changes.len();
                let changes: Vec<_> = state.pending_changes.drain(..).collect();
                for change in changes {
                    state.history.push(BranchHistoryEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        path: change.path,
                        color: format!("{:?}", change.color),
                        action: "rejected".to_string(),
                    });
                }
                state.rejected += count as u64;
                IpcResponse::Count { count }
            }

            IpcCommand::GetBranchHistory { limit } => {
                let state = self.branch_state.read();
                let entries: Vec<_> = state.history.iter().rev().take(limit).cloned().collect();
                IpcResponse::BranchHistory { entries }
            }

            IpcCommand::GetFileChanges { limit } => {
                let tracker = self.file_tracker.read();
                let changes = tracker.get_changes();
                let limit = limit.unwrap_or(50);
                let changes: Vec<_> = changes.iter().take(limit).cloned().collect();
                IpcResponse::FileChanges { changes }
            }

            IpcCommand::GetGitStatus { workspace_path } => {
                self.get_git_status(workspace_path.as_deref())
            }

            IpcCommand::SyncWithGit { workspace_path } => {
                // Reset file change counter and sync with Git
                self.stats.write().files_changed = 0;
                self.file_tracker.write().clear();

                // Get current Git status and populate tracker
                match self.get_git_status(workspace_path.as_deref()) {
                    IpcResponse::GitStatus(status) => {
                        let total_changes =
                            status.staged.len() + status.unstaged.len() + status.untracked.len();
                        self.stats.write().files_changed = total_changes as u64;
                        IpcResponse::GitStatus(status)
                    }
                    other => other,
                }
            }

            IpcCommand::ClearFileChanges => {
                self.file_tracker.write().clear();
                IpcResponse::Success
            }

            IpcCommand::Shutdown { force: _ } => {
                self.shutdown.store(true, Ordering::SeqCst);
                *self.state.write() = DaemonState::ShuttingDown;
                IpcResponse::Success
            }

            IpcCommand::FileChanged { path, change_type } => {
                self.stats.write().files_changed += 1;

                // Track the change with diff
                let tracked_change = self.file_tracker.write().track_change(&path, &change_type);

                println!(
                    "📝 File changed: {} ({}) [+{} -{} lines]",
                    path,
                    change_type,
                    tracked_change.diff.as_ref().map(|d| d.additions).unwrap_or(0),
                    tracked_change.diff.as_ref().map(|d| d.deletions).unwrap_or(0)
                );

                // Return the tracked change with diff info
                IpcResponse::FileChangeEvent(tracked_change)
            }
        }
    }

    fn parse_tool_name(&self, name: &str) -> Option<DxToolId> {
        match name.to_lowercase().as_str() {
            "bundler" | "dx-bundler" => Some(DxToolId::Bundler),
            "style" | "dx-style" => Some(DxToolId::Style),
            "test-runner" | "dx-test-runner" | "test" => Some(DxToolId::Test),
            "package-manager" | "dx-package-manager" | "node-modules" => {
                Some(DxToolId::NodeModules)
            }
            "serializer" | "dx-serializer" => Some(DxToolId::Serializer),
            "www" | "dx-www" => Some(DxToolId::Www),
            _ => None,
        }
    }

    /// Get Git status for the specified workspace directory
    fn get_git_status(&self, workspace_path: Option<&str>) -> IpcResponse {
        use std::process::Command;

        // Determine working directory
        let cwd = workspace_path
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Get current branch
        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&cwd)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // Get porcelain status
        let status_output = match Command::new("git")
            .args(["status", "--porcelain=v1"])
            .current_dir(&cwd)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return IpcResponse::Error {
                        message: format!("Git status failed: {}", stderr),
                    };
                }
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            Err(e) => {
                return IpcResponse::Error {
                    message: format!("Failed to run git status: {}", e),
                };
            }
        };

        let mut staged: Vec<GitFileStatus> = Vec::new();
        let mut unstaged: Vec<GitFileStatus> = Vec::new();
        let mut untracked: Vec<String> = Vec::new();

        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }

            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].to_string();

            // Untracked files
            if index_status == '?' && worktree_status == '?' {
                untracked.push(path);
                continue;
            }

            // Staged changes (index)
            if index_status != ' ' && index_status != '?' {
                let status = match index_status {
                    'M' => "modified",
                    'A' => "added",
                    'D' => "deleted",
                    'R' => "renamed",
                    'C' => "copied",
                    _ => "changed",
                };

                // Get diff for staged file
                let diff = self.get_git_diff(&path, true, &cwd);

                staged.push(GitFileStatus {
                    path: path.clone(),
                    status: status.to_string(),
                    diff,
                });
            }

            // Unstaged changes (worktree)
            if worktree_status != ' ' && worktree_status != '?' {
                let status = match worktree_status {
                    'M' => "modified",
                    'D' => "deleted",
                    _ => "changed",
                };

                // Get diff for unstaged file
                let diff = self.get_git_diff(&path, false, &cwd);

                unstaged.push(GitFileStatus {
                    path,
                    status: status.to_string(),
                    diff,
                });
            }
        }

        let is_clean = staged.is_empty() && unstaged.is_empty() && untracked.is_empty();

        IpcResponse::GitStatus(GitStatusResponse {
            is_clean,
            branch,
            staged,
            unstaged,
            untracked,
        })
    }

    /// Get diff for a specific file
    fn get_git_diff(&self, path: &str, staged: bool, cwd: &std::path::Path) -> Option<FileDiff> {
        use std::process::Command;

        let args = if staged {
            vec!["diff", "--cached", "--numstat", path]
        } else {
            vec!["diff", "--numstat", path]
        };

        let numstat = Command::new("git").args(&args).current_dir(cwd).output().ok()?;

        let numstat_str = String::from_utf8_lossy(&numstat.stdout);
        let parts: Vec<&str> = numstat_str.trim().split('\t').collect();

        if parts.len() < 2 {
            return None;
        }

        let additions: u32 = parts[0].parse().unwrap_or(0);
        let deletions: u32 = parts[1].parse().unwrap_or(0);

        // Get actual diff content (limited)
        let diff_args = if staged {
            vec!["diff", "--cached", "-U3", path]
        } else {
            vec!["diff", "-U3", path]
        };

        let diff_output = Command::new("git").args(&diff_args).current_dir(cwd).output().ok()?;

        let diff_content = String::from_utf8_lossy(&diff_output.stdout);

        // Parse hunks from diff output
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut hunk_content = String::new();

        for line in diff_content.lines() {
            if line.starts_with("@@") {
                // Save previous hunk
                if let Some(mut hunk) = current_hunk.take() {
                    hunk.content = hunk_content.clone();
                    hunks.push(hunk);
                    hunk_content.clear();
                }

                // Parse hunk header: @@ -start,lines +start,lines @@
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let old_range = parts[1].trim_start_matches('-');
                    let new_range = parts[2].trim_start_matches('+');

                    let (old_start, old_lines) = parse_range(old_range);
                    let (new_start, new_lines) = parse_range(new_range);

                    current_hunk = Some(DiffHunk {
                        old_start,
                        old_lines,
                        new_start,
                        new_lines,
                        content: String::new(),
                    });
                }
            } else if current_hunk.is_some()
                && (line.starts_with('+') || line.starts_with('-') || line.starts_with(' '))
            {
                hunk_content.push_str(line);
                hunk_content.push('\n');
            }
        }

        // Don't forget the last hunk
        if let Some(mut hunk) = current_hunk {
            hunk.content = hunk_content;
            hunks.push(hunk);
        }

        // Limit hunks
        hunks.truncate(5);

        Some(FileDiff {
            additions,
            deletions,
            hunks,
        })
    }
}

/// Parse a diff range like "10,5" or "10" into (start, lines)
fn parse_range(range: &str) -> (u32, u32) {
    let parts: Vec<&str> = range.split(',').collect();
    let start: u32 = parts[0].parse().unwrap_or(0);
    let lines: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    (start, lines)
}
