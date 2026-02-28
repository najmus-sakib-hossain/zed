//! Sandbox backend abstraction

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::config::SandboxConfig;

/// Sandbox backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxBackendType {
    /// Docker container isolation
    Docker,

    /// Podman container isolation
    Podman,

    /// Native OS sandboxing (platform-specific)
    Native,

    /// WebAssembly runtime isolation
    Wasm,

    /// Auto-detect best available backend
    Auto,
}

impl std::fmt::Display for SandboxBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxBackendType::Docker => write!(f, "docker"),
            SandboxBackendType::Podman => write!(f, "podman"),
            SandboxBackendType::Native => write!(f, "native"),
            SandboxBackendType::Wasm => write!(f, "wasm"),
            SandboxBackendType::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for SandboxBackendType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(SandboxBackendType::Docker),
            "podman" => Ok(SandboxBackendType::Podman),
            "native" => Ok(SandboxBackendType::Native),
            "wasm" => Ok(SandboxBackendType::Wasm),
            "auto" => Ok(SandboxBackendType::Auto),
            _ => Err(anyhow::anyhow!("Unknown backend type: {}", s)),
        }
    }
}

/// Sandbox execution result
#[derive(Debug)]
pub struct SandboxResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Trait for sandbox backend implementations
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    /// Initialize the sandbox environment
    async fn create(&mut self, config: &SandboxConfig) -> Result<()>;

    /// Execute a command in the sandbox
    async fn execute(&self, command: &[String]) -> Result<SandboxResult>;

    /// Copy files into the sandbox
    async fn copy_in(&self, host_path: &Path, sandbox_path: &Path) -> Result<()>;

    /// Copy files out of the sandbox
    async fn copy_out(&self, sandbox_path: &Path, host_path: &Path) -> Result<()>;

    /// Destroy the sandbox and cleanup resources
    async fn destroy(&mut self) -> Result<()>;

    /// Check if the backend is available on this system
    fn is_available() -> bool
    where
        Self: Sized;

    /// Get backend type
    fn backend_type(&self) -> SandboxBackendType;
}

/// Detect the best available sandbox backend
pub fn detect_backend() -> SandboxBackendType {
    // Prefer native for simplicity and no external dependencies
    // Native works on all platforms without installation
    SandboxBackendType::Native
}
