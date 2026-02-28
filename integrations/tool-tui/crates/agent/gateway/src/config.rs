//! Gateway configuration system (TOML-based).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,
    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    /// Maximum WebSocket message size in bytes
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Heartbeat interval in seconds
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
    /// Enable CORS
    #[serde(default = "default_true")]
    pub cors_enabled: bool,
    /// CORS allowed origins (empty = all)
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Require authentication for connections
    #[serde(default)]
    pub required: bool,
    /// JWT secret key (auto-generated if not set)
    #[serde(default)]
    pub jwt_secret: Option<String>,
    /// Token expiry in seconds
    #[serde(default = "default_token_expiry")]
    pub token_expiry_secs: u64,
    /// API keys for static auth
    #[serde(default)]
    pub api_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Max requests per window
    #[serde(default = "default_max_requests")]
    pub max_requests: usize,
    /// Time window in seconds
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log to file
    #[serde(default)]
    pub file: Option<PathBuf>,
    /// Log rotation: max file size in MB
    #[serde(default = "default_max_log_size")]
    pub max_size_mb: u64,
    /// Max number of rotated log files
    #[serde(default = "default_max_log_files")]
    pub max_files: usize,
    /// JSON format logging
    #[serde(default)]
    pub json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite database path
    #[serde(default = "default_db_path")]
    pub path: PathBuf,
}

// --- Defaults ---

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    31337
}
fn default_max_message_size() -> usize {
    1_048_576
}
fn default_max_connections() -> usize {
    10_000
}
fn default_heartbeat_interval() -> u64 {
    30
}
fn default_connection_timeout() -> u64 {
    300
}
fn default_true() -> bool {
    true
}
fn default_token_expiry() -> u64 {
    86400
}
fn default_max_requests() -> usize {
    100
}
fn default_window_secs() -> u64 {
    60
}
fn default_log_level() -> String {
    "info".into()
}
fn default_max_log_size() -> u64 {
    10
}
fn default_max_log_files() -> usize {
    5
}
fn default_db_path() -> PathBuf {
    dirs_data_path().join("dx-gateway.db")
}

fn dirs_data_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".into())).join("dx")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
            .join("Library/Application Support/dx")
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config/dx")
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfig::default(),
            logging: LoggingConfig::default(),
            database: DatabaseConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            max_message_size: default_max_message_size(),
            max_connections: default_max_connections(),
            heartbeat_interval_secs: default_heartbeat_interval(),
            connection_timeout_secs: default_connection_timeout(),
            cors_enabled: true,
            cors_origins: vec![],
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            required: false,
            jwt_secret: None,
            token_expiry_secs: default_token_expiry(),
            api_keys: vec![],
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_requests: default_max_requests(),
            window_secs: default_window_secs(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            max_size_mb: default_max_log_size(),
            max_files: default_max_log_files(),
            json: false,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

impl GatewayConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load from default config path, or return defaults
    pub fn load_or_default() -> Self {
        let config_path = dirs_data_path().join("gateway.toml");
        if config_path.exists() {
            Self::load(&config_path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the bind address as "host:port"
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default();
        assert_eq!(config.server.port, 31337);
        assert_eq!(config.server.host, "127.0.0.1");
        assert!(config.rate_limit.enabled);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = GatewayConfig::default();
        let toml = toml::to_string_pretty(&config).expect("serialize");
        let parsed: GatewayConfig = toml::from_str(&toml).expect("deserialize");
        assert_eq!(parsed.server.port, config.server.port);
    }

    #[test]
    fn test_bind_address() {
        let config = GatewayConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:31337");
    }
}
