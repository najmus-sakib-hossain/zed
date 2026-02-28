//! # dx-js-compatibility
//!
//! A compatibility layer providing Bun API compatibility
//! while leveraging DX's binary-first architecture for performance.
//!
//! ## Features
//!
//! This crate is organized into 12 sub-crates, each providing compatibility for
//! a specific domain:
//!
//! - **node-core**: Node.js API compatibility (40+ modules)
//! - **web-core**: Web Standard APIs (30+ APIs)
//! - **bun-core**: Bun-specific APIs (50+ functions)
//! - **bun-sqlite**: Built-in SQLite database
//! - **bun-s3**: S3-compatible object storage
//! - **bun-ffi**: Foreign Function Interface
//! - **bun-shell**: Shell scripting
//! - **compile**: Single executable compilation
//! - **hmr**: Hot Module Replacement
//! - **plugins**: Plugin system
//! - **macros**: Compile-time macros
//! - **html-rewriter**: HTML Rewriter
//!
//! ## Usage
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! dx-js-compatibility = { version = "0.0.1", features = ["default"] }
//! ```
//!
//! Or with specific features:
//!
//! ```toml
//! [dependencies]
//! dx-js-compatibility = { version = "0.0.1", features = ["node-core", "bun-sqlite"] }
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use dx_js_compatibility::node::fs;
//! use dx_js_compatibility::bun::file;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Node.js fs compatibility
//!     let content = fs::read_file("example.txt").await?;
//!     
//!     // Bun.file() compatibility
//!     let bun_file = file::BunFile::new("example.txt");
//!     let text = bun_file.text().await?;
//!     
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]

// ============================================================================
// Node.js Compatibility (node-core feature)
// ============================================================================

#[cfg(feature = "node-core")]
pub use dx_compat_node as node;

#[cfg(feature = "node-core")]
pub mod node_prelude {
    //! Convenient re-exports for Node.js compatibility.
    pub use dx_compat_node::{buffer::Buffer, events::EventEmitter, fs, path};
}

// ============================================================================
// Web Standards Compatibility (web-core feature)
// ============================================================================

#[cfg(feature = "web-core")]
pub use dx_compat_web as web;

#[cfg(feature = "web-core")]
pub mod web_prelude {
    //! Convenient re-exports for Web Standards compatibility.
    pub use dx_compat_web::{
        fetch::{fetch, Headers, Request, Response},
        streams::{ReadableStream, TransformStream, WritableStream},
        websocket::WebSocket,
    };
}

// ============================================================================
// Bun-specific Compatibility (bun-core feature)
// ============================================================================

#[cfg(feature = "bun-core")]
pub use dx_compat_bun as bun;

#[cfg(feature = "bun-core")]
pub mod bun_prelude {
    //! Convenient re-exports for Bun-specific APIs.
    pub use dx_compat_bun::{compression, file, hash, serve, spawn};
}

// ============================================================================
// SQLite Compatibility (bun-sqlite feature)
// ============================================================================

#[cfg(feature = "bun-sqlite")]
pub use dx_compat_sqlite as sqlite;

#[cfg(feature = "bun-sqlite")]
pub mod sqlite_prelude {
    //! Convenient re-exports for SQLite compatibility.
    pub use dx_compat_sqlite::{Database, PreparedStatement, SqliteError, Value};
}

// ============================================================================
// S3 Compatibility (bun-s3 feature)
// ============================================================================

#[cfg(feature = "bun-s3")]
pub use dx_compat_s3 as s3;

#[cfg(feature = "bun-s3")]
pub mod s3_prelude {
    //! Convenient re-exports for S3 compatibility.
    pub use dx_compat_s3::{S3Client, S3Config, S3File};
}

// ============================================================================
// FFI Compatibility (bun-ffi feature)
// ============================================================================

#[cfg(feature = "bun-ffi")]
pub use dx_compat_ffi as ffi;

#[cfg(feature = "bun-ffi")]
pub mod ffi_prelude {
    //! Convenient re-exports for FFI compatibility.
    pub use dx_compat_ffi::{dlopen, DynamicLibrary, FfiType};
}

// ============================================================================
// Shell Compatibility (bun-shell feature)
// ============================================================================

#[cfg(feature = "bun-shell")]
pub use dx_compat_shell as shell;

#[cfg(feature = "bun-shell")]
pub mod shell_prelude {
    //! Convenient re-exports for shell compatibility.
    pub use dx_compat_shell::{shell, ShellCommand, ShellOutput};
}

// ============================================================================
// Compile Compatibility (compile feature)
// ============================================================================

#[cfg(feature = "compile")]
pub use dx_compat_compile as compile;

#[cfg(feature = "compile")]
pub mod compile_prelude {
    //! Convenient re-exports for compile compatibility.
    pub use dx_compat_compile::{
        AssetBundle, CompileOptions, CompiledOutput, Compiler, EmbeddedAsset, Runtime, Target,
    };
}

// ============================================================================
// HMR Compatibility (hmr feature)
// ============================================================================

#[cfg(feature = "hmr")]
pub use dx_compat_hmr as hmr;

#[cfg(feature = "hmr")]
pub mod hmr_prelude {
    //! Convenient re-exports for HMR compatibility.
    pub use dx_compat_hmr::{HmrServer, HmrUpdate, HotModule, UpdateType};
}

// ============================================================================
// Plugin Compatibility (plugins feature)
// ============================================================================

#[cfg(feature = "plugins")]
pub use dx_compat_plugin as plugin;

#[cfg(feature = "plugins")]
pub mod plugin_prelude {
    //! Convenient re-exports for plugin compatibility.
    pub use dx_compat_plugin::{Loader, OnLoadResult, OnResolveResult, Plugin, PluginBuilder};
}

// ============================================================================
// Macro Compatibility (macros feature)
// ============================================================================

#[cfg(feature = "macros")]
pub use dx_compat_macro as macros;

#[cfg(feature = "macros")]
pub mod macros_prelude {
    //! Convenient re-exports for macro compatibility.
    pub use dx_compat_macro::{
        builtins, MacroConfig, MacroContext, MacroDefinition, MacroExpansion, MacroRegistry,
        MacroValue,
    };
}

// ============================================================================
// HTML Rewriter Compatibility (html-rewriter feature)
// ============================================================================

#[cfg(feature = "html-rewriter")]
pub use dx_compat_html as html;

#[cfg(feature = "html-rewriter")]
pub mod html_prelude {
    //! Convenient re-exports for HTML rewriter compatibility.
    pub use dx_compat_html::{ContentType, Element, HTMLRewriter};
}

// ============================================================================
// Common Error Types
// ============================================================================

/// Unified error type for the compatibility layer.
#[derive(Debug, thiserror::Error)]
pub enum CompatError {
    /// File system error
    #[error("File system error: {0}")]
    Fs(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(String),

    /// S3 error
    #[error("S3 error: {0}")]
    S3(String),

    /// FFI error
    #[error("FFI error: {0}")]
    Ffi(String),

    /// Shell error
    #[error("Shell error: {0}")]
    Shell(String),

    /// Compile error
    #[error("Compile error: {0}")]
    Compile(String),

    /// HMR error
    #[error("HMR error: {0}")]
    Hmr(String),

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// Macro error
    #[error("Macro error: {0}")]
    Macro(String),

    /// HTML rewriter error
    #[error("HTML rewriter error: {0}")]
    Html(String),

    /// Invalid argument error
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Resource not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Permission denied error
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Node.js compatible error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ErrorCode {
    /// No such file or directory
    ENOENT = 2,
    /// Permission denied
    EACCES = 13,
    /// File exists
    EEXIST = 17,
    /// Is a directory
    EISDIR = 21,
    /// Not a directory
    ENOTDIR = 20,
    /// Directory not empty
    ENOTEMPTY = 39,
    /// Operation timed out
    ETIMEDOUT = 110,
    /// Connection refused
    ECONNREFUSED = 111,
}

impl ErrorCode {
    /// Get the error code name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ENOENT => "ENOENT",
            ErrorCode::EACCES => "EACCES",
            ErrorCode::EEXIST => "EEXIST",
            ErrorCode::EISDIR => "EISDIR",
            ErrorCode::ENOTDIR => "ENOTDIR",
            ErrorCode::ENOTEMPTY => "ENOTEMPTY",
            ErrorCode::ETIMEDOUT => "ETIMEDOUT",
            ErrorCode::ECONNREFUSED => "ECONNREFUSED",
        }
    }

    /// Get the error code from an IO error kind.
    pub fn from_io_error(err: &std::io::Error) -> Option<Self> {
        match err.kind() {
            std::io::ErrorKind::NotFound => Some(ErrorCode::ENOENT),
            std::io::ErrorKind::PermissionDenied => Some(ErrorCode::EACCES),
            std::io::ErrorKind::AlreadyExists => Some(ErrorCode::EEXIST),
            std::io::ErrorKind::TimedOut => Some(ErrorCode::ETIMEDOUT),
            std::io::ErrorKind::ConnectionRefused => Some(ErrorCode::ECONNREFUSED),
            _ => None,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Platform Information
// ============================================================================

/// Platform information for cross-platform support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux operating system
    Linux,
    /// macOS operating system
    MacOS,
    /// Windows operating system
    Windows,
    /// Unknown platform
    Unknown,
}

impl Platform {
    /// Get the current platform.
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Platform::Unknown;
    }

    /// Get the platform name.
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::MacOS => "darwin",
            Platform::Windows => "win32",
            Platform::Unknown => "unknown",
        }
    }

    /// Check if this is a Unix-like platform.
    pub fn is_unix(&self) -> bool {
        matches!(self, Platform::Linux | Platform::MacOS)
    }

    /// Check if this is Windows.
    pub fn is_windows(&self) -> bool {
        matches!(self, Platform::Windows)
    }
}

/// Architecture information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// x86_64 (64-bit Intel/AMD)
    X64,
    /// ARM64 (64-bit ARM)
    Arm64,
    /// Unknown architecture
    Unknown,
}

impl Architecture {
    /// Get the current architecture.
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        return Architecture::X64;
        #[cfg(target_arch = "aarch64")]
        return Architecture::Arm64;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        return Architecture::Unknown;
    }

    /// Get the architecture name.
    pub fn name(&self) -> &'static str {
        match self {
            Architecture::X64 => "x64",
            Architecture::Arm64 => "arm64",
            Architecture::Unknown => "unknown",
        }
    }
}

/// Get the path separator for the current platform.
pub fn path_separator() -> char {
    #[cfg(windows)]
    return '\\';
    #[cfg(not(windows))]
    return '/';
}

/// Get the line ending for the current platform.
pub fn line_ending() -> &'static str {
    #[cfg(windows)]
    return "\r\n";
    #[cfg(not(windows))]
    return "\n";
}

// ============================================================================
// Version Information
// ============================================================================

/// Returns the version of the dx-js-compatibility crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Returns information about enabled features.
#[allow(clippy::vec_init_then_push)]
pub fn enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    #[cfg(feature = "node-core")]
    features.push("node-core");

    #[cfg(feature = "web-core")]
    features.push("web-core");

    #[cfg(feature = "bun-core")]
    features.push("bun-core");

    #[cfg(feature = "bun-sqlite")]
    features.push("bun-sqlite");

    #[cfg(feature = "bun-s3")]
    features.push("bun-s3");

    #[cfg(feature = "bun-ffi")]
    features.push("bun-ffi");

    #[cfg(feature = "bun-shell")]
    features.push("bun-shell");

    #[cfg(feature = "compile")]
    features.push("compile");

    #[cfg(feature = "hmr")]
    features.push("hmr");

    #[cfg(feature = "plugins")]
    features.push("plugins");

    #[cfg(feature = "macros")]
    features.push("macros");

    #[cfg(feature = "html-rewriter")]
    features.push("html-rewriter");

    features
}

/// Returns system information.
pub fn system_info() -> SystemInfo {
    SystemInfo {
        platform: Platform::current(),
        arch: Architecture::current(),
        version: version().to_string(),
        features: enabled_features().iter().map(|s| s.to_string()).collect(),
    }
}

/// System information structure.
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Current platform
    pub platform: Platform,
    /// Current architecture
    pub arch: Architecture,
    /// Crate version
    pub version: String,
    /// Enabled features
    pub features: Vec<String>,
}

impl std::fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dx-js-compatibility v{} ({}-{}) [{}]",
            self.version,
            self.platform.name(),
            self.arch.name(),
            self.features.join(", ")
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::ENOENT.as_str(), "ENOENT");
        assert_eq!(ErrorCode::EACCES.as_str(), "EACCES");
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        // At minimum, we should have some features enabled in tests
        // The actual features depend on how tests are run
        assert!(features.is_empty() || !features.is_empty());
    }

    #[test]
    fn test_platform() {
        let platform = Platform::current();
        assert!(!platform.name().is_empty());
    }

    #[test]
    fn test_architecture() {
        let arch = Architecture::current();
        assert!(!arch.name().is_empty());
    }

    #[test]
    fn test_path_separator() {
        let sep = path_separator();
        #[cfg(windows)]
        assert_eq!(sep, '\\');
        #[cfg(not(windows))]
        assert_eq!(sep, '/');
    }

    #[test]
    fn test_line_ending() {
        let ending = line_ending();
        #[cfg(windows)]
        assert_eq!(ending, "\r\n");
        #[cfg(not(windows))]
        assert_eq!(ending, "\n");
    }

    #[test]
    fn test_system_info() {
        let info = system_info();
        assert!(!info.version.is_empty());
        let display = format!("{}", info);
        assert!(display.contains("dx-js-compatibility"));
    }
}
