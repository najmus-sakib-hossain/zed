//! DX Agent Sandbox - Secure execution environments
//!
//! Provides Docker container isolation, native OS sandboxing (Windows Job Objects,
//! Linux seccomp/namespaces, macOS Sandbox profiles), and resource limits.

pub mod config;
pub mod docker;
pub mod manager;
pub mod native;

use serde::{Deserialize, Serialize};

/// Sandbox backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxType {
    /// Docker container isolation
    Docker,
    /// Native OS sandboxing (platform-specific)
    Native,
    /// Auto-detect best available
    Auto,
}

/// Sandbox execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub killed: bool,
}

/// Detect the best available sandbox type
pub fn detect_sandbox() -> SandboxType {
    // Check Docker first
    if docker::DockerSandbox::is_available() {
        return SandboxType::Docker;
    }
    SandboxType::Native
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sandbox() {
        let sandbox_type = detect_sandbox();
        // Should always return something
        assert!(sandbox_type == SandboxType::Docker || sandbox_type == SandboxType::Native);
    }
}
