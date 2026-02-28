//! Sandbox configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Docker image to use
    #[serde(default = "default_image")]
    pub image: String,

    /// Mounted volumes (host_path -> container_path)
    #[serde(default)]
    pub volumes: HashMap<String, String>,

    /// Working directory inside sandbox
    #[serde(default = "default_workdir")]
    pub workdir: String,

    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Resource limits
    #[serde(default)]
    pub limits: ResourceLimits,

    /// Network access
    #[serde(default)]
    pub network: NetworkConfig,

    /// Auto-cleanup on exit
    #[serde(default = "default_true")]
    pub auto_cleanup: bool,

    /// Maximum execution time in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Memory limit in MB
    #[serde(default = "default_memory")]
    pub memory_mb: u64,

    /// CPU limit (number of cores, e.g., 1.0 = 1 core)
    #[serde(default = "default_cpu")]
    pub cpu_cores: f64,

    /// Disk limit in MB
    #[serde(default = "default_disk")]
    pub disk_mb: u64,

    /// Max number of processes
    #[serde(default = "default_pids")]
    pub max_pids: u64,
}

/// Network configuration for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable network access
    #[serde(default)]
    pub enabled: bool,

    /// Allowed domains (if network enabled)
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Custom DNS servers
    #[serde(default)]
    pub dns: Vec<String>,
}

fn default_image() -> String {
    "ubuntu:24.04".into()
}
fn default_workdir() -> String {
    "/workspace".into()
}
fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    300
}
fn default_memory() -> u64 {
    512
}
fn default_cpu() -> f64 {
    1.0
}
fn default_disk() -> u64 {
    1024
}
fn default_pids() -> u64 {
    100
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: default_image(),
            volumes: HashMap::new(),
            workdir: default_workdir(),
            env: HashMap::new(),
            limits: ResourceLimits::default(),
            network: NetworkConfig::default(),
            auto_cleanup: true,
            timeout_secs: default_timeout(),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: default_memory(),
            cpu_cores: default_cpu(),
            disk_mb: default_disk(),
            max_pids: default_pids(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: Vec::new(),
            dns: Vec::new(),
        }
    }
}

impl SandboxConfig {
    /// Add a volume mount
    pub fn with_volume(mut self, host: impl Into<String>, container: impl Into<String>) -> Self {
        self.volumes.insert(host.into(), container.into());
        self
    }

    /// Set workspace directory mount
    pub fn with_workspace(self, workspace_path: &PathBuf) -> Self {
        self.with_volume(workspace_path.to_string_lossy().to_string(), "/workspace".to_string())
    }

    /// Enable network with allowed domains
    pub fn with_network(mut self, domains: Vec<String>) -> Self {
        self.network.enabled = true;
        self.network.allowed_domains = domains;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SandboxConfig::default();
        assert_eq!(config.image, "ubuntu:24.04");
        assert_eq!(config.limits.memory_mb, 512);
        assert!(!config.network.enabled);
        assert!(config.auto_cleanup);
    }

    #[test]
    fn test_builder_pattern() {
        let config = SandboxConfig::default()
            .with_volume("/home/user/project", "/workspace")
            .with_network(vec!["api.openai.com".into()]);

        assert!(config.volumes.contains_key("/home/user/project"));
        assert!(config.network.enabled);
        assert_eq!(config.network.allowed_domains.len(), 1);
    }
}
