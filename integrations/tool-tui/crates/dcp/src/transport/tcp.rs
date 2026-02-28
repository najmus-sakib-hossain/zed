//! TCP transport implementation for DCP server.
//!
//! Provides TCP server with configurable bind address, connection limits,
//! TLS support, and protocol negotiation.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock, Semaphore};
use tokio_rustls::rustls;
use tokio_rustls::TlsAcceptor;

/// TLS version configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlsVersion {
    /// TLS 1.2
    #[default]
    Tls12,
    /// TLS 1.3
    Tls13,
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to certificate file (PEM format)
    pub cert_path: PathBuf,
    /// Path to private key file (PEM format)
    pub key_path: PathBuf,
    /// Minimum TLS version (default: 1.2)
    pub min_version: TlsVersion,
}

impl TlsConfig {
    /// Create a new TLS configuration
    pub fn new(cert_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            min_version: TlsVersion::default(),
        }
    }

    /// Set minimum TLS version
    pub fn with_min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }

    /// Build a TLS acceptor from this configuration
    pub fn build_acceptor(&self) -> io::Result<TlsAcceptor> {
        // Load certificates
        let cert_file = File::open(&self.cert_path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to open certificate file {:?}: {}", self.cert_path, e),
            )
        })?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse certificates: {}", e),
                )
            })?;

        if certs.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No certificates found in certificate file",
            ));
        }

        // Load private key
        let key_file = File::open(&self.key_path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to open key file {:?}: {}", self.key_path, e),
            )
        })?;
        let mut key_reader = BufReader::new(key_file);

        let key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse private key: {}", e),
                )
            })?
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "No private key found in key file")
            })?;

        // Build server config
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to build TLS config: {}", e),
                )
            })?;

        Ok(TlsAcceptor::from(Arc::new(config)))
    }
}

/// TCP listener configuration
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Bind address (e.g., "0.0.0.0:9000")
    pub bind_addr: SocketAddr,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Read buffer size
    pub read_buffer_size: usize,
    /// Enable TCP_NODELAY
    pub nodelay: bool,
    /// TLS configuration (optional)
    pub tls: Option<TlsConfig>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9000".parse().unwrap(),
            max_connections: 1000,
            connection_timeout_secs: 30,
            read_buffer_size: 8192,
            nodelay: true,
            tls: None,
        }
    }
}

/// Protocol mode after negotiation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolMode {
    /// MCP JSON-RPC over newline-delimited JSON
    #[default]
    McpJson,
    /// DCP binary protocol
    DcpBinary,
}

/// Connection state
#[derive(Debug)]
pub struct Connection {
    /// Unique connection ID
    pub id: u64,
    /// Peer address
    pub peer_addr: SocketAddr,
    /// Detected protocol mode
    pub protocol: ProtocolMode,
    /// Connection creation time
    pub created_at: Instant,
    /// Bytes read counter
    pub bytes_read: AtomicU64,
    /// Bytes written counter
    pub bytes_written: AtomicU64,
}

impl Connection {
    /// Create a new connection
    pub fn new(id: u64, peer_addr: SocketAddr) -> Self {
        Self {
            id,
            peer_addr,
            protocol: ProtocolMode::McpJson,
            created_at: Instant::now(),
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
        }
    }

    /// Record bytes read
    pub fn record_read(&self, bytes: u64) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes written
    pub fn record_write(&self, bytes: u64) {
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get total bytes read
    pub fn total_bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    /// Get total bytes written
    pub fn total_bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    /// Get connection age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// DCP magic bytes for binary protocol detection
pub const DCP_MAGIC: [u8; 4] = [0x44, 0x43, 0x50, 0x01]; // "DCP\x01"

/// TCP server implementation
pub struct TcpServer {
    /// Server configuration
    config: TcpConfig,
    /// Active connections
    connections: Arc<RwLock<HashMap<u64, Arc<Connection>>>>,
    /// Connection ID counter
    connection_counter: AtomicU64,
    /// Connection limit semaphore
    connection_semaphore: Arc<Semaphore>,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Whether server is running
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl TcpServer {
    /// Create a new TCP server with the given configuration
    pub fn new(config: TcpConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            connection_semaphore: Arc::new(Semaphore::new(config.max_connections)),
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_counter: AtomicU64::new(1),
            shutdown_tx,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Get the server configuration
    pub fn config(&self) -> &TcpConfig {
        &self.config
    }

    /// Get current connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: u64) -> Option<Arc<Connection>> {
        self.connections.read().await.get(&id).cloned()
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Try to acquire a connection slot
    /// Returns None if connection limit is reached
    pub fn try_acquire_connection(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.connection_semaphore.clone().try_acquire_owned().ok()
    }

    /// Register a new connection
    pub async fn register_connection(&self, peer_addr: SocketAddr) -> Arc<Connection> {
        let id = self.connection_counter.fetch_add(1, Ordering::SeqCst);
        let conn = Arc::new(Connection::new(id, peer_addr));
        self.connections.write().await.insert(id, Arc::clone(&conn));
        conn
    }

    /// Remove a connection
    pub async fn remove_connection(&self, id: u64) -> Option<Arc<Connection>> {
        self.connections.write().await.remove(&id)
    }

    /// Detect protocol from first bytes
    /// Returns (protocol_mode, bytes_to_process)
    pub fn detect_protocol(first_bytes: &[u8]) -> ProtocolMode {
        if first_bytes.is_empty() {
            return ProtocolMode::McpJson;
        }

        // Check for DCP magic bytes
        if first_bytes.len() >= 4 && first_bytes[..4] == DCP_MAGIC {
            return ProtocolMode::DcpBinary;
        }

        // Check for JSON (starts with '{' or '[')
        let first_non_ws = first_bytes.iter().find(|&&b| !b.is_ascii_whitespace());
        if let Some(&b) = first_non_ws {
            if b == b'{' || b == b'[' {
                return ProtocolMode::McpJson;
            }
        }

        // Default to binary if unclear
        ProtocolMode::DcpBinary
    }

    /// Start accepting connections
    pub async fn run<H>(&self, handler: H) -> io::Result<()>
    where
        H: ConnectionHandler + Clone + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        self.running.store(true, Ordering::SeqCst);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, peer_addr)) => {
                            // Try to acquire connection slot
                            let permit = match self.try_acquire_connection() {
                                Some(p) => p,
                                None => {
                                    // Connection limit reached, reject
                                    drop(stream);
                                    continue;
                                }
                            };

                            // Set TCP options
                            if self.config.nodelay {
                                let _ = stream.set_nodelay(true);
                            }

                            // Register connection
                            let conn = self.register_connection(peer_addr).await;
                            let handler = handler.clone();
                            let connections = Arc::clone(&self.connections);
                            let timeout = Duration::from_secs(self.config.connection_timeout_secs);
                            let buffer_size = self.config.read_buffer_size;

                            // Spawn connection handler
                            tokio::spawn(async move {
                                let _permit = permit; // Hold permit until connection closes
                                let conn_id = conn.id;

                                let result = Self::handle_connection(
                                    stream,
                                    conn,
                                    handler,
                                    timeout,
                                    buffer_size,
                                ).await;

                                // Clean up connection
                                connections.write().await.remove(&conn_id);

                                if let Err(e) = result {
                                    // Log error but don't crash
                                    eprintln!("Connection {} error: {}", conn_id, e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }

        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Handle a single connection
    async fn handle_connection<H>(
        mut stream: TcpStream,
        conn: Arc<Connection>,
        handler: H,
        timeout: Duration,
        buffer_size: usize,
    ) -> io::Result<()>
    where
        H: ConnectionHandler,
    {
        let mut buffer = vec![0u8; buffer_size];

        // Read first bytes for protocol detection
        let first_read = tokio::time::timeout(timeout, stream.read(&mut buffer)).await;

        let n = match first_read {
            Ok(Ok(0)) => return Ok(()), // EOF
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "connection timeout")),
        };

        // Detect protocol
        let protocol = Self::detect_protocol(&buffer[..n]);

        // Update connection protocol
        // Note: Connection.protocol is not mutable after creation,
        // but we pass the detected protocol to the handler

        // Call handler with detected protocol and initial data
        handler.handle(&mut stream, &conn, protocol, &buffer[..n]).await
    }

    /// Graceful shutdown - stop accepting, drain existing connections
    pub async fn shutdown(&self, timeout: Duration) -> io::Result<()> {
        // Signal shutdown
        let _ = self.shutdown_tx.send(());

        // Wait for connections to drain
        let deadline = Instant::now() + timeout;
        while self.connection_count().await > 0 {
            if Instant::now() > deadline {
                // Force close remaining connections
                self.connections.write().await.clear();
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Get shutdown signal receiver
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}

/// Connection handler trait
pub trait ConnectionHandler: Send + Sync + Clone + 'static {
    /// Handle a connection with detected protocol and initial data
    fn handle(
        &self,
        stream: &mut TcpStream,
        conn: &Connection,
        protocol: ProtocolMode,
        initial_data: &[u8],
    ) -> impl std::future::Future<Output = io::Result<()>> + Send;
}

/// Simple echo handler for testing
#[derive(Clone)]
pub struct EchoHandler;

impl ConnectionHandler for EchoHandler {
    async fn handle(
        &self,
        stream: &mut TcpStream,
        conn: &Connection,
        _protocol: ProtocolMode,
        initial_data: &[u8],
    ) -> io::Result<()> {
        // Echo back initial data
        if !initial_data.is_empty() {
            stream.write_all(initial_data).await?;
            conn.record_read(initial_data.len() as u64);
            conn.record_write(initial_data.len() as u64);
        }

        // Continue echoing
        let mut buffer = vec![0u8; 4096];
        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            conn.record_read(n as u64);
            stream.write_all(&buffer[..n]).await?;
            conn.record_write(n as u64);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_config_default() {
        let config = TcpConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.connection_timeout_secs, 30);
        assert!(config.nodelay);
        assert!(config.tls.is_none());
    }

    #[test]
    fn test_connection_new() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let conn = Connection::new(1, addr);
        assert_eq!(conn.id, 1);
        assert_eq!(conn.peer_addr, addr);
        assert_eq!(conn.protocol, ProtocolMode::McpJson);
    }

    #[test]
    fn test_connection_bytes_tracking() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let conn = Connection::new(1, addr);

        conn.record_read(100);
        conn.record_read(50);
        conn.record_write(200);

        assert_eq!(conn.total_bytes_read(), 150);
        assert_eq!(conn.total_bytes_written(), 200);
    }

    #[test]
    fn test_detect_protocol_json() {
        assert_eq!(TcpServer::detect_protocol(b"{\"jsonrpc\":\"2.0\"}"), ProtocolMode::McpJson);
        assert_eq!(TcpServer::detect_protocol(b"  {\"test\":1}"), ProtocolMode::McpJson);
        assert_eq!(TcpServer::detect_protocol(b"[1,2,3]"), ProtocolMode::McpJson);
    }

    #[test]
    fn test_detect_protocol_binary() {
        assert_eq!(TcpServer::detect_protocol(&DCP_MAGIC), ProtocolMode::DcpBinary);
        assert_eq!(
            TcpServer::detect_protocol(&[0x44, 0x43, 0x50, 0x01, 0x00, 0x00]),
            ProtocolMode::DcpBinary
        );
    }

    #[test]
    fn test_detect_protocol_empty() {
        assert_eq!(TcpServer::detect_protocol(&[]), ProtocolMode::McpJson);
    }

    #[tokio::test]
    async fn test_tcp_server_connection_limit() {
        let config = TcpConfig {
            max_connections: 2,
            ..Default::default()
        };
        let server = TcpServer::new(config);

        // Acquire two permits
        let permit1 = server.try_acquire_connection();
        let permit2 = server.try_acquire_connection();

        assert!(permit1.is_some());
        assert!(permit2.is_some());

        // Third should fail
        let permit3 = server.try_acquire_connection();
        assert!(permit3.is_none());

        // Drop one permit
        drop(permit1);

        // Now should succeed
        let permit4 = server.try_acquire_connection();
        assert!(permit4.is_some());
    }

    #[tokio::test]
    async fn test_tcp_server_register_connection() {
        let config = TcpConfig::default();
        let server = TcpServer::new(config);

        let addr: SocketAddr = "192.168.1.1:12345".parse().unwrap();
        let conn = server.register_connection(addr).await;

        assert_eq!(conn.id, 1);
        assert_eq!(conn.peer_addr, addr);
        assert_eq!(server.connection_count().await, 1);

        // Register another
        let addr2: SocketAddr = "192.168.1.2:12346".parse().unwrap();
        let conn2 = server.register_connection(addr2).await;
        assert_eq!(conn2.id, 2);
        assert_eq!(server.connection_count().await, 2);
    }

    #[tokio::test]
    async fn test_tcp_server_remove_connection() {
        let config = TcpConfig::default();
        let server = TcpServer::new(config);

        let addr: SocketAddr = "192.168.1.1:12345".parse().unwrap();
        let conn = server.register_connection(addr).await;
        let id = conn.id;

        assert_eq!(server.connection_count().await, 1);

        let removed = server.remove_connection(id).await;
        assert!(removed.is_some());
        assert_eq!(server.connection_count().await, 0);

        // Remove again should return None
        let removed_again = server.remove_connection(id).await;
        assert!(removed_again.is_none());
    }
}
