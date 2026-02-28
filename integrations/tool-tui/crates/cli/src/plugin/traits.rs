//! Plugin Trait Definitions
//!
//! Core traits and types for the DX plugin system.

use std::collections::HashSet;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::PluginType;

/// Plugin capability (permissions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Network access (HTTP, WebSocket)
    Network,
    /// Filesystem read access
    FileRead,
    /// Filesystem write access
    FileWrite,
    /// Execute shell commands
    Shell,
    /// Access environment variables
    Environment,
    /// Access system clipboard
    Clipboard,
    /// Access system notifications
    Notifications,
    /// Access camera/microphone
    Media,
    /// Access location services
    Location,
    /// Access browser automation
    Browser,
    /// Access LLM APIs
    Llm,
    /// Full system access (dangerous!)
    System,
}

impl Capability {
    /// Parse capability from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "network" | "net" => Self::Network,
            "file_read" | "fileread" | "fs_read" => Self::FileRead,
            "file_write" | "filewrite" | "fs_write" => Self::FileWrite,
            "shell" | "exec" | "process" => Self::Shell,
            "environment" | "env" => Self::Environment,
            "clipboard" => Self::Clipboard,
            "notifications" | "notify" => Self::Notifications,
            "media" | "camera" | "microphone" => Self::Media,
            "location" | "gps" => Self::Location,
            "browser" => Self::Browser,
            "llm" | "ai" => Self::Llm,
            "system" | "full" => Self::System,
            _ => Self::Network, // Default to limited network access
        }
    }

    /// Get capability name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::FileRead => "file_read",
            Self::FileWrite => "file_write",
            Self::Shell => "shell",
            Self::Environment => "environment",
            Self::Clipboard => "clipboard",
            Self::Notifications => "notifications",
            Self::Media => "media",
            Self::Location => "location",
            Self::Browser => "browser",
            Self::Llm => "llm",
            Self::System => "system",
        }
    }

    /// Check if this is a dangerous capability
    pub fn is_dangerous(&self) -> bool {
        matches!(self, Self::Shell | Self::System | Self::FileWrite)
    }
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
    /// Required capabilities
    pub capabilities: Vec<Capability>,
    /// Plugin type (WASM or Native)
    pub plugin_type: PluginType,
    /// Path to plugin file
    pub path: PathBuf,
    /// Ed25519 signature (for native plugins)
    pub signature: Option<String>,
}

impl PluginMetadata {
    /// Check if plugin requires a specific capability
    pub fn requires(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Check if plugin has any dangerous capabilities
    pub fn has_dangerous_capabilities(&self) -> bool {
        self.capabilities.iter().any(|c| c.is_dangerous())
    }
}

/// Plugin execution context
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Working directory
    pub working_dir: PathBuf,
    /// Environment variables
    pub env: std::collections::HashMap<String, String>,
    /// Granted capabilities
    pub capabilities: HashSet<Capability>,
    /// Maximum memory (bytes)
    pub memory_limit: usize,
    /// Maximum CPU time (ms)
    pub cpu_limit_ms: u64,
    /// Plugin arguments
    pub args: Vec<String>,
}

impl Default for PluginContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            env: std::collections::HashMap::new(),
            capabilities: HashSet::new(),
            memory_limit: 256 * 1024 * 1024, // 256 MB
            cpu_limit_ms: 30_000,            // 30 seconds
            args: Vec::new(),
        }
    }
}

impl PluginContext {
    /// Create new context with capabilities
    pub fn with_capabilities(mut self, caps: impl IntoIterator<Item = Capability>) -> Self {
        self.capabilities = caps.into_iter().collect();
        self
    }

    /// Add arguments
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Set CPU limit
    pub fn with_cpu_limit(mut self, limit_ms: u64) -> Self {
        self.cpu_limit_ms = limit_ms;
        self
    }

    /// Check if capability is granted
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap) || self.capabilities.contains(&Capability::System)
    }
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Memory used (bytes)
    pub memory_used: usize,
    /// Optional return value (JSON)
    pub return_value: Option<String>,
}

impl PluginResult {
    /// Create a successful result
    pub fn success(stdout: String) -> Self {
        Self {
            exit_code: 0,
            stdout,
            stderr: String::new(),
            duration_ms: 0,
            memory_used: 0,
            return_value: None,
        }
    }

    /// Create an error result
    pub fn error(stderr: String) -> Self {
        Self {
            exit_code: 1,
            stdout: String::new(),
            stderr,
            duration_ms: 0,
            memory_used: 0,
            return_value: None,
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Core plugin trait
#[async_trait]
pub trait DxPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn init(&mut self) -> anyhow::Result<()>;

    /// Execute the plugin with context
    async fn execute(&self, ctx: &PluginContext) -> anyhow::Result<PluginResult>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> anyhow::Result<()>;

    /// Check if plugin is healthy
    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_from_str() {
        assert_eq!(Capability::from_str("network"), Capability::Network);
        assert_eq!(Capability::from_str("file_read"), Capability::FileRead);
        assert_eq!(Capability::from_str("shell"), Capability::Shell);
    }

    #[test]
    fn test_capability_dangerous() {
        assert!(Capability::Shell.is_dangerous());
        assert!(Capability::System.is_dangerous());
        assert!(!Capability::Network.is_dangerous());
    }

    #[test]
    fn test_plugin_context_default() {
        let ctx = PluginContext::default();
        assert_eq!(ctx.memory_limit, 256 * 1024 * 1024);
        assert_eq!(ctx.cpu_limit_ms, 30_000);
    }
}
