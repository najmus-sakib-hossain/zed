//! N8n Sidecar Engine
//!
//! This module manages the n8n execution engine as a child process with IPC communication.
//! On Unix systems, it uses Unix Domain Sockets; on Windows, it uses named pipes or TCP.

use crate::protocol::{IpcMessage, IpcResponse, WorkflowDefinition, WorkflowResult};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, oneshot};

/// Configuration for the n8n sidecar process
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Path to the forked n8n project directory
    pub n8n_project_path: PathBuf,

    /// Path for the IPC socket/pipe
    pub ipc_path: String,

    /// Database type: "sqlite" | "postgres"
    pub db_type: String,

    /// Database connection string or path
    pub db_connection: String,

    /// Additional environment variables
    pub env_vars: HashMap<String, String>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            n8n_project_path: PathBuf::from("./n8n"),
            ipc_path: Self::default_ipc_path(),
            db_type: "sqlite".to_string(),
            db_connection: "./n8n/database.sqlite".to_string(),
            env_vars: HashMap::new(),
        }
    }
}

impl SidecarConfig {
    /// Get the default IPC path for the current platform
    #[cfg(unix)]
    fn default_ipc_path() -> String {
        "/tmp/dx-n8n-engine.sock".to_string()
    }

    #[cfg(windows)]
    fn default_ipc_path() -> String {
        // Windows uses TCP loopback instead of Unix sockets
        "127.0.0.1:58765".to_string()
    }
}

/// The n8n sidecar process manager
///
/// Manages the lifecycle of the n8n Node.js process and provides IPC communication.
pub struct N8nSidecar {
    /// The child process handle
    child: Option<Child>,

    /// IPC path (socket or TCP address)
    ipc_path: String,

    /// TCP stream for Windows / Unix socket stream for Unix
    stream: Arc<Mutex<Option<SidecarStream>>>,

    /// Pending request handlers
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<WorkflowResult>>>>,

    /// Whether the sidecar is connected
    connected: Arc<Mutex<bool>>,
}

/// Platform-specific stream type
#[cfg(unix)]
type SidecarStream = tokio::net::UnixStream;

#[cfg(windows)]
type SidecarStream = tokio::net::TcpStream;

impl N8nSidecar {
    /// Spawn the n8n sidecar process and establish IPC connection
    pub async fn spawn(config: SidecarConfig) -> Result<Self> {
        log::info!(
            "[n8n Sidecar] Starting with config: {:?}",
            config.n8n_project_path
        );

        // Clean up old socket (Unix only)
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&config.ipc_path);
        }

        // Build the n8n sidecar command
        let ipc_engine_path = config
            .n8n_project_path
            .join("dist")
            .join("ipc-engine.js");

        // Check if the IPC engine script exists
        if !ipc_engine_path.exists() {
            log::warn!(
                "[n8n Sidecar] IPC engine not found at {:?}, using mock mode",
                ipc_engine_path
            );
            return Self::spawn_mock(config).await;
        }

        let mut cmd = Command::new("node");
        cmd.arg(&ipc_engine_path)
            .env("N8N_IPC_SOCKET", &config.ipc_path)
            .env("N8N_EXECUTION_MODE", "ipc")
            .env("DB_TYPE", &config.db_type)
            .env("DB_SQLITE_DATABASE", &config.db_connection)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add custom environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .context("Failed to spawn n8n sidecar process")?;

        log::info!("[n8n Sidecar] Process spawned, waiting for IPC connection...");

        // Wait for the IPC connection to become available
        let stream = Self::connect_with_retry(&config.ipc_path, 50, 100).await?;

        log::info!("[n8n Sidecar] Connected via IPC");

        Ok(Self {
            child: Some(child),
            ipc_path: config.ipc_path,
            stream: Arc::new(Mutex::new(Some(stream))),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            connected: Arc::new(Mutex::new(true)),
        })
    }

    /// Spawn a mock sidecar for testing without actual n8n
    async fn spawn_mock(config: SidecarConfig) -> Result<Self> {
        log::info!("[n8n Sidecar] Running in mock mode");
        Ok(Self {
            child: None,
            ipc_path: config.ipc_path,
            stream: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            connected: Arc::new(Mutex::new(false)),
        })
    }

    /// Connect to the IPC endpoint with retry logic
    #[cfg(unix)]
    async fn connect_with_retry(
        path: &str,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Result<SidecarStream> {
        for i in 0..max_retries {
            match tokio::net::UnixStream::connect(path).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    if i == max_retries - 1 {
                        return Err(anyhow::anyhow!(
                            "Failed to connect to n8n sidecar after {} retries: {}",
                            max_retries,
                            e
                        ));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms)).await;
                }
            }
        }
        unreachable!()
    }

    #[cfg(windows)]
    async fn connect_with_retry(
        address: &str,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Result<SidecarStream> {
        for i in 0..max_retries {
            match tokio::net::TcpStream::connect(address).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    if i == max_retries - 1 {
                        return Err(anyhow::anyhow!(
                            "Failed to connect to n8n sidecar after {} retries: {}",
                            max_retries,
                            e
                        ));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms)).await;
                }
            }
        }
        unreachable!()
    }

    /// Check if the sidecar is connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    /// Execute a workflow synchronously (blocking until completion)
    pub async fn execute_workflow(
        &self,
        workflow: &WorkflowDefinition,
        input_data: serde_json::Value,
    ) -> Result<WorkflowResult> {
        // Check for mock mode
        if !self.is_connected().await {
            return self.execute_workflow_mock(workflow, input_data).await;
        }

        let request_id = uuid::Uuid::new_v4().to_string();

        let message = IpcMessage {
            id: request_id.clone(),
            msg_type: "execute".to_string(),
            payload: serde_json::json!({
                "workflow": workflow,
                "input_data": input_data,
                "mode": "integrated",
            }),
        };

        self.send_and_receive(message).await
    }

    /// Execute a workflow asynchronously (fire-and-forget)
    pub async fn execute_workflow_async(
        &self,
        workflow: &WorkflowDefinition,
        input_data: serde_json::Value,
    ) -> Result<String> {
        let request_id = uuid::Uuid::new_v4().to_string();

        let message = IpcMessage {
            id: request_id.clone(),
            msg_type: "execute_async".to_string(),
            payload: serde_json::json!({
                "workflow": workflow,
                "input_data": input_data,
            }),
        };

        // Send without waiting for full result
        self.send_message(&message).await?;
        Ok(request_id)
    }

    /// Get the status of an async execution
    pub async fn get_execution_status(&self, execution_id: &str) -> Result<WorkflowResult> {
        let message = IpcMessage {
            id: execution_id.to_string(),
            msg_type: "get_status".to_string(),
            payload: serde_json::json!({
                "execution_id": execution_id,
            }),
        };

        self.send_and_receive(message).await
    }

    /// Stop a running execution
    pub async fn stop_execution(&self, execution_id: &str) -> Result<()> {
        let message = IpcMessage {
            id: execution_id.to_string(),
            msg_type: "stop".to_string(),
            payload: serde_json::json!({
                "execution_id": execution_id,
            }),
        };

        self.send_message(&message).await
    }

    /// Send a message and wait for the response
    async fn send_and_receive(&self, message: IpcMessage) -> Result<WorkflowResult> {
        let mut stream_guard = self.stream.lock().await;

        if let Some(ref mut stream) = *stream_guard {
            // Serialize and send
            let serialized = serde_json::to_string(&message)? + "\n";
            stream
                .write_all(serialized.as_bytes())
                .await
                .context("Failed to write to sidecar")?;
            stream.flush().await?;

            // Read response
            let mut reader = BufReader::new(stream);
            let mut response_line = String::new();
            reader
                .read_line(&mut response_line)
                .await
                .context("Failed to read from sidecar")?;

            let response: IpcResponse = serde_json::from_str(&response_line)
                .context("Failed to parse sidecar response")?;

            // Check for error response
            if response.msg_type == "error" {
                let error_msg = response
                    .payload
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("Sidecar error: {}", error_msg);
            }

            let result: WorkflowResult = serde_json::from_value(response.payload)
                .context("Failed to parse workflow result")?;

            Ok(result)
        } else {
            anyhow::bail!("No connection to n8n sidecar")
        }
    }

    /// Send a message without waiting for response
    async fn send_message(&self, message: &IpcMessage) -> Result<()> {
        let mut stream_guard = self.stream.lock().await;

        if let Some(ref mut stream) = *stream_guard {
            let serialized = serde_json::to_string(message)? + "\n";
            stream.write_all(serialized.as_bytes()).await?;
            stream.flush().await?;
            Ok(())
        } else {
            anyhow::bail!("No connection to n8n sidecar")
        }
    }

    /// Mock execution for testing without n8n
    async fn execute_workflow_mock(
        &self,
        workflow: &WorkflowDefinition,
        input_data: serde_json::Value,
    ) -> Result<WorkflowResult> {
        log::info!(
            "[n8n Sidecar Mock] Executing workflow: {}",
            workflow.name
        );

        // Simulate execution delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(WorkflowResult {
            execution_id: uuid::Uuid::new_v4().to_string(),
            status: "success".to_string(),
            data: serde_json::json!({
                "mock": true,
                "workflow_name": workflow.name,
                "input_data": input_data,
                "nodes_executed": workflow.nodes.len(),
            }),
            execution_time_ms: 100,
        })
    }

    /// Gracefully shutdown the sidecar
    pub async fn shutdown(&mut self) -> Result<()> {
        log::info!("[n8n Sidecar] Shutting down...");

        // Send shutdown command
        if self.is_connected().await {
            let message = IpcMessage {
                id: "shutdown".to_string(),
                msg_type: "shutdown".to_string(),
                payload: serde_json::json!({}),
            };

            let _ = self.send_message(&message).await;
        }

        // Mark as disconnected
        *self.connected.lock().await = false;

        // Close the stream
        let mut stream_guard = self.stream.lock().await;
        stream_guard.take();

        // Kill the child process
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            log::info!("[n8n Sidecar] Process terminated");
        }

        // Clean up socket (Unix only)
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&self.ipc_path);
        }

        Ok(())
    }
}

impl Drop for N8nSidecar {
    fn drop(&mut self) {
        // Best-effort cleanup
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }

        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&self.ipc_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_sidecar() {
        let config = SidecarConfig::default();
        let sidecar = N8nSidecar::spawn_mock(config).await.unwrap();

        let workflow = crate::protocol::WorkflowBuilder::new("Test")
            .add_trigger()
            .build();

        let result = sidecar
            .execute_workflow(&workflow, serde_json::json!({"test": true}))
            .await
            .unwrap();

        assert!(result.is_success());
        assert!(result.data.get("mock").and_then(|m| m.as_bool()).unwrap());
    }
}
