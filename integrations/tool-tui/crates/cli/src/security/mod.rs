//! # Production Security Module
//!
//! Comprehensive security system for DX production deployments.
//! Includes permission management, WASM sandboxing, secrets management,
//! and audit logging with cryptographic integrity.

pub mod audit;
pub mod permissions;
pub mod sandbox;
pub mod secrets;

use std::collections::HashMap;
use thiserror::Error;

/// Security errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Invalid trust level: {0}")]
    InvalidTrustLevel(String),
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Audit failure: {0}")]
    AuditFailure(String),
    #[error("Hash chain corrupted: {0}")]
    HashChainCorrupted(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Trust level for permission system
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrustLevel {
    /// No permissions - sandbox only
    Untrusted = 0,
    /// Basic file read in project directory
    Basic = 1,
    /// File read/write in project directory
    Standard = 2,
    /// Network access, limited system calls
    Extended = 3,
    /// Full system access (requires explicit user approval)
    Full = 4,
}

impl TrustLevel {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<TrustLevel> {
        match s.to_lowercase().as_str() {
            "untrusted" | "none" | "0" => Some(TrustLevel::Untrusted),
            "basic" | "read" | "1" => Some(TrustLevel::Basic),
            "standard" | "readwrite" | "2" => Some(TrustLevel::Standard),
            "extended" | "network" | "3" => Some(TrustLevel::Extended),
            "full" | "admin" | "4" => Some(TrustLevel::Full),
            _ => None,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            TrustLevel::Untrusted => "No permissions, sandboxed execution only",
            TrustLevel::Basic => "Read-only access to project files",
            TrustLevel::Standard => "Read/write access to project directory",
            TrustLevel::Extended => "Network and limited system access",
            TrustLevel::Full => "Full system access (requires approval)",
        }
    }

    /// Get allowed capabilities
    pub fn capabilities(&self) -> Vec<Capability> {
        match self {
            TrustLevel::Untrusted => vec![],
            TrustLevel::Basic => vec![Capability::FileRead],
            TrustLevel::Standard => vec![
                Capability::FileRead,
                Capability::FileWrite,
                Capability::ProcessSpawn,
            ],
            TrustLevel::Extended => vec![
                Capability::FileRead,
                Capability::FileWrite,
                Capability::ProcessSpawn,
                Capability::NetworkConnect,
                Capability::NetworkListen,
            ],
            TrustLevel::Full => vec![
                Capability::FileRead,
                Capability::FileWrite,
                Capability::ProcessSpawn,
                Capability::NetworkConnect,
                Capability::NetworkListen,
                Capability::SystemCall,
                Capability::EnvAccess,
            ],
        }
    }
}

/// Individual capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Read files
    FileRead,
    /// Write files
    FileWrite,
    /// Spawn processes
    ProcessSpawn,
    /// Outbound network connections
    NetworkConnect,
    /// Listen on ports
    NetworkListen,
    /// Raw system calls
    SystemCall,
    /// Environment variable access
    EnvAccess,
    /// Memory allocation beyond limit
    MemoryExtended,
}

impl Capability {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Capability::FileRead => "file:read",
            Capability::FileWrite => "file:write",
            Capability::ProcessSpawn => "process:spawn",
            Capability::NetworkConnect => "network:connect",
            Capability::NetworkListen => "network:listen",
            Capability::SystemCall => "system:call",
            Capability::EnvAccess => "env:access",
            Capability::MemoryExtended => "memory:extended",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Capability> {
        match s {
            "file:read" => Some(Capability::FileRead),
            "file:write" => Some(Capability::FileWrite),
            "process:spawn" => Some(Capability::ProcessSpawn),
            "network:connect" => Some(Capability::NetworkConnect),
            "network:listen" => Some(Capability::NetworkListen),
            "system:call" => Some(Capability::SystemCall),
            "env:access" => Some(Capability::EnvAccess),
            "memory:extended" => Some(Capability::MemoryExtended),
            _ => None,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Default trust level
    pub default_trust: TrustLevel,
    /// Maximum memory per WASM instance (bytes)
    pub max_memory: usize,
    /// Maximum execution time (ms)
    pub max_execution_time: u64,
    /// Allowed network hosts
    pub allowed_hosts: Vec<String>,
    /// Blocked file patterns
    pub blocked_paths: Vec<String>,
    /// Enable audit logging
    pub audit_enabled: bool,
    /// Audit log path
    pub audit_path: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_trust: TrustLevel::Standard,
            max_memory: 256 * 1024 * 1024, // 256 MB
            max_execution_time: 30_000,    // 30 seconds
            allowed_hosts: vec!["localhost".into(), "127.0.0.1".into()],
            blocked_paths: vec![
                "~/.ssh".into(),
                "~/.aws".into(),
                "~/.gnupg".into(),
                "/etc/passwd".into(),
                "/etc/shadow".into(),
            ],
            audit_enabled: true,
            audit_path: Some(".dx/audit.log".into()),
        }
    }
}

/// Security manager
pub struct SecurityManager {
    /// Configuration
    config: SecurityConfig,
    /// Active permissions per context
    permissions: HashMap<String, TrustLevel>,
    /// Audit logger
    audit: Option<audit::AuditLogger>,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: SecurityConfig) -> Self {
        let audit = if config.audit_enabled {
            config.audit_path.as_ref().and_then(|path| audit::AuditLogger::new(path).ok())
        } else {
            None
        };

        Self {
            config,
            permissions: HashMap::new(),
            audit,
        }
    }

    /// Set trust level for context
    pub fn set_trust(&mut self, context: &str, level: TrustLevel) {
        self.permissions.insert(context.to_string(), level);
        if let Some(audit) = &mut self.audit {
            let _ = audit.log_event(audit::AuditEvent {
                timestamp: chrono::Utc::now(),
                event_type: audit::EventType::TrustChange,
                context: context.to_string(),
                details: format!("Trust level set to {:?}", level),
                success: true,
            });
        }
    }

    /// Get trust level for context
    pub fn get_trust(&self, context: &str) -> TrustLevel {
        self.permissions.get(context).copied().unwrap_or(self.config.default_trust)
    }

    /// Check if capability is allowed
    pub fn check_capability(&self, context: &str, capability: Capability) -> bool {
        let level = self.get_trust(context);
        level.capabilities().contains(&capability)
    }

    /// Request capability (with audit logging)
    pub fn request_capability(
        &mut self,
        context: &str,
        capability: Capability,
    ) -> Result<(), SecurityError> {
        let allowed = self.check_capability(context, capability);

        if let Some(audit) = &mut self.audit {
            let _ = audit.log_event(audit::AuditEvent {
                timestamp: chrono::Utc::now(),
                event_type: audit::EventType::CapabilityRequest,
                context: context.to_string(),
                details: format!("Requested capability: {}", capability.name()),
                success: allowed,
            });
        }

        if allowed {
            Ok(())
        } else {
            Err(SecurityError::PermissionDenied(format!(
                "Capability '{}' not allowed for trust level {:?}",
                capability.name(),
                self.get_trust(context)
            )))
        }
    }

    /// Verify path access
    pub fn verify_path(&self, context: &str, path: &str, write: bool) -> Result<(), SecurityError> {
        let required = if write {
            Capability::FileWrite
        } else {
            Capability::FileRead
        };

        if !self.check_capability(context, required) {
            return Err(SecurityError::PermissionDenied(format!(
                "File {} not allowed",
                if write { "write" } else { "read" }
            )));
        }

        // Check blocked paths
        for blocked in &self.config.blocked_paths {
            if path.contains(blocked) || path.starts_with(blocked) {
                return Err(SecurityError::PermissionDenied(format!(
                    "Access to '{}' is blocked",
                    path
                )));
            }
        }

        Ok(())
    }

    /// Verify network access
    pub fn verify_network(
        &self,
        context: &str,
        host: &str,
        listen: bool,
    ) -> Result<(), SecurityError> {
        let required = if listen {
            Capability::NetworkListen
        } else {
            Capability::NetworkConnect
        };

        if !self.check_capability(context, required) {
            return Err(SecurityError::PermissionDenied(format!(
                "Network {} not allowed",
                if listen { "listen" } else { "connect" }
            )));
        }

        // Check allowed hosts (only for extended trust, not full)
        let level = self.get_trust(context);
        if level < TrustLevel::Full {
            let allowed = self
                .config
                .allowed_hosts
                .iter()
                .any(|h| host == h || host.ends_with(&format!(".{}", h)));
            if !allowed {
                return Err(SecurityError::PermissionDenied(format!(
                    "Host '{}' not in allowed list",
                    host
                )));
            }
        }

        Ok(())
    }

    /// Get security statistics
    pub fn stats(&self) -> SecurityStats {
        SecurityStats {
            active_contexts: self.permissions.len(),
            audit_enabled: self.audit.is_some(),
            default_trust: self.config.default_trust,
            max_memory: self.config.max_memory,
        }
    }
}

/// Security statistics
#[derive(Debug)]
pub struct SecurityStats {
    pub active_contexts: usize,
    pub audit_enabled: bool,
    pub default_trust: TrustLevel,
    pub max_memory: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_levels() {
        assert!(TrustLevel::Full > TrustLevel::Extended);
        assert!(TrustLevel::Extended > TrustLevel::Standard);
        assert!(TrustLevel::Standard > TrustLevel::Basic);
        assert!(TrustLevel::Basic > TrustLevel::Untrusted);
    }

    #[test]
    fn test_capabilities() {
        let caps = TrustLevel::Standard.capabilities();
        assert!(caps.contains(&Capability::FileRead));
        assert!(caps.contains(&Capability::FileWrite));
        assert!(!caps.contains(&Capability::NetworkConnect));
    }

    #[test]
    fn test_security_manager() {
        let mut manager = SecurityManager::new(SecurityConfig::default());

        manager.set_trust("test", TrustLevel::Basic);
        assert_eq!(manager.get_trust("test"), TrustLevel::Basic);
        assert!(manager.check_capability("test", Capability::FileRead));
        assert!(!manager.check_capability("test", Capability::FileWrite));
    }

    #[test]
    fn test_blocked_paths() {
        let manager = SecurityManager::new(SecurityConfig::default());

        assert!(manager.verify_path("test", "~/.ssh/id_rsa", false).is_err());
        assert!(manager.verify_path("test", "/etc/passwd", false).is_err());
    }
}
