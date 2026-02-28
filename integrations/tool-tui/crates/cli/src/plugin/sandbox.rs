//! Plugin Sandbox
//!
//! Provides capability-based sandboxing for plugins.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::traits::Capability;

/// Sandbox configuration for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum memory usage (bytes)
    pub memory_limit: usize,
    /// Maximum CPU time per execution (ms)
    pub cpu_limit_ms: u64,
    /// Maximum execution time (wall clock)
    pub timeout: Duration,
    /// Allowed filesystem paths for read
    pub fs_read_paths: Vec<PathBuf>,
    /// Allowed filesystem paths for write
    pub fs_write_paths: Vec<PathBuf>,
    /// Network access policy
    pub network_policy: NetworkPolicy,
    /// Allowed capabilities
    pub capabilities: HashSet<Capability>,
    /// Enable detailed logging
    pub enable_logging: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit: 256 * 1024 * 1024,  // 256 MB
            cpu_limit_ms: 30_000,             // 30 seconds
            timeout: Duration::from_secs(60), // 1 minute wall clock
            fs_read_paths: vec![],
            fs_write_paths: vec![],
            network_policy: NetworkPolicy::Allowed,
            capabilities: HashSet::new(),
            enable_logging: false,
        }
    }
}

impl SandboxConfig {
    /// Create a restrictive sandbox config
    pub fn restrictive() -> Self {
        Self {
            memory_limit: 64 * 1024 * 1024, // 64 MB
            cpu_limit_ms: 5_000,            // 5 seconds
            timeout: Duration::from_secs(10),
            fs_read_paths: vec![],
            fs_write_paths: vec![],
            network_policy: NetworkPolicy::Denied,
            capabilities: HashSet::new(),
            enable_logging: true,
        }
    }

    /// Create a permissive sandbox config (dangerous!)
    pub fn permissive() -> Self {
        Self {
            memory_limit: 1024 * 1024 * 1024, // 1 GB
            cpu_limit_ms: 300_000,            // 5 minutes
            timeout: Duration::from_secs(600),
            fs_read_paths: vec![PathBuf::from("/")],
            fs_write_paths: vec![],
            network_policy: NetworkPolicy::Allowed,
            capabilities: [
                Capability::Network,
                Capability::FileRead,
                Capability::Environment,
            ]
            .into_iter()
            .collect(),
            enable_logging: false,
        }
    }

    /// Add a capability
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.insert(cap);
        self
    }

    /// Add filesystem read path
    pub fn with_fs_read(mut self, path: PathBuf) -> Self {
        self.fs_read_paths.push(path);
        self.capabilities.insert(Capability::FileRead);
        self
    }

    /// Add filesystem write path
    pub fn with_fs_write(mut self, path: PathBuf) -> Self {
        self.fs_write_paths.push(path);
        self.capabilities.insert(Capability::FileWrite);
        self
    }

    /// Set network policy
    pub fn with_network(mut self, policy: NetworkPolicy) -> Self {
        if matches!(policy, NetworkPolicy::Allowed | NetworkPolicy::AllowedHosts(_)) {
            self.capabilities.insert(Capability::Network);
        }
        self.network_policy = policy;
        self
    }

    /// Check if a capability is allowed
    pub fn allows(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap) || self.capabilities.contains(&Capability::System)
    }

    /// Check if a path is readable
    pub fn can_read(&self, path: &PathBuf) -> bool {
        if !self.capabilities.contains(&Capability::FileRead) {
            return false;
        }
        self.fs_read_paths.iter().any(|allowed| path.starts_with(allowed))
    }

    /// Check if a path is writable
    pub fn can_write(&self, path: &PathBuf) -> bool {
        if !self.capabilities.contains(&Capability::FileWrite) {
            return false;
        }
        self.fs_write_paths.iter().any(|allowed| path.starts_with(allowed))
    }
}

/// Network access policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkPolicy {
    /// Network access denied
    Denied,
    /// Network access allowed
    Allowed,
    /// Only specific hosts allowed
    AllowedHosts(Vec<String>),
    /// Only specific ports allowed
    AllowedPorts(Vec<u16>),
}

impl NetworkPolicy {
    /// Check if a host is allowed
    pub fn allows_host(&self, host: &str) -> bool {
        match self {
            Self::Denied => false,
            Self::Allowed => true,
            Self::AllowedHosts(hosts) => hosts.iter().any(|h| h == host || host.ends_with(h)),
            Self::AllowedPorts(_) => true, // Port filtering done separately
        }
    }

    /// Check if a port is allowed
    pub fn allows_port(&self, port: u16) -> bool {
        match self {
            Self::Denied => false,
            Self::Allowed => true,
            Self::AllowedHosts(_) => true, // Host filtering done separately
            Self::AllowedPorts(ports) => ports.contains(&port),
        }
    }
}

/// Plugin sandbox for enforcing resource limits
pub struct PluginSandbox {
    config: SandboxConfig,
    /// Audit log of sandbox events
    audit_log: Vec<SandboxEvent>,
}

/// Sandbox audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxEvent {
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type
    pub event_type: SandboxEventType,
    /// Event details
    pub details: String,
    /// Was the event allowed?
    pub allowed: bool,
}

/// Types of sandbox events
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SandboxEventType {
    /// Capability check
    CapabilityCheck,
    /// Filesystem access
    FileAccess,
    /// Network access
    NetworkAccess,
    /// Memory allocation
    MemoryAllocation,
    /// Timeout
    Timeout,
}

impl PluginSandbox {
    /// Create a new sandbox with config
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            audit_log: Vec::new(),
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Check and log a capability request
    pub fn check_capability(&mut self, cap: Capability) -> Result<()> {
        let allowed = self.config.allows(cap);

        if self.config.enable_logging {
            self.audit_log.push(SandboxEvent {
                timestamp: chrono::Utc::now(),
                event_type: SandboxEventType::CapabilityCheck,
                details: format!("Capability: {:?}", cap),
                allowed,
            });
        }

        if allowed {
            Ok(())
        } else {
            anyhow::bail!("Capability denied: {:?}", cap)
        }
    }

    /// Check and log a file access
    pub fn check_file_access(&mut self, path: &PathBuf, write: bool) -> Result<()> {
        let allowed = if write {
            self.config.can_write(path)
        } else {
            self.config.can_read(path)
        };

        if self.config.enable_logging {
            self.audit_log.push(SandboxEvent {
                timestamp: chrono::Utc::now(),
                event_type: SandboxEventType::FileAccess,
                details: format!("Path: {}, Write: {}", path.display(), write),
                allowed,
            });
        }

        if allowed {
            Ok(())
        } else {
            anyhow::bail!("File access denied: {} (write: {})", path.display(), write)
        }
    }

    /// Check and log network access
    pub fn check_network_access(&mut self, host: &str, port: u16) -> Result<()> {
        let allowed = self.config.network_policy.allows_host(host)
            && self.config.network_policy.allows_port(port);

        if self.config.enable_logging {
            self.audit_log.push(SandboxEvent {
                timestamp: chrono::Utc::now(),
                event_type: SandboxEventType::NetworkAccess,
                details: format!("Host: {}, Port: {}", host, port),
                allowed,
            });
        }

        if allowed {
            Ok(())
        } else {
            anyhow::bail!("Network access denied: {}:{}", host, port)
        }
    }

    /// Get the audit log
    pub fn audit_log(&self) -> &[SandboxEvent] {
        &self.audit_log
    }

    /// Clear the audit log
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.memory_limit, 256 * 1024 * 1024);
        assert_eq!(config.cpu_limit_ms, 30_000);
    }

    #[test]
    fn test_sandbox_config_restrictive() {
        let config = SandboxConfig::restrictive();
        assert_eq!(config.memory_limit, 64 * 1024 * 1024);
        assert!(config.enable_logging);
    }

    #[test]
    fn test_network_policy_allows() {
        let policy = NetworkPolicy::AllowedHosts(vec!["api.example.com".to_string()]);
        assert!(policy.allows_host("api.example.com"));
        assert!(!policy.allows_host("evil.com"));
    }

    #[test]
    fn test_sandbox_capability_check() {
        let config = SandboxConfig::default().with_capability(Capability::Network);
        let mut sandbox = PluginSandbox::new(config);

        assert!(sandbox.check_capability(Capability::Network).is_ok());
        assert!(sandbox.check_capability(Capability::Shell).is_err());
    }
}
