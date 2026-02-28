//! DCP Server implementation.
//!
//! Provides the main server struct with router, context, and session management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::context::DcpContext;
use crate::dispatch::{BinaryTrieRouter, ServerCapabilities, SharedArgs, ToolResult};
use crate::DCPError;

/// Protocol version for DCP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolVersion {
    /// MCP JSON-RPC protocol
    #[default]
    Mcp,
    /// DCP binary protocol v1
    DcpV1,
}

/// Session state for a connected client
#[derive(Debug)]
pub struct Session {
    /// Unique session ID
    pub id: u64,
    /// Current protocol version
    pub protocol: ProtocolVersion,
    /// Session creation timestamp
    pub created_at: u64,
    /// Last activity timestamp
    pub last_activity: AtomicU64,
    /// Custom session data
    pub data: RwLock<HashMap<String, Vec<u8>>>,
    /// Message count
    pub message_count: AtomicU64,
}

impl Session {
    /// Create a new session
    pub fn new(id: u64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        Self {
            id,
            protocol: ProtocolVersion::Mcp,
            created_at: now,
            last_activity: AtomicU64::new(now),
            data: RwLock::new(HashMap::new()),
            message_count: AtomicU64::new(0),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        self.last_activity.store(now, Ordering::Release);
    }

    /// Increment message count
    pub fn increment_messages(&self) -> u64 {
        self.message_count.fetch_add(1, Ordering::AcqRel)
    }

    /// Get session data
    pub fn get_data(&self, key: &str) -> Option<Vec<u8>> {
        self.data.read().ok()?.get(key).cloned()
    }

    /// Set session data
    pub fn set_data(&self, key: String, value: Vec<u8>) {
        if let Ok(mut data) = self.data.write() {
            data.insert(key, value);
        }
    }

    /// Upgrade protocol version
    pub fn upgrade_protocol(&mut self, version: ProtocolVersion) {
        self.protocol = version;
    }

    /// Check if session is using DCP protocol
    pub fn is_dcp(&self) -> bool {
        matches!(self.protocol, ProtocolVersion::DcpV1)
    }
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Maximum concurrent sessions
    pub max_sessions: usize,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Server name for identification
    pub server_name: String,
    /// Server version
    pub server_version: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_sessions: 1000,
            session_timeout_secs: 3600,
            enable_metrics: true,
            server_name: "dcp-server".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Performance metrics for protocol comparison
#[derive(Debug, Default)]
pub struct Metrics {
    /// MCP message count
    pub mcp_messages: AtomicU64,
    /// DCP message count
    pub dcp_messages: AtomicU64,
    /// MCP total bytes
    pub mcp_bytes: AtomicU64,
    /// DCP total bytes
    pub dcp_bytes: AtomicU64,
    /// MCP total latency (microseconds)
    pub mcp_latency_us: AtomicU64,
    /// DCP total latency (microseconds)
    pub dcp_latency_us: AtomicU64,
    /// Tool invocation count
    pub tool_invocations: AtomicU64,
    /// Error count
    pub errors: AtomicU64,
}

impl Metrics {
    /// Record an MCP message
    pub fn record_mcp(&self, bytes: u64, latency_us: u64) {
        self.mcp_messages.fetch_add(1, Ordering::Relaxed);
        self.mcp_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.mcp_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a DCP message
    pub fn record_dcp(&self, bytes: u64, latency_us: u64) {
        self.dcp_messages.fetch_add(1, Ordering::Relaxed);
        self.dcp_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.dcp_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a tool invocation
    pub fn record_invocation(&self) {
        self.tool_invocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get average MCP latency in microseconds
    pub fn avg_mcp_latency_us(&self) -> u64 {
        let count = self.mcp_messages.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.mcp_latency_us.load(Ordering::Relaxed) / count
    }

    /// Get average DCP latency in microseconds
    pub fn avg_dcp_latency_us(&self) -> u64 {
        let count = self.dcp_messages.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.dcp_latency_us.load(Ordering::Relaxed) / count
    }

    /// Get average MCP message size
    pub fn avg_mcp_size(&self) -> u64 {
        let count = self.mcp_messages.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.mcp_bytes.load(Ordering::Relaxed) / count
    }

    /// Get average DCP message size
    pub fn avg_dcp_size(&self) -> u64 {
        let count = self.dcp_messages.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.dcp_bytes.load(Ordering::Relaxed) / count
    }

    /// Get snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            mcp_messages: self.mcp_messages.load(Ordering::Relaxed),
            dcp_messages: self.dcp_messages.load(Ordering::Relaxed),
            mcp_bytes: self.mcp_bytes.load(Ordering::Relaxed),
            dcp_bytes: self.dcp_bytes.load(Ordering::Relaxed),
            avg_mcp_latency_us: self.avg_mcp_latency_us(),
            avg_dcp_latency_us: self.avg_dcp_latency_us(),
            tool_invocations: self.tool_invocations.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub mcp_messages: u64,
    pub dcp_messages: u64,
    pub mcp_bytes: u64,
    pub dcp_bytes: u64,
    pub avg_mcp_latency_us: u64,
    pub avg_dcp_latency_us: u64,
    pub tool_invocations: u64,
    pub errors: u64,
}

/// DCP Server
pub struct DcpServer {
    /// Tool router
    pub router: BinaryTrieRouter,
    /// Shared context
    pub context: Arc<DcpContext>,
    /// Server configuration
    pub config: ServerConfig,
    /// Active sessions
    sessions: RwLock<HashMap<u64, Arc<Session>>>,
    /// Session ID counter
    session_counter: AtomicU64,
    /// Performance metrics
    pub metrics: Arc<Metrics>,
}

impl DcpServer {
    /// Create a new DCP server
    pub fn new(router: BinaryTrieRouter, context: DcpContext, config: ServerConfig) -> Self {
        Self {
            router,
            context: Arc::new(context),
            config,
            sessions: RwLock::new(HashMap::new()),
            session_counter: AtomicU64::new(1),
            metrics: Arc::new(Metrics::default()),
        }
    }

    /// Create a new session
    pub fn create_session(&self) -> Result<Arc<Session>, DCPError> {
        let sessions = self.sessions.read().map_err(|_| DCPError::InternalError)?;
        if sessions.len() >= self.config.max_sessions {
            return Err(DCPError::ResourceExhausted);
        }
        drop(sessions);

        let id = self.session_counter.fetch_add(1, Ordering::SeqCst);
        let session = Arc::new(Session::new(id));

        let mut sessions = self.sessions.write().map_err(|_| DCPError::InternalError)?;
        sessions.insert(id, Arc::clone(&session));

        Ok(session)
    }

    /// Get a session by ID
    pub fn get_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.read().ok()?.get(&id).cloned()
    }

    /// Remove a session
    pub fn remove_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.write().ok()?.remove(&id)
    }

    /// Get active session count
    pub fn session_count(&self) -> usize {
        self.sessions.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Invoke a tool by ID
    pub fn invoke(&self, tool_id: u16, args: &SharedArgs) -> Result<ToolResult, DCPError> {
        if self.config.enable_metrics {
            self.metrics.record_invocation();
        }

        let handler = self.router.dispatch(tool_id).ok_or(DCPError::ToolNotFound)?;

        handler.execute(args)
    }

    /// Invoke a tool by name (for MCP compatibility)
    pub fn invoke_by_name(&self, name: &str, args: &SharedArgs) -> Result<ToolResult, DCPError> {
        let tool_id = self.router.resolve_name(name).ok_or(DCPError::ToolNotFound)?;

        self.invoke(tool_id, args)
    }

    /// Upgrade a session from MCP to DCP
    pub fn upgrade_session(&self, session_id: u64) -> Result<(), DCPError> {
        let _session = self.get_session(session_id).ok_or(DCPError::SessionNotFound)?;

        // Session data is preserved during upgrade
        // Only the protocol version changes
        let mut sessions = self.sessions.write().map_err(|_| DCPError::InternalError)?;
        if let Some(session) = sessions.get_mut(&session_id) {
            // Create new session with upgraded protocol
            let mut new_session = Session::new(session_id);
            new_session.protocol = ProtocolVersion::DcpV1;

            // Copy over session data
            if let Ok(old_data) = session.data.read() {
                if let Ok(mut new_data) = new_session.data.write() {
                    for (k, v) in old_data.iter() {
                        new_data.insert(k.clone(), v.clone());
                    }
                }
            }

            *session = Arc::new(new_session);
        }

        Ok(())
    }

    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&self) -> usize {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        let mut sessions = match self.sessions.write() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let expired: Vec<u64> = sessions
            .iter()
            .filter(|(_, session)| {
                let last = session.last_activity.load(Ordering::Acquire);
                now - last > self.config.session_timeout_secs
            })
            .map(|(id, _)| *id)
            .collect();

        let count = expired.len();
        for id in expired {
            sessions.remove(&id);
        }

        count
    }

    /// Get server info
    pub fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: self.config.server_name.clone(),
            version: self.config.server_version.clone(),
            protocol_version: "1.0".to_string(),
            capabilities: self.router.capabilities(),
        }
    }
}

/// Server information for capability negotiation
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::ToolHandler;
    use crate::protocol::ToolSchema;

    struct TestHandler;

    impl ToolHandler for TestHandler {
        fn execute(&self, _args: &SharedArgs) -> Result<ToolResult, DCPError> {
            Ok(ToolResult::success(vec![1, 2, 3]))
        }

        fn schema(&self) -> &ToolSchema {
            static SCHEMA: ToolSchema = ToolSchema {
                name: "test",
                id: 1,
                description: "Test tool",
                input: crate::protocol::InputSchema {
                    required: 0,
                    fields: Vec::new(),
                },
            };
            &SCHEMA
        }
    }

    #[test]
    fn test_session_creation() {
        let session = Session::new(1);
        assert_eq!(session.id, 1);
        assert_eq!(session.protocol, ProtocolVersion::Mcp);
        assert!(!session.is_dcp());
    }

    #[test]
    fn test_session_data() {
        let session = Session::new(1);
        session.set_data("key".to_string(), vec![1, 2, 3]);
        assert_eq!(session.get_data("key"), Some(vec![1, 2, 3]));
        assert_eq!(session.get_data("missing"), None);
    }

    #[test]
    fn test_session_touch() {
        let session = Session::new(1);
        let initial = session.last_activity.load(Ordering::Acquire);
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();
        let updated = session.last_activity.load(Ordering::Acquire);
        assert!(updated >= initial);
    }

    #[test]
    fn test_metrics() {
        let metrics = Metrics::default();

        metrics.record_mcp(100, 1000);
        metrics.record_mcp(200, 2000);
        metrics.record_dcp(50, 500);

        assert_eq!(metrics.mcp_messages.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.dcp_messages.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.avg_mcp_latency_us(), 1500);
        assert_eq!(metrics.avg_dcp_latency_us(), 500);
    }

    #[test]
    fn test_server_session_management() {
        let router = BinaryTrieRouter::new();
        let context = DcpContext::new(1);
        let config = ServerConfig {
            max_sessions: 10,
            ..Default::default()
        };
        let server = DcpServer::new(router, context, config);

        // Create session
        let session = server.create_session().unwrap();
        assert_eq!(session.id, 1);
        assert_eq!(server.session_count(), 1);

        // Get session
        let retrieved = server.get_session(1).unwrap();
        assert_eq!(retrieved.id, 1);

        // Remove session
        server.remove_session(1);
        assert_eq!(server.session_count(), 0);
    }

    #[test]
    fn test_server_max_sessions() {
        let router = BinaryTrieRouter::new();
        let context = DcpContext::new(1);
        let config = ServerConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let server = DcpServer::new(router, context, config);

        server.create_session().unwrap();
        server.create_session().unwrap();

        let result = server.create_session();
        assert!(matches!(result, Err(DCPError::ResourceExhausted)));
    }
}
