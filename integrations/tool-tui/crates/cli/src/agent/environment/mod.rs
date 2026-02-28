//! Multi-Runtime WASM Bridge
//!
//! This module provides a unified interface for compiling code from multiple
//! runtimes (Node.js, Python, Go, Rust, Deno, Bun) into WASM components that
//! can be executed in the DX Agent sandbox.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                      Environment Manager                            │
//! │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │
//! │  │  Node.js  │ │  Python   │ │    Go     │ │   Rust    │           │
//! │  │   javy    │ │componentize│ │  tinygo   │ │  cargo    │           │
//! │  └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘           │
//! │        │             │             │             │                  │
//! │        └─────────────┴──────┬──────┴─────────────┘                  │
//! │                             ▼                                       │
//! │                    ┌─────────────────┐                              │
//! │                    │  WASM Component │                              │
//! │                    └────────┬────────┘                              │
//! │                             ▼                                       │
//! │                    ┌─────────────────┐                              │
//! │                    │   DxHost API    │                              │
//! │                    └─────────────────┘                              │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supported Runtimes
//!
//! | Runtime | Compiler | Status |
//! |---------|----------|--------|
//! | Node.js | javy     | ✓      |
//! | Python  | componentize-py | ✓ |
//! | Go      | tinygo   | ✓      |
//! | Rust    | cargo-component | ✓ |
//! | Deno    | native   | ✓      |
//! | Bun     | javy     | ✓      |

pub mod channel_creator;
pub mod compiler;
pub mod host;
pub mod manager;
pub mod native;

use std::path::PathBuf;
use thiserror::Error;

/// Supported runtime environments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Runtime {
    /// Node.js runtime (compiled via javy)
    NodeJs = 0,
    /// Python runtime (compiled via componentize-py)
    Python = 1,
    /// Go runtime (compiled via tinygo)
    Go = 2,
    /// Rust runtime (compiled via cargo-component)
    Rust = 3,
    /// Deno runtime (native WASM support)
    Deno = 4,
    /// Bun runtime (compiled via javy)
    Bun = 5,
}

impl Runtime {
    /// Get the compiler tool name for this runtime
    pub const fn compiler_name(&self) -> &'static str {
        match self {
            Runtime::NodeJs => "javy",
            Runtime::Python => "componentize-py",
            Runtime::Go => "tinygo",
            Runtime::Rust => "cargo-component",
            Runtime::Deno => "deno",
            Runtime::Bun => "javy",
        }
    }

    /// Get file extensions associated with this runtime
    pub const fn extensions(&self) -> &'static [&'static str] {
        match self {
            Runtime::NodeJs => &["js", "mjs", "cjs"],
            Runtime::Python => &["py", "pyw"],
            Runtime::Go => &["go"],
            Runtime::Rust => &["rs"],
            Runtime::Deno => &["ts", "tsx", "js", "jsx"],
            Runtime::Bun => &["ts", "tsx", "js", "jsx"],
        }
    }

    /// Detect runtime from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" | "pyw" => Some(Runtime::Python),
            "go" => Some(Runtime::Go),
            "rs" => Some(Runtime::Rust),
            "js" | "mjs" | "cjs" => Some(Runtime::NodeJs),
            "ts" | "tsx" | "jsx" => Some(Runtime::Deno), // Default to Deno for TS
            _ => None,
        }
    }

    /// All supported runtimes
    pub const fn all() -> &'static [Runtime] {
        &[
            Runtime::NodeJs,
            Runtime::Python,
            Runtime::Go,
            Runtime::Rust,
            Runtime::Deno,
            Runtime::Bun,
        ]
    }
}

impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runtime::NodeJs => write!(f, "Node.js"),
            Runtime::Python => write!(f, "Python"),
            Runtime::Go => write!(f, "Go"),
            Runtime::Rust => write!(f, "Rust"),
            Runtime::Deno => write!(f, "Deno"),
            Runtime::Bun => write!(f, "Bun"),
        }
    }
}

/// Environment-related errors
#[derive(Error, Debug)]
pub enum EnvironmentError {
    #[error("Runtime not installed: {runtime}")]
    RuntimeNotInstalled { runtime: Runtime },

    #[error("Compiler not found: {compiler}")]
    CompilerNotFound { compiler: String },

    #[error("Compilation failed: {message}")]
    CompilationFailed { message: String },

    #[error("Invalid WASM module: {reason}")]
    InvalidWasm { reason: String },

    #[error("Capability denied: {capability}")]
    CapabilityDenied { capability: String },

    #[error("IPC error: {message}")]
    IpcError { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    #[error("Installation failed: {reason}")]
    InstallationFailed { reason: String },

    #[error("Cache error: {reason}")]
    CacheError { reason: String },

    #[error("Channel creation failed: {reason}")]
    ChannelCreationFailed { reason: String },
}

/// Result type for environment operations
pub type EnvironmentResult<T> = Result<T, EnvironmentError>;

/// Configuration for the environment manager
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// Root directory for DX configuration (~/.dx/)
    pub dx_root: PathBuf,
    /// Cache directory for compiled WASM (~/.dx/compiled-cache/)
    pub cache_dir: PathBuf,
    /// Directory for installed runtimes
    pub runtimes_dir: PathBuf,
    /// Maximum cache size in bytes (default: 1GB)
    pub max_cache_size: u64,
    /// Enable automatic runtime installation
    pub auto_install: bool,
    /// Enable compilation caching
    pub cache_enabled: bool,
    /// WASM optimization level (0-4)
    pub optimization_level: u8,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        let dx_root = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".dx");

        Self {
            cache_dir: dx_root.join("compiled-cache"),
            runtimes_dir: dx_root.join("runtimes"),
            dx_root,
            max_cache_size: 1024 * 1024 * 1024, // 1GB
            auto_install: true,
            cache_enabled: true,
            optimization_level: 2,
        }
    }
}

/// Schema for environments.sr file
#[derive(Debug, Clone)]
pub struct EnvironmentsSchema {
    /// Installed runtimes with their versions
    pub runtimes: Vec<RuntimeEntry>,
    /// Compiled WASM cache entries
    pub cache_entries: Vec<CacheEntry>,
    /// Last scan timestamp
    pub last_scan: u64,
}

/// Entry for a single runtime in environments.sr
#[derive(Debug, Clone)]
pub struct RuntimeEntry {
    /// Runtime type
    pub runtime: Runtime,
    /// Installed version
    pub version: String,
    /// Installation path
    pub path: PathBuf,
    /// Last verified timestamp
    pub last_verified: u64,
    /// Health status
    pub healthy: bool,
}

/// Entry for a cached WASM compilation
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Source file hash
    pub source_hash: [u8; 32],
    /// Compiled WASM path
    pub wasm_path: PathBuf,
    /// Compilation timestamp
    pub compiled_at: u64,
    /// Source runtime
    pub runtime: Runtime,
    /// File size in bytes
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_detection() {
        assert_eq!(Runtime::from_extension("py"), Some(Runtime::Python));
        assert_eq!(Runtime::from_extension("go"), Some(Runtime::Go));
        assert_eq!(Runtime::from_extension("rs"), Some(Runtime::Rust));
        assert_eq!(Runtime::from_extension("js"), Some(Runtime::NodeJs));
        assert_eq!(Runtime::from_extension("ts"), Some(Runtime::Deno));
        assert_eq!(Runtime::from_extension("unknown"), None);
    }

    #[test]
    fn test_runtime_compiler_names() {
        assert_eq!(Runtime::NodeJs.compiler_name(), "javy");
        assert_eq!(Runtime::Python.compiler_name(), "componentize-py");
        assert_eq!(Runtime::Go.compiler_name(), "tinygo");
        assert_eq!(Runtime::Rust.compiler_name(), "cargo-component");
    }

    #[test]
    fn test_config_defaults() {
        let config = EnvironmentConfig::default();
        assert!(config.auto_install);
        assert!(config.cache_enabled);
        assert_eq!(config.optimization_level, 2);
        assert_eq!(config.max_cache_size, 1024 * 1024 * 1024);
    }
}
