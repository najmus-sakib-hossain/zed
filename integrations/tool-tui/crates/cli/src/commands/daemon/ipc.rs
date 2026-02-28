//! Inter-Process Communication for Daemon Architecture
//!
//! All IPC uses .sr (serializer) format for efficiency:
//! - 52-73% token savings over JSON
//! - Zero-copy deserialization
//! - RKYV format for machine-to-machine

use anyhow::Result;
use std::path::PathBuf;

/// IPC message types
#[derive(Debug, Clone)]
pub enum IpcMessage {
    // Agent -> Project
    SyncRequest(SyncRequest),
    UpdateNotification(UpdateNotification),
    BranchConfig(BranchConfig),

    // Project -> Agent
    CheckResults(CheckResultsMsg),
    FileSync(FileSyncMsg),
    StatusReport(StatusReport),

    // Bidirectional
    Heartbeat(Heartbeat),
    Error(ErrorMsg),
}

/// Request to sync files to R2
#[derive(Debug, Clone)]
pub struct SyncRequest {
    pub files: Vec<PathBuf>,
    pub priority: u8,
    pub compress: bool,
}

/// AI update notification
#[derive(Debug, Clone)]
pub struct UpdateNotification {
    pub bot_id: String,
    pub new_version: String,
    pub changelog: String,
    pub requires_approval: bool,
}

/// Traffic branch configuration update
#[derive(Debug, Clone)]
pub struct BranchConfig {
    pub branch_id: String,
    pub percentage: u8,
    pub rules: Vec<u8>, // Serialized rules
}

/// Check results message
#[derive(Debug, Clone)]
pub struct CheckResultsMsg {
    pub project_path: PathBuf,
    pub score: u32,
    pub issues_count: u32,
    pub duration_ms: u64,
    pub serialized_results: Vec<u8>, // Full results in .sr format
}

/// File sync message
#[derive(Debug, Clone)]
pub struct FileSyncMsg {
    pub path: PathBuf,
    pub hash: u64,
    pub size: u64,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncAction {
    Upload,
    Download,
    Delete,
}

/// Status report
#[derive(Debug, Clone)]
pub struct StatusReport {
    pub daemon_type: DaemonType,
    pub uptime_secs: u64,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub active_tasks: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum DaemonType {
    Agent,
    Project,
}

/// Heartbeat message
#[derive(Debug, Clone)]
pub struct Heartbeat {
    pub timestamp: u64,
    pub sequence: u64,
}

/// Error message
#[derive(Debug, Clone)]
pub struct ErrorMsg {
    pub code: u32,
    pub message: String,
    pub recoverable: bool,
}

/// IPC client for connecting to daemons
pub struct IpcClient {
    socket_path: PathBuf,
    connected: bool,
    sequence: u64,
}

impl IpcClient {
    /// Create new client for agent daemon
    pub fn for_agent() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/dx-agent.sock"),
            connected: false,
            sequence: 0,
        }
    }

    /// Create new client for project daemon
    pub fn for_project(project_path: &PathBuf) -> Self {
        let hash = hash_path(project_path);
        Self {
            socket_path: PathBuf::from(format!("/tmp/dx-project-{}.sock", hash)),
            connected: false,
            sequence: 0,
        }
    }

    /// Connect to daemon
    pub async fn connect(&mut self) -> Result<()> {
        // TODO: Implement actual Unix socket connection
        self.connected = true;
        Ok(())
    }

    /// Send message to daemon
    pub async fn send(&mut self, msg: IpcMessage) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Not connected"));
        }

        // Serialize message using .sr format
        let _serialized = serialize_message(&msg)?;

        // TODO: Send over socket
        self.sequence += 1;
        Ok(())
    }

    /// Receive message from daemon
    pub async fn receive(&mut self) -> Result<IpcMessage> {
        if !self.connected {
            return Err(anyhow::anyhow!("Not connected"));
        }

        // TODO: Receive from socket and deserialize
        Ok(IpcMessage::Heartbeat(Heartbeat {
            timestamp: 0,
            sequence: self.sequence,
        }))
    }

    /// Send heartbeat
    pub async fn heartbeat(&mut self) -> Result<()> {
        self.send(IpcMessage::Heartbeat(Heartbeat {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            sequence: self.sequence,
        }))
        .await
    }

    /// Disconnect from daemon
    pub fn disconnect(&mut self) {
        self.connected = false;
    }
}

/// Serialize message to .sr format
fn serialize_message(_msg: &IpcMessage) -> Result<Vec<u8>> {
    // TODO: Use actual serializer crate
    // For now, return empty vec
    Ok(vec![])
}

/// Deserialize message from .sr format
fn deserialize_message(_data: &[u8]) -> Result<IpcMessage> {
    // TODO: Use actual serializer crate
    Ok(IpcMessage::Heartbeat(Heartbeat {
        timestamp: 0,
        sequence: 0,
    }))
}

fn hash_path(path: &PathBuf) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// IPC server for daemon
pub struct IpcServer {
    socket_path: PathBuf,
    running: bool,
}

impl IpcServer {
    /// Create new server
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            running: false,
        }
    }

    /// Start server
    pub async fn start(&mut self) -> Result<()> {
        // Remove existing socket
        let _ = std::fs::remove_file(&self.socket_path);

        // TODO: Bind Unix socket
        self.running = true;
        Ok(())
    }

    /// Accept connection
    pub async fn accept(&self) -> Result<IpcConnection> {
        // TODO: Accept incoming connection
        Ok(IpcConnection { id: rand::random() })
    }

    /// Stop server
    pub fn stop(&mut self) {
        self.running = false;
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// IPC connection handle
pub struct IpcConnection {
    pub id: u64,
}

impl IpcConnection {
    /// Send message to connected client
    pub async fn send(&self, msg: IpcMessage) -> Result<()> {
        let _serialized = serialize_message(&msg)?;
        // TODO: Send over connection
        Ok(())
    }

    /// Receive message from connected client
    pub async fn receive(&self) -> Result<IpcMessage> {
        // TODO: Receive and deserialize
        Ok(IpcMessage::Heartbeat(Heartbeat {
            timestamp: 0,
            sequence: 0,
        }))
    }
}
