//! WASM Host Functions
//!
//! Provides host functions that WASM plugins can call back into the host runtime.
//! These functions are injected into the WASM linker during instantiation.
//!
//! # Available Host Functions
//!
//! - `dx_log(level, msg_ptr, msg_len)` — Log a message at the given level
//! - `dx_http_get(url_ptr, url_len) -> status` — Perform an HTTP GET request
//! - `dx_kv_get(key_ptr, key_len, val_ptr, val_len) -> bytes_written` — Read from KV store
//! - `dx_kv_set(key_ptr, key_len, val_ptr, val_len) -> 0|1` — Write to KV store
//! - `dx_kv_delete(key_ptr, key_len) -> 0|1` — Delete from KV store
//! - `dx_env_get(key_ptr, key_len, val_ptr, val_len) -> bytes_written` — Get env var

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Host state shared with WASM plugin instances
#[derive(Clone)]
pub struct HostState {
    /// Simple key-value store
    pub kv_store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Log buffer (captured log messages)
    pub log_buffer: Arc<Mutex<Vec<LogEntry>>>,
    /// HTTP response buffer (last response body)
    pub http_response: Arc<Mutex<Vec<u8>>>,
    /// Output buffer for plugin return data
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    /// Allowed hosts for network access
    pub allowed_hosts: Vec<String>,
    /// Whether network access is enabled
    pub network_enabled: bool,
    /// Memory limit in bytes
    pub memory_limit: usize,
    /// CPU fuel limit
    pub fuel_limit: u64,
}

/// A captured log entry from a plugin
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Log levels for host logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl LogLevel {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::Trace,
            1 => Self::Debug,
            2 => Self::Info,
            3 => Self::Warn,
            4 => Self::Error,
            _ => Self::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

impl Default for HostState {
    fn default() -> Self {
        Self {
            kv_store: Arc::new(Mutex::new(HashMap::new())),
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            http_response: Arc::new(Mutex::new(Vec::new())),
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            allowed_hosts: Vec::new(),
            network_enabled: false,
            memory_limit: 256 * 1024 * 1024, // 256 MB
            fuel_limit: 1_000_000_000,
        }
    }
}

impl HostState {
    /// Create a new host state with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable network access with allowed hosts
    pub fn with_network(mut self, allowed_hosts: Vec<String>) -> Self {
        self.network_enabled = true;
        self.allowed_hosts = allowed_hosts;
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Set fuel limit
    pub fn with_fuel_limit(mut self, limit: u64) -> Self {
        self.fuel_limit = limit;
        self
    }

    /// Get captured log entries
    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.log_buffer.lock().unwrap().clone()
    }

    /// Clear log buffer
    pub fn clear_logs(&self) {
        self.log_buffer.lock().unwrap().clear();
    }

    /// Get KV store entry
    pub fn kv_get(&self, key: &str) -> Option<Vec<u8>> {
        self.kv_store.lock().unwrap().get(key).cloned()
    }

    /// Set KV store entry
    pub fn kv_set(&self, key: &str, value: Vec<u8>) {
        self.kv_store.lock().unwrap().insert(key.to_string(), value);
    }

    /// Delete KV store entry
    pub fn kv_delete(&self, key: &str) -> bool {
        self.kv_store.lock().unwrap().remove(key).is_some()
    }

    /// Log a message (called from host function)
    pub fn log(&self, level: LogLevel, message: String) {
        let entry = LogEntry {
            level,
            message: message.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.log_buffer.lock().unwrap().push(entry);

        match level {
            LogLevel::Trace => tracing::trace!("[plugin] {}", message),
            LogLevel::Debug => tracing::debug!("[plugin] {}", message),
            LogLevel::Info => tracing::info!("[plugin] {}", message),
            LogLevel::Warn => tracing::warn!("[plugin] {}", message),
            LogLevel::Error => tracing::error!("[plugin] {}", message),
        }
    }

    /// Check if a host is allowed for network access
    pub fn is_host_allowed(&self, host: &str) -> bool {
        if !self.network_enabled {
            return false;
        }
        if self.allowed_hosts.is_empty() {
            return true; // Empty = allow all when network enabled
        }
        self.allowed_hosts.iter().any(|h| host.ends_with(h))
    }
}

// ---------------------------------------------------------------------------
// Host function implementations (pure logic, no wasmtime Linker dependency)
// ---------------------------------------------------------------------------

/// Read a string from WASM linear memory bytes at the given offset + length.
/// Returns `None` if out of bounds or invalid UTF-8.
pub fn read_wasm_string(memory: &[u8], ptr: u32, len: u32) -> Option<String> {
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if end > memory.len() {
        return None;
    }
    std::str::from_utf8(&memory[start..end]).ok().map(|s| s.to_string())
}

/// Write bytes into WASM linear memory at the given offset.
/// Returns the number of bytes actually written (capped by `max_len`).
pub fn write_wasm_bytes(memory: &mut [u8], ptr: u32, max_len: u32, data: &[u8]) -> u32 {
    let start = ptr as usize;
    let max = max_len as usize;
    let to_write = data.len().min(max);
    let end = start + to_write;
    if end > memory.len() {
        return 0;
    }
    memory[start..end].copy_from_slice(&data[..to_write]);
    to_write as u32
}

/// Host-side implementation of `dx_log`
pub fn host_log(state: &HostState, level: u32, message: &str) {
    let log_level = LogLevel::from_u32(level);
    state.log(log_level, message.to_string());
}

/// Host-side implementation of `dx_kv_get` — returns the value bytes (or empty)
pub fn host_kv_get(state: &HostState, key: &str) -> Vec<u8> {
    state.kv_get(key).unwrap_or_default()
}

/// Host-side implementation of `dx_kv_set` — returns true on success
pub fn host_kv_set(state: &HostState, key: &str, value: &[u8]) -> bool {
    state.kv_set(key, value.to_vec());
    true
}

/// Host-side implementation of `dx_kv_delete` — returns true if key existed
pub fn host_kv_delete(state: &HostState, key: &str) -> bool {
    state.kv_delete(key)
}

/// Host-side implementation of `dx_env_get` — returns env var value (or empty)
pub fn host_env_get(key: &str) -> Vec<u8> {
    std::env::var(key).unwrap_or_default().into_bytes()
}

/// Host-side implementation of `dx_http_get` — performs a blocking-ish GET.
/// In real usage this would go through the sandbox network policy.
/// Returns `(status_code, body_bytes)`.
pub async fn host_http_get(state: &HostState, url: &str) -> Result<(u16, Vec<u8>), String> {
    if !state.network_enabled {
        return Err("Network access denied".to_string());
    }

    // Parse host for allowlist check
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            if !state.is_host_allowed(host) {
                return Err(format!("Host not allowed: {}", host));
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;

    let status = resp.status().as_u16();
    let body = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();

    // Cache response body
    *state.http_response.lock().unwrap() = body.clone();

    Ok((status, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_state_default() {
        let state = HostState::new();
        assert!(!state.network_enabled);
        assert_eq!(state.memory_limit, 256 * 1024 * 1024);
    }

    #[test]
    fn test_host_state_with_network() {
        let state = HostState::new().with_network(vec!["api.example.com".to_string()]);
        assert!(state.network_enabled);
        assert!(state.is_host_allowed("api.example.com"));
        assert!(!state.is_host_allowed("evil.com"));
    }

    #[test]
    fn test_host_state_network_all_allowed() {
        let state = HostState::new().with_network(vec![]);
        assert!(state.is_host_allowed("anything.com"));
    }

    #[test]
    fn test_kv_store_operations() {
        let state = HostState::new();

        // Set
        assert!(host_kv_set(&state, "key1", b"value1"));

        // Get
        let val = host_kv_get(&state, "key1");
        assert_eq!(val, b"value1");

        // Get missing
        let val = host_kv_get(&state, "missing");
        assert!(val.is_empty());

        // Delete
        assert!(host_kv_delete(&state, "key1"));
        assert!(!host_kv_delete(&state, "key1")); // Already deleted
    }

    #[test]
    fn test_log_capture() {
        let state = HostState::new();

        host_log(&state, 2, "Hello from plugin");
        host_log(&state, 4, "Error occurred");

        let logs = state.get_logs();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].level, LogLevel::Info);
        assert_eq!(logs[0].message, "Hello from plugin");
        assert_eq!(logs[1].level, LogLevel::Error);

        state.clear_logs();
        assert!(state.get_logs().is_empty());
    }

    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::from_u32(0), LogLevel::Trace);
        assert_eq!(LogLevel::from_u32(1), LogLevel::Debug);
        assert_eq!(LogLevel::from_u32(2), LogLevel::Info);
        assert_eq!(LogLevel::from_u32(3), LogLevel::Warn);
        assert_eq!(LogLevel::from_u32(4), LogLevel::Error);
        assert_eq!(LogLevel::from_u32(99), LogLevel::Info); // fallback
    }

    #[test]
    fn test_read_wasm_string() {
        let memory = b"Hello, WASM!";
        assert_eq!(read_wasm_string(memory, 0, 5), Some("Hello".to_string()));
        assert_eq!(read_wasm_string(memory, 7, 5), Some("WASM!".to_string()));
        // Out of bounds
        assert_eq!(read_wasm_string(memory, 0, 100), None);
    }

    #[test]
    fn test_write_wasm_bytes() {
        let mut memory = vec![0u8; 32];
        let written = write_wasm_bytes(&mut memory, 0, 5, b"Hello");
        assert_eq!(written, 5);
        assert_eq!(&memory[0..5], b"Hello");

        // Capped by max_len
        let written = write_wasm_bytes(&mut memory, 10, 3, b"Hello");
        assert_eq!(written, 3);
        assert_eq!(&memory[10..13], b"Hel");
    }

    #[test]
    fn test_env_get() {
        // PATH should exist on most systems
        let val = host_env_get("PATH");
        assert!(!val.is_empty());

        // Nonexistent var
        let val = host_env_get("DX_NONEXISTENT_VAR_12345");
        assert!(val.is_empty());
    }
}
