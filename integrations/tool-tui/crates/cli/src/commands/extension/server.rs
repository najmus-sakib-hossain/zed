//! Extension server for handling VS Code requests

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::protocol::{Protocol, Request, RequestId, Response};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Socket path (Unix) or named pipe (Windows)
    pub socket_path: PathBuf,

    /// Max concurrent requests
    pub max_concurrent: usize,

    /// Request timeout ms
    pub timeout_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        #[cfg(windows)]
        let socket_path = PathBuf::from(r"\\.\pipe\dx-extension");

        #[cfg(not(windows))]
        let socket_path = PathBuf::from("/tmp/dx-extension.sock");

        Self {
            socket_path,
            max_concurrent: 10,
            timeout_ms: 30000,
        }
    }
}

/// Extension server state
pub struct Server {
    config: ServerConfig,
    handlers: HashMap<String, Box<dyn Handler + Send + Sync>>,
}

/// Request handler trait
pub trait Handler {
    fn handle(&self, params: Option<Vec<u8>>) -> Result<Vec<u8>>;
}

impl Server {
    /// Create new server
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            handlers: HashMap::new(),
        }
    }

    /// Register handler for method
    pub fn register<H>(&mut self, method: &str, handler: H)
    where
        H: Handler + Send + Sync + 'static,
    {
        self.handlers.insert(method.to_string(), Box::new(handler));
    }

    /// Start server (blocking)
    pub async fn start(&self) -> Result<()> {
        // Platform-specific server implementation
        #[cfg(windows)]
        {
            self.start_windows().await
        }

        #[cfg(not(windows))]
        {
            self.start_unix().await
        }
    }

    #[cfg(windows)]
    async fn start_windows(&self) -> Result<()> {
        use std::io::{Read, Write};

        // TODO: Implement Windows named pipe server
        // For now, fall back to TCP
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:9123")?;
        println!("Extension server listening on 127.0.0.1:9123");

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    // Read message
                    let mut buffer = vec![0u8; 65536];
                    let n = stream.read(&mut buffer)?;
                    buffer.truncate(n);

                    // Process message
                    if let Some((message, _)) = Protocol::decode(&buffer) {
                        let response = self.process_message(&message)?;
                        let encoded = Protocol::encode(&response);
                        stream.write_all(&encoded)?;
                    }
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    async fn start_unix(&self) -> Result<()> {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixListener;

        // Remove existing socket
        let _ = std::fs::remove_file(&self.config.socket_path);

        let listener = UnixListener::bind(&self.config.socket_path)?;
        println!("Extension server listening on {:?}", self.config.socket_path);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    // Read message
                    let mut buffer = vec![0u8; 65536];
                    let n = stream.read(&mut buffer)?;
                    buffer.truncate(n);

                    // Process message
                    if let Some((message, _)) = Protocol::decode(&buffer) {
                        let response = self.process_message(&message)?;
                        let encoded = Protocol::encode(&response);
                        stream.write_all(&encoded)?;
                    }
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }

        Ok(())
    }

    fn process_message(&self, _message: &[u8]) -> Result<Vec<u8>> {
        // TODO: Parse JSON-RPC request and dispatch to handler
        Ok(b"{}".to_vec())
    }
}

/// Check handler
pub struct CheckHandler {
    project_path: PathBuf,
}

impl CheckHandler {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

impl Handler for CheckHandler {
    fn handle(&self, _params: Option<Vec<u8>>) -> Result<Vec<u8>> {
        // TODO: Run checks and return diagnostics
        Ok(b"{}".to_vec())
    }
}

/// Score handler
pub struct ScoreHandler;

impl Handler for ScoreHandler {
    fn handle(&self, _params: Option<Vec<u8>>) -> Result<Vec<u8>> {
        // TODO: Get score and return
        Ok(b"{}".to_vec())
    }
}

/// Watch handler
pub struct WatchHandler {
    watchers: Arc<std::sync::Mutex<HashMap<PathBuf, bool>>>,
}

impl WatchHandler {
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

impl Handler for WatchHandler {
    fn handle(&self, _params: Option<Vec<u8>>) -> Result<Vec<u8>> {
        // TODO: Start/stop watching
        Ok(b"{}".to_vec())
    }
}

/// Client for connecting to extension server
pub struct Client {
    #[cfg(windows)]
    address: String,

    #[cfg(not(windows))]
    socket_path: PathBuf,
}

impl Client {
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            Self {
                address: "127.0.0.1:9123".to_string(),
            }
        }

        #[cfg(not(windows))]
        {
            Self {
                socket_path: PathBuf::from("/tmp/dx-extension.sock"),
            }
        }
    }

    /// Send request and wait for response
    pub fn send(&self, _request: &Request) -> Result<Response> {
        // TODO: Implement client send
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: None,
            error: None,
        })
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.timeout_ms, 30000);
    }
}
