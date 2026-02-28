//! .sr File Exchange System
//!
//! Protocol for inter-daemon communication using DX Serializer format
//!
//! # Benefits
//! - 52-73% token savings over JSON
//! - Zero-copy deserialization
//! - Type-safe message exchange
//! - RKYV backend for machine format

use anyhow::Result;
use std::path::PathBuf;

pub mod encoder;
pub mod messages;
pub mod protocol;

/// Exchange message envelope
#[derive(Debug, Clone)]
pub struct Envelope {
    /// Message ID (UUID)
    pub id: String,

    /// Message type identifier
    pub msg_type: MessageType,

    /// Sender identifier
    pub sender: DaemonId,

    /// Recipient identifier (None = broadcast)
    pub recipient: Option<DaemonId>,

    /// Timestamp (Unix millis)
    pub timestamp: u64,

    /// Correlation ID for request/response
    pub correlation_id: Option<String>,

    /// TTL in seconds (0 = no expiry)
    pub ttl: u32,

    /// Payload in .sr format
    pub payload: Vec<u8>,
}

/// Daemon identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DaemonId {
    Agent,
    Project(String), // Project path hash
}

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    // Check operations
    CheckRequest,
    CheckResult,

    // Score operations
    ScoreRequest,
    ScoreResult,

    // Sync operations
    SyncRequest,
    SyncProgress,
    SyncComplete,

    // Branch operations
    BranchConfig,
    BranchUpdate,

    // AI operations
    AiUpdateRequest,
    AiUpdateApproval,
    AiUpdateResult,

    // Health operations
    Heartbeat,
    StatusRequest,
    StatusResponse,

    // Error handling
    Error,
    Ack,
}

impl Envelope {
    /// Create new envelope
    pub fn new(msg_type: MessageType, sender: DaemonId, payload: Vec<u8>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            msg_type,
            sender,
            recipient: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            correlation_id: None,
            ttl: 60,
            payload,
        }
    }

    /// Set recipient
    pub fn to(mut self, recipient: DaemonId) -> Self {
        self.recipient = Some(recipient);
        self
    }

    /// Set correlation ID
    pub fn correlate(mut self, id: &str) -> Self {
        self.correlation_id = Some(id.to_string());
        self
    }

    /// Set TTL
    pub fn expires_in(mut self, secs: u32) -> Self {
        self.ttl = secs;
        self
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        if self.ttl == 0 {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        now > self.timestamp + (self.ttl as u64 * 1000)
    }

    /// Serialize to .sr format
    pub fn to_sr(&self) -> Result<Vec<u8>> {
        encoder::encode_envelope(self)
    }

    /// Deserialize from .sr format
    pub fn from_sr(data: &[u8]) -> Result<Self> {
        encoder::decode_envelope(data)
    }

    /// Serialize to LLM format (human readable, token-efficient)
    pub fn to_llm(&self) -> String {
        let mut output = String::new();
        output.push_str("MSG ");
        output.push_str(&self.id[..8]); // Short ID
        output.push('|');
        output.push_str(&format!("{:?}", self.msg_type));
        output.push('|');
        output.push_str(&format!("{:?}", self.sender));
        if let Some(ref r) = self.recipient {
            output.push_str(&format!("|to:{:?}", r));
        }
        if let Some(ref c) = self.correlation_id {
            output.push_str(&format!("|cor:{}", &c[..8]));
        }
        output.push_str(&format!("|{}b", self.payload.len()));
        output
    }
}

/// Exchange channel for communication
pub struct ExchangeChannel {
    local_id: DaemonId,
    socket_path: PathBuf,
    pending_responses: std::collections::HashMap<String, tokio::sync::oneshot::Sender<Envelope>>,
}

impl ExchangeChannel {
    /// Create new channel
    pub fn new(local_id: DaemonId, socket_path: PathBuf) -> Self {
        Self {
            local_id,
            socket_path,
            pending_responses: std::collections::HashMap::new(),
        }
    }

    /// Send message (fire and forget)
    pub async fn send(&self, envelope: Envelope) -> Result<()> {
        let _data = envelope.to_sr()?;
        // TODO: Send over socket
        Ok(())
    }

    /// Send request and wait for response
    pub async fn request(&mut self, envelope: Envelope, timeout_secs: u64) -> Result<Envelope> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let id = envelope.id.clone();
        self.pending_responses.insert(id.clone(), tx);

        self.send(envelope).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(anyhow::anyhow!("Channel closed")),
            Err(_) => {
                self.pending_responses.remove(&id);
                Err(anyhow::anyhow!("Request timed out"))
            }
        }
    }

    /// Receive next message
    pub async fn receive(&self) -> Result<Envelope> {
        // TODO: Receive from socket
        Err(anyhow::anyhow!("Not implemented"))
    }

    /// Handle incoming response
    pub fn handle_response(&mut self, envelope: Envelope) {
        if let Some(correlation_id) = &envelope.correlation_id {
            if let Some(tx) = self.pending_responses.remove(correlation_id) {
                let _ = tx.send(envelope);
            }
        }
    }
}

/// Message queue for buffering
pub struct MessageQueue {
    queue: std::collections::VecDeque<Envelope>,
    max_size: usize,
}

impl MessageQueue {
    /// Create new queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: std::collections::VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push message to queue
    pub fn push(&mut self, envelope: Envelope) -> bool {
        if self.queue.len() >= self.max_size {
            return false;
        }
        self.queue.push_back(envelope);
        true
    }

    /// Pop message from queue
    pub fn pop(&mut self) -> Option<Envelope> {
        self.queue.pop_front()
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Remove expired messages
    pub fn cleanup_expired(&mut self) {
        self.queue.retain(|e| !e.is_expired());
    }
}
