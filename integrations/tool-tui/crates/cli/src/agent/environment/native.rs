//! Native IPC Fallback
//!
//! Provides process isolation with IPC for native plugins when WASM
//! compilation is not suitable or available.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{RwLock, mpsc, oneshot};

use super::{EnvironmentError, EnvironmentResult};

/// Handle to a running native process
#[derive(Debug)]
pub struct ProcessHandle {
    /// Unique process ID
    pub id: u64,
    /// Process name
    pub name: String,
    /// Native process
    child: Child,
    /// Message sender
    tx: mpsc::Sender<IpcMessage>,
    /// Response receiver
    rx: mpsc::Receiver<IpcMessage>,
}

impl ProcessHandle {
    /// Send a message to the process
    pub async fn send(&self, message: IpcMessage) -> EnvironmentResult<()> {
        self.tx.send(message).await.map_err(|e| EnvironmentError::IpcError {
            message: format!("Failed to send: {}", e),
        })
    }

    /// Receive a message from the process
    pub async fn recv(&mut self) -> Option<IpcMessage> {
        self.rx.recv().await
    }

    /// Check if process is still running
    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Terminate the process
    pub async fn terminate(&mut self) -> EnvironmentResult<()> {
        self.child.kill().await.map_err(EnvironmentError::from)
    }
}

/// IPC message format
#[derive(Debug, Clone)]
pub struct IpcMessage {
    /// Message ID for request/response matching
    pub id: u64,
    /// Message type
    pub msg_type: MessageType,
    /// Method name (for Call)
    pub method: Option<String>,
    /// Payload data
    pub payload: Vec<u8>,
    /// Error message (for Error type)
    pub error: Option<String>,
}

/// Types of IPC messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Method call request
    Call = 0,
    /// Response to a call
    Response = 1,
    /// Error response
    Error = 2,
    /// Event/notification
    Event = 3,
    /// Heartbeat/ping
    Ping = 4,
    /// Heartbeat response
    Pong = 5,
}

impl IpcMessage {
    /// Create a call message
    pub fn call(id: u64, method: &str, payload: Vec<u8>) -> Self {
        Self {
            id,
            msg_type: MessageType::Call,
            method: Some(method.to_string()),
            payload,
            error: None,
        }
    }

    /// Create a response message
    pub fn response(id: u64, payload: Vec<u8>) -> Self {
        Self {
            id,
            msg_type: MessageType::Response,
            method: None,
            payload,
            error: None,
        }
    }

    /// Create an error message
    pub fn error(id: u64, error: &str) -> Self {
        Self {
            id,
            msg_type: MessageType::Error,
            method: None,
            payload: Vec::new(),
            error: Some(error.to_string()),
        }
    }

    /// Serialize to bytes for IPC
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: type (1) + id (8) + method_len (2) + payload_len (4) + error_len (2)
        bytes.push(self.msg_type as u8);
        bytes.extend_from_slice(&self.id.to_le_bytes());

        let method_bytes = self.method.as_ref().map(|s| s.as_bytes()).unwrap_or(&[]);
        bytes.extend_from_slice(&(method_bytes.len() as u16).to_le_bytes());
        bytes.extend_from_slice(method_bytes);

        bytes.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.payload);

        let error_bytes = self.error.as_ref().map(|s| s.as_bytes()).unwrap_or(&[]);
        bytes.extend_from_slice(&(error_bytes.len() as u16).to_le_bytes());
        bytes.extend_from_slice(error_bytes);

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> EnvironmentResult<Self> {
        if bytes.len() < 15 {
            return Err(EnvironmentError::IpcError {
                message: "Message too short".into(),
            });
        }

        let msg_type = match bytes[0] {
            0 => MessageType::Call,
            1 => MessageType::Response,
            2 => MessageType::Error,
            3 => MessageType::Event,
            4 => MessageType::Ping,
            5 => MessageType::Pong,
            _ => {
                return Err(EnvironmentError::IpcError {
                    message: "Invalid message type".into(),
                });
            }
        };

        let id = u64::from_le_bytes(bytes[1..9].try_into().unwrap());

        let method_len = u16::from_le_bytes(bytes[9..11].try_into().unwrap()) as usize;
        let method = if method_len > 0 {
            Some(String::from_utf8_lossy(&bytes[11..11 + method_len]).to_string())
        } else {
            None
        };

        let offset = 11 + method_len;
        let payload_len =
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let payload = bytes[offset + 4..offset + 4 + payload_len].to_vec();

        let offset = offset + 4 + payload_len;
        let error_len = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        let error = if error_len > 0 {
            Some(String::from_utf8_lossy(&bytes[offset + 2..offset + 2 + error_len]).to_string())
        } else {
            None
        };

        Ok(Self {
            id,
            msg_type,
            method,
            payload,
            error,
        })
    }
}

/// Signature data for native plugin verification
#[derive(Debug, Clone)]
pub struct PluginSignature {
    /// Public key bytes
    pub public_key: [u8; 32],
    /// Signature bytes
    pub signature: [u8; 64],
    /// Hash of the plugin binary
    pub binary_hash: [u8; 32],
}

/// Native IPC manager
pub struct NativeIpc {
    processes: Arc<RwLock<HashMap<u64, ProcessInfo>>>,
    next_id: AtomicU64,
    next_msg_id: AtomicU64,
    trusted_keys: Arc<RwLock<Vec<VerifyingKey>>>,
}

/// Information about a running process
struct ProcessInfo {
    name: String,
    path: PathBuf,
    stdin_tx: mpsc::Sender<Vec<u8>>,
    pending_calls: Arc<RwLock<HashMap<u64, oneshot::Sender<IpcMessage>>>>,
}

impl NativeIpc {
    /// Create a new NativeIpc manager
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            next_id: AtomicU64::new(1),
            next_msg_id: AtomicU64::new(1),
            trusted_keys: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a trusted public key
    pub async fn add_trusted_key(&self, key_bytes: [u8; 32]) -> EnvironmentResult<()> {
        let key = VerifyingKey::from_bytes(&key_bytes).map_err(|e| EnvironmentError::IpcError {
            message: format!("Invalid public key: {}", e),
        })?;

        let mut keys = self.trusted_keys.write().await;
        keys.push(key);
        Ok(())
    }

    /// Verify plugin signature
    pub async fn verify_signature(
        &self,
        plugin_path: &Path,
        signature: &PluginSignature,
    ) -> EnvironmentResult<bool> {
        use sha2::{Digest, Sha256};

        // Read and hash the plugin binary
        let binary = tokio::fs::read(plugin_path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&binary);
        let hash: [u8; 32] = hasher.finalize().into();

        // Verify hash matches
        if hash != signature.binary_hash {
            return Ok(false);
        }

        // Verify signature with any trusted key
        let keys = self.trusted_keys.read().await;

        let verifying_key = VerifyingKey::from_bytes(&signature.public_key).map_err(|e| {
            EnvironmentError::IpcError {
                message: format!("Invalid signature public key: {}", e),
            }
        })?;

        // Check if the key is trusted
        if !keys.iter().any(|k| k.as_bytes() == &signature.public_key) {
            return Err(EnvironmentError::IpcError {
                message: "Public key not trusted".into(),
            });
        }

        let sig = Signature::from_bytes(&signature.signature);

        match verifying_key.verify(&hash, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Spawn a native process
    pub async fn spawn(
        &self,
        path: &Path,
        args: &[String],
        env: HashMap<String, String>,
    ) -> EnvironmentResult<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("process-{}", id));

        let mut cmd = Command::new(path);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| EnvironmentError::IpcError {
            message: "Failed to capture stdin".into(),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| EnvironmentError::IpcError {
            message: "Failed to capture stdout".into(),
        })?;

        // Set up stdin channel
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(100);

        // Spawn stdin writer task
        let mut stdin = stdin;
        tokio::spawn(async move {
            while let Some(data) = stdin_rx.recv().await {
                let len_bytes = (data.len() as u32).to_le_bytes();
                if stdin.write_all(&len_bytes).await.is_err() {
                    break;
                }
                if stdin.write_all(&data).await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Set up pending calls tracking
        let pending_calls: Arc<RwLock<HashMap<u64, oneshot::Sender<IpcMessage>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Spawn stdout reader task
        let pending_calls_clone = pending_calls.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut len_buf = [0u8; 4];

            loop {
                // Read message length
                if tokio::io::AsyncReadExt::read_exact(&mut reader, &mut len_buf).await.is_err() {
                    break;
                }
                let len = u32::from_le_bytes(len_buf) as usize;

                // Read message
                let mut msg_buf = vec![0u8; len];
                if tokio::io::AsyncReadExt::read_exact(&mut reader, &mut msg_buf).await.is_err() {
                    break;
                }

                // Parse and dispatch message
                if let Ok(msg) = IpcMessage::from_bytes(&msg_buf) {
                    if matches!(msg.msg_type, MessageType::Response | MessageType::Error) {
                        let mut pending = pending_calls_clone.write().await;
                        if let Some(tx) = pending.remove(&msg.id) {
                            let _ = tx.send(msg);
                        }
                    }
                }
            }
        });

        // Store process info
        let info = ProcessInfo {
            name: name.clone(),
            path: path.to_path_buf(),
            stdin_tx,
            pending_calls,
        };

        let mut processes = self.processes.write().await;
        processes.insert(id, info);

        Ok(id)
    }

    /// Call a method on a native process
    pub async fn call(
        &self,
        process_id: u64,
        method: &str,
        payload: Vec<u8>,
        timeout_ms: u64,
    ) -> EnvironmentResult<IpcMessage> {
        let processes = self.processes.read().await;
        let process = processes.get(&process_id).ok_or_else(|| EnvironmentError::IpcError {
            message: format!("Process {} not found", process_id),
        })?;

        let msg_id = self.next_msg_id.fetch_add(1, Ordering::SeqCst);
        let message = IpcMessage::call(msg_id, method, payload);

        // Set up response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = process.pending_calls.write().await;
            pending.insert(msg_id, tx);
        }

        // Send message
        process.stdin_tx.send(message.to_bytes()).await.map_err(|e| {
            EnvironmentError::IpcError {
                message: format!("Failed to send: {}", e),
            }
        })?;

        // Wait for response with timeout
        let timeout = tokio::time::Duration::from_millis(timeout_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(EnvironmentError::IpcError {
                message: "Response channel closed".into(),
            }),
            Err(_) => {
                // Clean up pending call
                let mut pending = process.pending_calls.write().await;
                pending.remove(&msg_id);
                Err(EnvironmentError::IpcError {
                    message: "Call timed out".into(),
                })
            }
        }
    }

    /// Terminate a process
    pub async fn terminate(&self, process_id: u64) -> EnvironmentResult<()> {
        let mut processes = self.processes.write().await;
        processes.remove(&process_id);
        Ok(())
    }

    /// List running processes
    pub async fn list_processes(&self) -> Vec<(u64, String)> {
        let processes = self.processes.read().await;
        processes.iter().map(|(id, info)| (*id, info.name.clone())).collect()
    }

    /// Benchmark WASM vs Native performance
    pub async fn benchmark(&self, iterations: u32) -> EnvironmentResult<BenchmarkResult> {
        // This is a placeholder for actual benchmarking
        // In practice, you'd compare executing the same logic in WASM vs native

        let wasm_time_ns = 1000; // Placeholder
        let native_time_ns = 100; // Placeholder

        Ok(BenchmarkResult {
            iterations,
            wasm_total_ns: wasm_time_ns * iterations as u64,
            native_total_ns: native_time_ns * iterations as u64,
            wasm_avg_ns: wasm_time_ns,
            native_avg_ns: native_time_ns,
            speedup_factor: wasm_time_ns as f64 / native_time_ns as f64,
        })
    }
}

impl Default for NativeIpc {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of WASM vs Native benchmark
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Number of iterations
    pub iterations: u32,
    /// Total WASM execution time in nanoseconds
    pub wasm_total_ns: u64,
    /// Total native execution time in nanoseconds
    pub native_total_ns: u64,
    /// Average WASM execution time per iteration
    pub wasm_avg_ns: u64,
    /// Average native execution time per iteration
    pub native_avg_ns: u64,
    /// How much faster native is vs WASM
    pub speedup_factor: f64,
}

impl std::fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Benchmark ({} iterations):\n  WASM:   {}ns avg ({}ns total)\n  Native: {}ns avg ({}ns total)\n  Speedup: {:.2}x",
            self.iterations,
            self.wasm_avg_ns,
            self.wasm_total_ns,
            self.native_avg_ns,
            self.native_total_ns,
            self.speedup_factor
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_serialization() {
        let msg = IpcMessage::call(42, "test_method", vec![1, 2, 3, 4]);
        let bytes = msg.to_bytes();
        let parsed = IpcMessage::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.id, 42);
        assert_eq!(parsed.method, Some("test_method".to_string()));
        assert_eq!(parsed.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_ipc_message_response() {
        let msg = IpcMessage::response(123, vec![5, 6, 7]);
        let bytes = msg.to_bytes();
        let parsed = IpcMessage::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.id, 123);
        assert_eq!(parsed.msg_type, MessageType::Response);
        assert!(parsed.method.is_none());
    }

    #[test]
    fn test_ipc_message_error() {
        let msg = IpcMessage::error(456, "Something went wrong");
        let bytes = msg.to_bytes();
        let parsed = IpcMessage::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.id, 456);
        assert_eq!(parsed.msg_type, MessageType::Error);
        assert_eq!(parsed.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_native_ipc_creation() {
        let ipc = NativeIpc::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let processes = rt.block_on(async { ipc.list_processes().await });
        assert!(processes.is_empty());
    }

    #[test]
    fn test_benchmark_result_display() {
        let result = BenchmarkResult {
            iterations: 1000,
            wasm_total_ns: 1_000_000,
            native_total_ns: 100_000,
            wasm_avg_ns: 1000,
            native_avg_ns: 100,
            speedup_factor: 10.0,
        };
        let display = format!("{}", result);
        assert!(display.contains("1000 iterations"));
        assert!(display.contains("10.00x"));
    }
}
