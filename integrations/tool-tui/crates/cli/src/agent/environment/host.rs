//! WASM Host Interface
//!
//! Provides host functions that WASM plugins can call.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{EnvironmentError, EnvironmentResult};

/// Host capabilities that can be granted to WASM plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HostCapability {
    /// HTTP request capability
    Http = 0,
    /// WebSocket capability
    WebSocket = 1,
    /// Key-Value storage
    KeyValue = 2,
    /// Logging capability
    Logging = 3,
    /// File system read
    FileRead = 4,
    /// File system write
    FileWrite = 5,
    /// Environment variables
    Environment = 6,
    /// Clock/time access
    Clock = 7,
    /// Random number generation
    Random = 8,
    /// Process spawning
    Process = 9,
}

impl HostCapability {
    /// Get all capabilities
    pub const fn all() -> &'static [HostCapability] {
        &[
            HostCapability::Http,
            HostCapability::WebSocket,
            HostCapability::KeyValue,
            HostCapability::Logging,
            HostCapability::FileRead,
            HostCapability::FileWrite,
            HostCapability::Environment,
            HostCapability::Clock,
            HostCapability::Random,
            HostCapability::Process,
        ]
    }

    /// Safe capabilities that can be granted by default
    pub const fn safe_defaults() -> &'static [HostCapability] {
        &[
            HostCapability::Logging,
            HostCapability::Clock,
            HostCapability::Random,
        ]
    }
}

impl std::fmt::Display for HostCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostCapability::Http => write!(f, "http"),
            HostCapability::WebSocket => write!(f, "websocket"),
            HostCapability::KeyValue => write!(f, "kv"),
            HostCapability::Logging => write!(f, "logging"),
            HostCapability::FileRead => write!(f, "fs:read"),
            HostCapability::FileWrite => write!(f, "fs:write"),
            HostCapability::Environment => write!(f, "env"),
            HostCapability::Clock => write!(f, "clock"),
            HostCapability::Random => write!(f, "random"),
            HostCapability::Process => write!(f, "process"),
        }
    }
}

/// Configuration for the host
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// Allowed capabilities
    pub capabilities: Vec<HostCapability>,
    /// Allowed HTTP hosts (empty = all)
    pub allowed_hosts: Vec<String>,
    /// Key-value storage path
    pub kv_path: PathBuf,
    /// Allowed file system paths
    pub allowed_paths: Vec<PathBuf>,
    /// Maximum HTTP request size
    pub max_http_request_size: usize,
    /// Maximum HTTP response size
    pub max_http_response_size: usize,
    /// HTTP timeout in seconds
    pub http_timeout_secs: u64,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            capabilities: HostCapability::safe_defaults().to_vec(),
            allowed_hosts: Vec::new(),
            kv_path: PathBuf::from(".dx/kv"),
            allowed_paths: Vec::new(),
            max_http_request_size: 10 * 1024 * 1024,   // 10MB
            max_http_response_size: 100 * 1024 * 1024, // 100MB
            http_timeout_secs: 30,
        }
    }
}

/// HTTP request from WASM
#[derive(Debug, Clone)]
pub struct WasmHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

/// HTTP response to WASM
#[derive(Debug, Clone)]
pub struct WasmHttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// WebSocket message
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

/// Key-Value entry
#[derive(Debug, Clone)]
pub struct KvEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub expires_at: Option<u64>,
}

/// Log level for WASM logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

/// Trait for DX host functionality
pub trait DxHost: Send + Sync {
    /// Check if a capability is allowed
    fn has_capability(&self, cap: HostCapability) -> bool;

    /// Make an HTTP request
    fn http_request(
        &self,
        request: WasmHttpRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = EnvironmentResult<WasmHttpResponse>> + Send + '_>,
    >;

    /// Open a WebSocket connection
    fn ws_connect(
        &self,
        url: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EnvironmentResult<u64>> + Send + '_>>;

    /// Send WebSocket message
    fn ws_send(
        &self,
        conn_id: u64,
        message: WsMessage,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EnvironmentResult<()>> + Send + '_>>;

    /// Receive WebSocket message
    fn ws_recv(
        &self,
        conn_id: u64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = EnvironmentResult<WsMessage>> + Send + '_>,
    >;

    /// Close WebSocket connection
    fn ws_close(
        &self,
        conn_id: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EnvironmentResult<()>> + Send + '_>>;

    /// Get value from KV store
    fn kv_get(&self, key: &[u8]) -> EnvironmentResult<Option<Vec<u8>>>;

    /// Set value in KV store
    fn kv_set(&self, key: &[u8], value: &[u8], ttl_secs: Option<u64>) -> EnvironmentResult<()>;

    /// Delete value from KV store
    fn kv_delete(&self, key: &[u8]) -> EnvironmentResult<bool>;

    /// List keys with prefix
    fn kv_list(&self, prefix: &[u8]) -> EnvironmentResult<Vec<Vec<u8>>>;

    /// Log a message
    fn log(&self, level: LogLevel, message: &str);

    /// Get current time in milliseconds
    fn now_millis(&self) -> u64;

    /// Generate random bytes
    fn random_bytes(&self, len: usize) -> Vec<u8>;

    /// Read environment variable
    fn env_get(&self, name: &str) -> Option<String>;

    /// Read file (if permitted)
    fn file_read(&self, path: &str) -> EnvironmentResult<Vec<u8>>;

    /// Write file (if permitted)
    fn file_write(&self, path: &str, content: &[u8]) -> EnvironmentResult<()>;
}

/// Default implementation of DxHost
pub struct DefaultDxHost {
    config: HostConfig,
    kv_store: Arc<RwLock<HashMap<Vec<u8>, KvEntry>>>,
    ws_connections: Arc<RwLock<HashMap<u64, ()>>>, // Placeholder for actual WS
    next_ws_id: Arc<std::sync::atomic::AtomicU64>,
}

impl DefaultDxHost {
    /// Create a new default host
    pub fn new(config: HostConfig) -> Self {
        Self {
            config,
            kv_store: Arc::new(RwLock::new(HashMap::new())),
            ws_connections: Arc::new(RwLock::new(HashMap::new())),
            next_ws_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Create with safe defaults
    pub fn with_safe_defaults() -> Self {
        Self::new(HostConfig::default())
    }

    /// Check if host is allowed for HTTP
    fn is_host_allowed(&self, url: &str) -> bool {
        if self.config.allowed_hosts.is_empty() {
            return true;
        }

        let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(|s| s.to_string()));

        match host {
            Some(h) => self
                .config
                .allowed_hosts
                .iter()
                .any(|allowed| h == *allowed || h.ends_with(&format!(".{}", allowed))),
            None => false,
        }
    }

    /// Check if path is allowed for file operations
    fn is_path_allowed(&self, path: &str) -> bool {
        if self.config.allowed_paths.is_empty() {
            return false;
        }

        let path = PathBuf::from(path);
        let canonical = path.canonicalize().unwrap_or(path);

        self.config.allowed_paths.iter().any(|allowed| canonical.starts_with(allowed))
    }
}

impl DxHost for DefaultDxHost {
    fn has_capability(&self, cap: HostCapability) -> bool {
        self.config.capabilities.contains(&cap)
    }

    fn http_request(
        &self,
        request: WasmHttpRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = EnvironmentResult<WasmHttpResponse>> + Send + '_>,
    > {
        Box::pin(async move {
            if !self.has_capability(HostCapability::Http) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: "http".into(),
                });
            }

            if !self.is_host_allowed(&request.url) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: format!("http to {}", request.url),
                });
            }

            // Check request size
            if let Some(ref body) = request.body {
                if body.len() > self.config.max_http_request_size {
                    return Err(EnvironmentError::CapabilityDenied {
                        capability: format!(
                            "http request body exceeds {} bytes",
                            self.config.max_http_request_size
                        ),
                    });
                }
            }

            // Build reqwest request
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(self.config.http_timeout_secs))
                .build()
                .map_err(|e| EnvironmentError::IpcError {
                    message: e.to_string(),
                })?;

            let method = request.method.parse::<reqwest::Method>().map_err(|e| {
                EnvironmentError::IpcError {
                    message: format!("Invalid HTTP method: {}", e),
                }
            })?;

            let mut req_builder = client.request(method, &request.url);

            for (key, value) in &request.headers {
                req_builder = req_builder.header(key, value);
            }

            if let Some(body) = request.body {
                req_builder = req_builder.body(body);
            }

            let response = req_builder.send().await.map_err(|e| EnvironmentError::IpcError {
                message: e.to_string(),
            })?;

            let status = response.status().as_u16();
            let headers: HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let body = response.bytes().await.map_err(|e| EnvironmentError::IpcError {
                message: e.to_string(),
            })?;

            if body.len() > self.config.max_http_response_size {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: format!(
                        "http response body exceeds {} bytes",
                        self.config.max_http_response_size
                    ),
                });
            }

            Ok(WasmHttpResponse {
                status,
                headers,
                body: body.to_vec(),
            })
        })
    }

    fn ws_connect(
        &self,
        url: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EnvironmentResult<u64>> + Send + '_>>
    {
        let url = url.to_string();
        Box::pin(async move {
            if !self.has_capability(HostCapability::WebSocket) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: "websocket".into(),
                });
            }

            if !self.is_host_allowed(&url) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: format!("websocket to {}", url),
                });
            }

            let id = self.next_ws_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            // TODO: Actually establish WebSocket connection
            // For now, just track the connection ID
            let mut conns = self.ws_connections.write().await;
            conns.insert(id, ());

            Ok(id)
        })
    }

    fn ws_send(
        &self,
        conn_id: u64,
        _message: WsMessage,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EnvironmentResult<()>> + Send + '_>>
    {
        Box::pin(async move {
            if !self.has_capability(HostCapability::WebSocket) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: "websocket".into(),
                });
            }

            let conns = self.ws_connections.read().await;
            if !conns.contains_key(&conn_id) {
                return Err(EnvironmentError::IpcError {
                    message: format!("WebSocket connection {} not found", conn_id),
                });
            }

            // TODO: Actually send message
            Ok(())
        })
    }

    fn ws_recv(
        &self,
        conn_id: u64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = EnvironmentResult<WsMessage>> + Send + '_>,
    > {
        Box::pin(async move {
            if !self.has_capability(HostCapability::WebSocket) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: "websocket".into(),
                });
            }

            let conns = self.ws_connections.read().await;
            if !conns.contains_key(&conn_id) {
                return Err(EnvironmentError::IpcError {
                    message: format!("WebSocket connection {} not found", conn_id),
                });
            }

            // TODO: Actually receive message
            Ok(WsMessage::Close)
        })
    }

    fn ws_close(
        &self,
        conn_id: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = EnvironmentResult<()>> + Send + '_>>
    {
        Box::pin(async move {
            if !self.has_capability(HostCapability::WebSocket) {
                return Err(EnvironmentError::CapabilityDenied {
                    capability: "websocket".into(),
                });
            }

            let mut conns = self.ws_connections.write().await;
            conns.remove(&conn_id);

            Ok(())
        })
    }

    fn kv_get(&self, key: &[u8]) -> EnvironmentResult<Option<Vec<u8>>> {
        if !self.has_capability(HostCapability::KeyValue) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: "kv".into(),
            });
        }

        let store = self.kv_store.blocking_read();

        match store.get(key) {
            Some(entry) => {
                // Check expiration
                if let Some(expires) = entry.expires_at {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    if now > expires {
                        return Ok(None);
                    }
                }
                Ok(Some(entry.value.clone()))
            }
            None => Ok(None),
        }
    }

    fn kv_set(&self, key: &[u8], value: &[u8], ttl_secs: Option<u64>) -> EnvironmentResult<()> {
        if !self.has_capability(HostCapability::KeyValue) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: "kv".into(),
            });
        }

        let expires_at = ttl_secs.map(|ttl| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + ttl
        });

        let mut store = self.kv_store.blocking_write();
        store.insert(
            key.to_vec(),
            KvEntry {
                key: key.to_vec(),
                value: value.to_vec(),
                expires_at,
            },
        );

        Ok(())
    }

    fn kv_delete(&self, key: &[u8]) -> EnvironmentResult<bool> {
        if !self.has_capability(HostCapability::KeyValue) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: "kv".into(),
            });
        }

        let mut store = self.kv_store.blocking_write();
        Ok(store.remove(key).is_some())
    }

    fn kv_list(&self, prefix: &[u8]) -> EnvironmentResult<Vec<Vec<u8>>> {
        if !self.has_capability(HostCapability::KeyValue) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: "kv".into(),
            });
        }

        let store = self.kv_store.blocking_read();
        let keys: Vec<Vec<u8>> = store.keys().filter(|k| k.starts_with(prefix)).cloned().collect();

        Ok(keys)
    }

    fn log(&self, level: LogLevel, message: &str) {
        if !self.has_capability(HostCapability::Logging) {
            return;
        }

        let level_str = match level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };

        eprintln!("[WASM {}] {}", level_str, message);
    }

    fn now_millis(&self) -> u64 {
        if !self.has_capability(HostCapability::Clock) {
            return 0;
        }

        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn random_bytes(&self, len: usize) -> Vec<u8> {
        if !self.has_capability(HostCapability::Random) {
            return vec![0; len];
        }

        use rand::RngCore;
        let mut bytes = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    }

    fn env_get(&self, name: &str) -> Option<String> {
        if !self.has_capability(HostCapability::Environment) {
            return None;
        }

        std::env::var(name).ok()
    }

    fn file_read(&self, path: &str) -> EnvironmentResult<Vec<u8>> {
        if !self.has_capability(HostCapability::FileRead) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: "fs:read".into(),
            });
        }

        if !self.is_path_allowed(path) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: format!("fs:read {}", path),
            });
        }

        std::fs::read(path).map_err(EnvironmentError::from)
    }

    fn file_write(&self, path: &str, content: &[u8]) -> EnvironmentResult<()> {
        if !self.has_capability(HostCapability::FileWrite) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: "fs:write".into(),
            });
        }

        if !self.is_path_allowed(path) {
            return Err(EnvironmentError::CapabilityDenied {
                capability: format!("fs:write {}", path),
            });
        }

        std::fs::write(path, content).map_err(EnvironmentError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_defaults() {
        let defaults = HostCapability::safe_defaults();
        assert!(defaults.contains(&HostCapability::Logging));
        assert!(defaults.contains(&HostCapability::Clock));
        assert!(defaults.contains(&HostCapability::Random));
        assert!(!defaults.contains(&HostCapability::Http));
    }

    #[test]
    fn test_host_creation() {
        let host = DefaultDxHost::with_safe_defaults();
        assert!(host.has_capability(HostCapability::Logging));
        assert!(!host.has_capability(HostCapability::Http));
    }

    #[test]
    fn test_kv_operations() {
        let mut config = HostConfig::default();
        config.capabilities.push(HostCapability::KeyValue);
        let host = DefaultDxHost::new(config);

        // Set
        host.kv_set(b"test_key", b"test_value", None).unwrap();

        // Get
        let value = host.kv_get(b"test_key").unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));

        // Delete
        let deleted = host.kv_delete(b"test_key").unwrap();
        assert!(deleted);

        // Get after delete
        let value = host.kv_get(b"test_key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_random_bytes() {
        let host = DefaultDxHost::with_safe_defaults();
        let bytes = host.random_bytes(32);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_now_millis() {
        let host = DefaultDxHost::with_safe_defaults();
        let now = host.now_millis();
        assert!(now > 0);
    }
}
