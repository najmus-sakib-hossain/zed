//! Rule Sandbox
//!
//! Isolated execution environment for rule processing.

use super::{Capability, CapabilityManifest};
use crate::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Allowed read paths
    pub read_paths: HashSet<PathBuf>,
    /// Allowed write paths
    pub write_paths: HashSet<PathBuf>,
    /// Allowed environment variables
    pub env_vars: HashSet<String>,
    /// Maximum memory (bytes)
    pub max_memory: usize,
    /// Maximum execution time (ms)
    pub max_time_ms: u64,
    /// Allow network access
    pub allow_network: bool,
    /// Allow process spawning
    pub allow_spawn: bool,
}

impl SandboxConfig {
    /// Create a restrictive sandbox
    pub fn restricted() -> Self {
        Self {
            read_paths: HashSet::new(),
            write_paths: HashSet::new(),
            env_vars: HashSet::new(),
            max_memory: 64 * 1024 * 1024, // 64MB
            max_time_ms: 5000,            // 5 seconds
            allow_network: false,
            allow_spawn: false,
        }
    }

    /// Create a permissive sandbox
    pub fn permissive() -> Self {
        Self {
            read_paths: HashSet::new(), // Empty = allow all
            write_paths: HashSet::new(),
            env_vars: HashSet::new(),
            max_memory: 512 * 1024 * 1024, // 512MB
            max_time_ms: 30000,            // 30 seconds
            allow_network: true,
            allow_spawn: true,
        }
    }

    /// Allow reading from a path
    pub fn allow_read(mut self, path: impl Into<PathBuf>) -> Self {
        self.read_paths.insert(path.into());
        self
    }

    /// Allow writing to a path
    pub fn allow_write(mut self, path: impl Into<PathBuf>) -> Self {
        self.write_paths.insert(path.into());
        self
    }

    /// Allow access to environment variable
    pub fn allow_env(mut self, var: impl Into<String>) -> Self {
        self.env_vars.insert(var.into());
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, bytes: usize) -> Self {
        self.max_memory = bytes;
        self
    }

    /// Set time limit
    pub fn with_time_limit(mut self, ms: u64) -> Self {
        self.max_time_ms = ms;
        self
    }

    /// Convert to capability manifest
    pub fn to_manifest(&self) -> CapabilityManifest {
        let mut manifest = CapabilityManifest::new();

        if !self.read_paths.is_empty() {
            manifest.require(Capability::FileRead);
        }

        if !self.write_paths.is_empty() {
            manifest.require(Capability::FileWrite);
        }

        if !self.env_vars.is_empty() {
            manifest.require(Capability::Environment);
        }

        if self.allow_network {
            manifest.require(Capability::Network);
        } else {
            manifest.deny(Capability::Network);
        }

        if self.allow_spawn {
            manifest.require(Capability::Process);
        } else {
            manifest.deny(Capability::Process);
        }

        manifest
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::restricted()
    }
}

/// Sandbox for isolated rule execution
#[derive(Debug)]
pub struct Sandbox {
    /// Configuration
    config: SandboxConfig,
    /// Execution started
    started: Option<std::time::Instant>,
    /// Memory used
    memory_used: usize,
    /// Violations detected
    violations: Vec<SandboxViolation>,
}

/// Sandbox violation
#[derive(Debug, Clone)]
pub struct SandboxViolation {
    /// Violation type
    pub kind: ViolationKind,
    /// Description
    pub message: String,
    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// Violation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// Unauthorized file read
    UnauthorizedRead,
    /// Unauthorized file write
    UnauthorizedWrite,
    /// Memory limit exceeded
    MemoryExceeded,
    /// Time limit exceeded
    TimeExceeded,
    /// Network access denied
    NetworkDenied,
    /// Process spawn denied
    SpawnDenied,
    /// Environment access denied
    EnvDenied,
}

impl Sandbox {
    /// Create a new sandbox
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            started: None,
            memory_used: 0,
            violations: Vec::new(),
        }
    }

    /// Start sandbox execution
    pub fn start(&mut self) {
        self.started = Some(std::time::Instant::now());
        self.memory_used = 0;
        self.violations.clear();
    }

    /// Check if execution is within time limit
    pub fn check_time(&mut self) -> Result<bool> {
        if let Some(started) = self.started {
            if started.elapsed().as_millis() as u64 > self.config.max_time_ms {
                self.record_violation(ViolationKind::TimeExceeded, "Time limit exceeded");
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check if memory allocation is allowed
    pub fn check_memory(&mut self, bytes: usize) -> Result<bool> {
        if self.memory_used + bytes > self.config.max_memory {
            self.record_violation(ViolationKind::MemoryExceeded, "Memory limit exceeded");
            return Ok(false);
        }
        self.memory_used += bytes;
        Ok(true)
    }

    /// Check if file read is allowed
    pub fn check_read(&mut self, path: &Path) -> bool {
        // If no specific paths, deny all
        if self.config.read_paths.is_empty() {
            self.record_violation(
                ViolationKind::UnauthorizedRead,
                &format!("Read denied: {}", path.display()),
            );
            return false;
        }

        // Check if path is under any allowed path
        let allowed = self
            .config
            .read_paths
            .iter()
            .any(|allowed| path.starts_with(allowed) || path == allowed);

        if !allowed {
            self.record_violation(
                ViolationKind::UnauthorizedRead,
                &format!("Read denied: {}", path.display()),
            );
        }

        allowed
    }

    /// Check if file write is allowed
    pub fn check_write(&mut self, path: &Path) -> bool {
        if self.config.write_paths.is_empty() {
            self.record_violation(
                ViolationKind::UnauthorizedWrite,
                &format!("Write denied: {}", path.display()),
            );
            return false;
        }

        let allowed = self
            .config
            .write_paths
            .iter()
            .any(|allowed| path.starts_with(allowed) || path == allowed);

        if !allowed {
            self.record_violation(
                ViolationKind::UnauthorizedWrite,
                &format!("Write denied: {}", path.display()),
            );
        }

        allowed
    }

    /// Check if environment variable access is allowed
    pub fn check_env(&mut self, var: &str) -> bool {
        if self.config.env_vars.is_empty() {
            self.record_violation(ViolationKind::EnvDenied, &format!("Env access denied: {}", var));
            return false;
        }

        let allowed = self.config.env_vars.contains(var);

        if !allowed {
            self.record_violation(ViolationKind::EnvDenied, &format!("Env access denied: {}", var));
        }

        allowed
    }

    /// Check if network access is allowed
    pub fn check_network(&mut self) -> bool {
        if !self.config.allow_network {
            self.record_violation(ViolationKind::NetworkDenied, "Network access denied");
            return false;
        }
        true
    }

    /// Check if process spawning is allowed
    pub fn check_spawn(&mut self) -> bool {
        if !self.config.allow_spawn {
            self.record_violation(ViolationKind::SpawnDenied, "Process spawn denied");
            return false;
        }
        true
    }

    /// Record a violation
    fn record_violation(&mut self, kind: ViolationKind, message: &str) {
        self.violations.push(SandboxViolation {
            kind,
            message: message.to_string(),
            timestamp: std::time::Instant::now(),
        });
    }

    /// Get all violations
    pub fn violations(&self) -> &[SandboxViolation] {
        &self.violations
    }

    /// Check if any violations occurred
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Get memory used
    pub fn memory_used(&self) -> usize {
        self.memory_used
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.started.map(|s| s.elapsed())
    }

    /// Reset sandbox state
    pub fn reset(&mut self) {
        self.started = None;
        self.memory_used = 0;
        self.violations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config() {
        let config = SandboxConfig::restricted()
            .allow_read("/home/user")
            .allow_write("/tmp")
            .with_memory_limit(128 * 1024 * 1024);

        assert!(config.read_paths.contains(Path::new("/home/user")));
        assert!(config.write_paths.contains(Path::new("/tmp")));
        assert_eq!(config.max_memory, 128 * 1024 * 1024);
    }

    #[test]
    fn test_sandbox_file_access() {
        let config = SandboxConfig::restricted().allow_read("/home/user/project");

        let mut sandbox = Sandbox::new(config);
        sandbox.start();

        assert!(sandbox.check_read(Path::new("/home/user/project/src/main.rs")));
        assert!(!sandbox.check_read(Path::new("/etc/passwd")));
        assert!(sandbox.has_violations());
    }

    #[test]
    fn test_sandbox_memory() {
        let config = SandboxConfig::restricted().with_memory_limit(1024);

        let mut sandbox = Sandbox::new(config);
        sandbox.start();

        assert!(sandbox.check_memory(512).unwrap());
        assert!(sandbox.check_memory(512).unwrap());
        assert!(!sandbox.check_memory(1).unwrap()); // Over limit
    }
}
