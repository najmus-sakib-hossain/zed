//! Daemon Client with Retry Logic
//!
//! Provides connection management to DX daemons with exponential backoff retry.
//!
//! Feature: cli-production-ready
//! Validates: Requirements 3.1, 3.4

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};

use super::update::CURRENT_VERSION;

/// Configuration for connection retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of connection attempts
    pub max_attempts: u32,
    /// Initial delay between retries in milliseconds  
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 2000,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_attempts: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
        }
    }

    /// Calculate the delay for a given attempt (0-indexed)
    /// Uses exponential backoff: delay doubles each attempt, capped at max_delay_ms
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
        Duration::from_millis(delay_ms.min(self.max_delay_ms))
    }
}

/// Connection target for a daemon
#[derive(Debug, Clone)]
pub enum DaemonTarget {
    /// Connect to agent daemon (24/7 service)
    Agent,
    /// Connect to project daemon at specified path
    Project(PathBuf),
    /// Connect via Unix socket
    #[cfg(unix)]
    UnixSocket(PathBuf),
    /// Connect via TCP port
    TcpPort(u16),
}

impl DaemonTarget {
    /// Get a human-readable description of the target
    pub fn description(&self) -> String {
        match self {
            DaemonTarget::Agent => "agent daemon".to_string(),
            DaemonTarget::Project(path) => format!("project daemon at {}", path.display()),
            #[cfg(unix)]
            DaemonTarget::UnixSocket(path) => format!("socket {}", path.display()),
            DaemonTarget::TcpPort(port) => format!("port {}", port),
        }
    }
}

/// Result of a connection attempt
pub enum ConnectionResult {
    /// Successfully connected
    Connected(DaemonConnection),
    /// Connection failed but can retry
    Retry(String),
    /// Connection failed, cannot retry (fatal error)
    Fatal(String),
}

impl std::fmt::Debug for ConnectionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionResult::Connected(_) => write!(f, "Connected"),
            ConnectionResult::Retry(msg) => write!(f, "Retry({})", msg),
            ConnectionResult::Fatal(msg) => write!(f, "Fatal({})", msg),
        }
    }
}

/// An active connection to a daemon
pub struct DaemonConnection {
    target: DaemonTarget,
    connected_at: std::time::Instant,
    daemon_version: Option<String>,
}

/// Handshake request sent to daemon
#[derive(Debug, Clone)]
pub struct HandshakeRequest {
    /// Client version
    pub client_version: String,
    /// Protocol version
    pub protocol_version: u32,
}

impl HandshakeRequest {
    /// Create a new handshake request with current version
    pub fn new() -> Self {
        Self {
            client_version: CURRENT_VERSION.to_string(),
            protocol_version: 1,
        }
    }
}

impl Default for HandshakeRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Handshake response from daemon
#[derive(Debug, Clone)]
pub struct HandshakeResponse {
    /// Daemon version
    pub daemon_version: String,
    /// Protocol version
    pub protocol_version: u32,
    /// Whether versions are compatible
    pub compatible: bool,
    /// Minimum required client version (if incompatible)
    pub min_client_version: Option<String>,
    /// Message from daemon
    pub message: Option<String>,
}

/// Handshake error
#[derive(Debug, Clone)]
pub enum HandshakeError {
    /// Protocol versions don't match
    ProtocolMismatch { client: u32, daemon: u32 },
    /// Version incompatibility
    VersionIncompatible {
        client_version: String,
        daemon_version: String,
        min_required: Option<String>,
        message: String,
    },
    /// Connection error during handshake
    ConnectionError(String),
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandshakeError::ProtocolMismatch { client, daemon } => {
                write!(
                    f,
                    "Protocol version mismatch: client={}, daemon={}. Please upgrade both CLI and daemon.",
                    client, daemon
                )
            }
            HandshakeError::VersionIncompatible {
                client_version,
                daemon_version,
                min_required,
                message,
            } => {
                write!(
                    f,
                    "Version incompatibility: CLI v{} is not compatible with daemon v{}. {}",
                    client_version, daemon_version, message
                )?;
                if let Some(min) = min_required {
                    write!(f, " Minimum required version: v{}", min)?;
                }
                write!(f, " Please run `dx self update` to upgrade.")
            }
            HandshakeError::ConnectionError(msg) => {
                write!(f, "Handshake failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for HandshakeError {}

impl DaemonConnection {
    fn new(target: DaemonTarget) -> Self {
        Self {
            target,
            connected_at: std::time::Instant::now(),
            daemon_version: None,
        }
    }

    fn with_version(target: DaemonTarget, daemon_version: String) -> Self {
        Self {
            target,
            connected_at: std::time::Instant::now(),
            daemon_version: Some(daemon_version),
        }
    }

    /// Get the connection target
    pub fn target(&self) -> &DaemonTarget {
        &self.target
    }

    /// Get how long the connection has been active
    pub fn uptime(&self) -> Duration {
        self.connected_at.elapsed()
    }

    /// Get the daemon version (if handshake was performed)
    pub fn daemon_version(&self) -> Option<&str> {
        self.daemon_version.as_deref()
    }

    /// Check if the connection is still alive
    pub fn is_alive(&self) -> bool {
        // TODO: Implement actual health check via heartbeat
        true
    }

    /// Close the connection gracefully
    pub async fn close(self) -> Result<()> {
        // TODO: Send disconnect message
        Ok(())
    }
}

/// Daemon client with retry capabilities
pub struct DaemonClient {
    retry_config: RetryConfig,
}

impl DaemonClient {
    /// Create a new daemon client with default retry configuration
    pub fn new() -> Self {
        Self {
            retry_config: RetryConfig::default(),
        }
    }

    /// Create a new daemon client with custom retry configuration
    pub fn with_retry_config(retry_config: RetryConfig) -> Self {
        Self { retry_config }
    }

    /// Attempt a single connection to the daemon
    async fn try_connect(&self, target: &DaemonTarget) -> ConnectionResult {
        match target {
            DaemonTarget::Agent => {
                // TODO: Implement actual agent connection
                // For now, simulate connection attempt
                ConnectionResult::Connected(DaemonConnection::new(target.clone()))
            }
            DaemonTarget::Project(path) => {
                if !path.exists() {
                    return ConnectionResult::Fatal(format!(
                        "Project path does not exist: {}",
                        path.display()
                    ));
                }
                // TODO: Implement actual project daemon connection
                ConnectionResult::Connected(DaemonConnection::new(target.clone()))
            }
            #[cfg(unix)]
            DaemonTarget::UnixSocket(path) => {
                if !path.exists() {
                    return ConnectionResult::Retry(format!(
                        "Socket not found: {}",
                        path.display()
                    ));
                }
                // TODO: Implement actual Unix socket connection
                ConnectionResult::Connected(DaemonConnection::new(target.clone()))
            }
            DaemonTarget::TcpPort(port) => {
                if *port == 0 {
                    return ConnectionResult::Fatal("Invalid port: 0".to_string());
                }
                // TODO: Implement actual TCP connection
                ConnectionResult::Connected(DaemonConnection::new(target.clone()))
            }
        }
    }

    /// Perform version handshake with daemon
    ///
    /// Validates that CLI and daemon versions are compatible.
    /// Returns error if versions are incompatible - the daemon must be updated.
    ///
    /// Task 3.1: Modify perform_handshake to fail on incompatibility
    async fn perform_handshake(&self, conn: &mut DaemonConnection) -> Result<(), HandshakeError> {
        let request = HandshakeRequest::new();

        // TODO: Send actual handshake request over IPC
        // For now, simulate a compatible response
        let response = HandshakeResponse {
            daemon_version: CURRENT_VERSION.to_string(),
            protocol_version: 1,
            compatible: true,
            min_client_version: None,
            message: None,
        };

        // Check protocol version first
        if response.protocol_version != request.protocol_version {
            return Err(HandshakeError::ProtocolMismatch {
                client: request.protocol_version,
                daemon: response.protocol_version,
            });
        }

        // Task 3.1: Return Err when response.compatible is false
        // Include clear error message with upgrade instructions
        if !response.compatible {
            return Err(HandshakeError::VersionIncompatible {
                client_version: request.client_version,
                daemon_version: response.daemon_version.clone(),
                min_required: response.min_client_version,
                message: response
                    .message
                    .unwrap_or_else(|| "CLI version is not compatible with daemon.".to_string()),
            });
        }

        // Store daemon version in connection
        conn.daemon_version = Some(response.daemon_version);

        Ok(())
    }

    /// Connect to a daemon with automatic retry and exponential backoff
    ///
    /// This method will attempt to connect up to `max_attempts` times, with
    /// exponentially increasing delays between attempts (starting at `initial_delay_ms`
    /// and capping at `max_delay_ms`).
    ///
    /// # Arguments
    /// * `target` - The daemon target to connect to
    ///
    /// # Returns
    /// * `Ok(DaemonConnection)` - Successfully connected
    /// * `Err` - Failed to connect after all retry attempts
    ///
    /// # Example
    /// ```ignore
    /// let client = DaemonClient::new();
    /// let conn = client.connect_with_retry(DaemonTarget::Agent).await?;
    /// ```
    pub async fn connect_with_retry(&self, target: DaemonTarget) -> Result<DaemonConnection> {
        let mut last_error = String::new();

        for attempt in 0..self.retry_config.max_attempts {
            if attempt > 0 {
                let delay = self.retry_config.delay_for_attempt(attempt - 1);
                tracing::debug!(
                    "Retry attempt {} of {} for {}, waiting {:?}",
                    attempt + 1,
                    self.retry_config.max_attempts,
                    target.description(),
                    delay
                );
                tokio::time::sleep(delay).await;
            }

            match self.try_connect(&target).await {
                ConnectionResult::Connected(conn) => {
                    if attempt > 0 {
                        tracing::debug!(
                            "Successfully connected to {} after {} attempts",
                            target.description(),
                            attempt + 1
                        );
                    }
                    return Ok(conn);
                }
                ConnectionResult::Retry(err) => {
                    last_error = err;
                    tracing::debug!("Connection attempt {} failed: {}", attempt + 1, last_error);
                }
                ConnectionResult::Fatal(err) => {
                    return Err(anyhow::anyhow!(
                        "Fatal error connecting to {}: {}",
                        target.description(),
                        err
                    ));
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to connect to {} after {} attempts. Last error: {}. \
            Please ensure the daemon is running (`dx daemon status`) or start it with `dx daemon agent`.",
            target.description(),
            self.retry_config.max_attempts,
            last_error
        ))
    }

    /// Connect to the agent daemon with retry
    pub async fn connect_agent(&self) -> Result<DaemonConnection> {
        self.connect_with_retry(DaemonTarget::Agent)
            .await
            .context("Failed to connect to agent daemon")
    }

    /// Connect to a project daemon with retry
    pub async fn connect_project(&self, project_path: PathBuf) -> Result<DaemonConnection> {
        self.connect_with_retry(DaemonTarget::Project(project_path))
            .await
            .context("Failed to connect to project daemon")
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 2000);
    }

    #[test]
    fn test_delay_for_attempt_exponential() {
        let config = RetryConfig::new(5, 100, 10000);

        // Attempt 0: 100ms
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        // Attempt 1: 200ms
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        // Attempt 2: 400ms
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
        // Attempt 3: 800ms
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(800));
        // Attempt 4: 1600ms
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(1600));
    }

    #[test]
    fn test_delay_capped_at_max() {
        let config = RetryConfig::new(10, 100, 500);

        // Should cap at 500ms
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(500)); // Capped
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(500)); // Still capped
        assert_eq!(config.delay_for_attempt(10), Duration::from_millis(500)); // Still capped
    }

    #[test]
    fn test_daemon_target_description() {
        assert_eq!(DaemonTarget::Agent.description(), "agent daemon");
        assert_eq!(
            DaemonTarget::Project(PathBuf::from("/test")).description(),
            "project daemon at /test"
        );
        assert_eq!(DaemonTarget::TcpPort(8080).description(), "port 8080");
    }

    #[tokio::test]
    async fn test_daemon_client_creation() {
        let client = DaemonClient::new();
        assert_eq!(client.retry_config.max_attempts, 3);

        let custom_config = RetryConfig::new(5, 50, 1000);
        let client = DaemonClient::with_retry_config(custom_config);
        assert_eq!(client.retry_config.max_attempts, 5);
        assert_eq!(client.retry_config.initial_delay_ms, 50);
    }

    #[test]
    fn test_handshake_request_default() {
        let request = HandshakeRequest::new();
        assert_eq!(request.client_version, CURRENT_VERSION);
        assert_eq!(request.protocol_version, 1);
    }

    #[test]
    fn test_handshake_error_display() {
        let err = HandshakeError::ProtocolMismatch {
            client: 1,
            daemon: 2,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Protocol version mismatch"));
        assert!(msg.contains("client=1"));
        assert!(msg.contains("daemon=2"));

        let err = HandshakeError::VersionIncompatible {
            client_version: "1.0.0".to_string(),
            daemon_version: "2.0.0".to_string(),
            min_required: Some("1.5.0".to_string()),
            message: "Upgrade required".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Version incompatibility"));
        assert!(msg.contains("1.0.0"));
        assert!(msg.contains("2.0.0"));
        assert!(msg.contains("dx self update"));
    }

    #[test]
    fn test_daemon_connection_version() {
        let conn = DaemonConnection::new(DaemonTarget::Agent);
        assert!(conn.daemon_version().is_none());

        let conn = DaemonConnection::with_version(DaemonTarget::Agent, "1.2.3".to_string());
        assert_eq!(conn.daemon_version(), Some("1.2.3"));
    }
}
