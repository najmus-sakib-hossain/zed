//! Sandbox configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Resource limits
    pub limits: ResourceLimits,

    /// Network configuration
    pub network: NetworkMode,

    /// Mounted volumes (host_path -> container_path)
    pub mounts: HashMap<PathBuf, PathBuf>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Working directory inside sandbox
    pub workdir: PathBuf,

    /// Allow network access
    pub network_enabled: bool,

    /// Timeout for operations (seconds)
    pub timeout_secs: Option<u64>,

    /// Auto-cleanup on exit
    pub auto_cleanup: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            limits: ResourceLimits::default(),
            network: NetworkMode::None,
            mounts: HashMap::new(),
            env: HashMap::new(),
            workdir: PathBuf::from("/workspace"),
            network_enabled: false,
            timeout_secs: Some(300), // 5 minutes default
            auto_cleanup: true,
        }
    }
}

/// Resource limits for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (None = unlimited)
    pub memory_bytes: Option<u64>,

    /// Maximum CPU shares (0-1024, None = unlimited)
    pub cpu_shares: Option<u64>,

    /// Maximum disk space in bytes (None = unlimited)
    pub disk_bytes: Option<u64>,

    /// Maximum number of processes (None = unlimited)
    pub max_pids: Option<u64>,

    /// Maximum open files (None = unlimited)
    pub max_files: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: Some(512 * 1024 * 1024), // 512 MB
            cpu_shares: Some(512),                 // 50% of CPU
            disk_bytes: Some(1024 * 1024 * 1024),  // 1 GB
            max_pids: Some(100),
            max_files: Some(1024),
        }
    }
}

impl ResourceLimits {
    /// Create unlimited resource limits
    pub fn unlimited() -> Self {
        Self {
            memory_bytes: None,
            cpu_shares: None,
            disk_bytes: None,
            max_pids: None,
            max_files: None,
        }
    }

    /// Create strict resource limits for untrusted code
    pub fn strict() -> Self {
        Self {
            memory_bytes: Some(128 * 1024 * 1024), // 128 MB
            cpu_shares: Some(256),                 // 25% of CPU
            disk_bytes: Some(256 * 1024 * 1024),   // 256 MB
            max_pids: Some(50),
            max_files: Some(256),
        }
    }
}

/// Network isolation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    /// No network access
    None,

    /// Host network (no isolation)
    Host,

    /// Bridge network (isolated)
    Bridge,

    /// Custom network
    Custom,
}

impl std::fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMode::None => write!(f, "none"),
            NetworkMode::Host => write!(f, "host"),
            NetworkMode::Bridge => write!(f, "bridge"),
            NetworkMode::Custom => write!(f, "custom"),
        }
    }
}
