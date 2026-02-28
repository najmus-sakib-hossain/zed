//! Error type definitions

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Main error type for the DX CLI
#[derive(Error, Debug)]
pub enum DxError {
    /// Configuration file not found at the specified path
    #[error("Configuration file not found: {path}")]
    ConfigNotFound {
        /// Path to the missing configuration file
        path: PathBuf,
    },

    /// Invalid configuration file content
    #[error("Invalid configuration: {message}\n  → at {path}:{line}")]
    ConfigInvalid {
        /// Path to the invalid configuration file
        path: PathBuf,
        /// Line number where the error occurred
        line: usize,
        /// Error message describing the issue
        message: String,
    },

    /// Required field missing from configuration
    #[error("Missing required field '{field}' in configuration")]
    ConfigMissingField {
        /// Name of the missing field
        field: String,
    },

    /// File not found at the specified path
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path to the missing file
        path: PathBuf,
    },

    /// Directory not found at the specified path
    #[error("Directory not found: {path}")]
    DirectoryNotFound {
        /// Path to the missing directory
        path: PathBuf,
    },

    /// Permission denied when accessing a path
    #[error("Permission denied: {path}")]
    PermissionDenied {
        /// Path that was denied access
        path: PathBuf,
    },

    /// File already exists at the specified path
    #[error("File already exists: {path}")]
    FileExists {
        /// Path to the existing file
        path: PathBuf,
    },

    /// General I/O error
    #[error("I/O error: {message}")]
    Io {
        /// Error message
        message: String,
    },

    /// Symlink loop detected (too many levels of indirection)
    #[error("Too many levels of symbolic links (max 40): {path}")]
    SymlinkLoop {
        /// Path where the loop was detected
        path: PathBuf,
    },

    /// Network communication error
    #[error("Network error: {message}")]
    Network {
        /// Error message
        message: String,
    },

    /// Request timed out
    #[error("Request timed out after {timeout_secs}s")]
    Timeout {
        /// Timeout duration in seconds
        timeout_secs: u64,
    },

    /// TLS/SSL error
    #[error("TLS error: {message}")]
    Tls {
        /// Error message
        message: String,
    },

    /// HTTP error with status code
    #[error("HTTP error {status}: {message}")]
    Http {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Required tool not installed
    #[error("Tool not installed: {name}\n  → Run `dx forge install {name}` to install")]
    ToolNotInstalled {
        /// Name of the missing tool
        name: String,
    },

    /// Tool version does not match requirements
    #[error("Tool version mismatch: {name} requires {required}, found {found}")]
    ToolVersionMismatch {
        /// Name of the tool
        name: String,
        /// Required version
        required: String,
        /// Found version
        found: String,
    },

    /// Tool execution failed
    #[error("Tool execution failed: {name}\n  → {message}")]
    ToolExecutionFailed {
        /// Name of the tool
        name: String,
        /// Error message
        message: String,
    },

    /// Build process failed
    #[error("Build failed: {message}")]
    BuildFailed {
        /// Error message
        message: String,
    },

    /// Compilation error at a specific location
    #[error("Compilation error at {file}:{line}:{column}\n  → {message}")]
    CompilationError {
        /// Source file path
        file: PathBuf,
        /// Line number
        line: usize,
        /// Column number
        column: usize,
        /// Error message
        message: String,
    },

    /// Signature verification failed during update
    #[error("Signature verification failed")]
    SignatureInvalid,

    /// Checksum verification failed
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// Expected checksum
        expected: String,
        /// Actual checksum
        actual: String,
    },

    /// Update download failed
    #[error("Failed to download update: {message}")]
    UpdateDownloadFailed {
        /// Error message
        message: String,
    },

    /// Delta patch application failed
    #[error("Delta patch failed: {message}")]
    DeltaPatchFailed {
        /// Error message
        message: String,
    },

    /// Shell type could not be detected
    #[error("Could not detect shell type")]
    ShellNotDetected,

    /// Shell integration already installed
    #[error("Shell integration already installed for {shell}")]
    ShellIntegrationExists {
        /// Shell name
        shell: String,
    },

    /// Invalid argument provided
    #[error("Invalid argument: {message}")]
    InvalidArgument {
        /// Error message
        message: String,
    },

    /// Operation was cancelled by user
    #[error("Operation cancelled")]
    Cancelled,

    /// Internal error (bug)
    #[error("Internal error: {message}")]
    Internal {
        /// Error message
        message: String,
    },

    /// File lock acquisition timed out
    #[error("Lock acquisition timed out after {timeout:?} for: {path}")]
    LockTimeout {
        /// Path to the locked file
        path: PathBuf,
        /// Timeout duration
        timeout: Duration,
    },
}

impl DxError {
    /// Create a ConfigNotFound error
    pub fn config_not_found(path: impl Into<PathBuf>) -> Self {
        DxError::ConfigNotFound { path: path.into() }
    }

    /// Create a ConfigInvalid error
    pub fn config_invalid(
        path: impl Into<PathBuf>,
        line: usize,
        message: impl Into<String>,
    ) -> Self {
        DxError::ConfigInvalid {
            path: path.into(),
            line,
            message: message.into(),
        }
    }

    /// Create a FileNotFound error
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        DxError::FileNotFound { path: path.into() }
    }

    /// Create a PermissionDenied error
    pub fn permission_denied(path: impl Into<PathBuf>) -> Self {
        DxError::PermissionDenied { path: path.into() }
    }

    /// Create a Network error
    pub fn network(message: impl Into<String>) -> Self {
        DxError::Network {
            message: message.into(),
        }
    }

    /// Create a Timeout error
    pub fn timeout(timeout_secs: u64) -> Self {
        DxError::Timeout { timeout_secs }
    }

    /// Create a ToolNotInstalled error
    pub fn tool_not_installed(name: impl Into<String>) -> Self {
        DxError::ToolNotInstalled { name: name.into() }
    }
}

impl From<std::io::Error> for DxError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => DxError::FileNotFound {
                path: PathBuf::from("<unknown>"),
            },
            ErrorKind::PermissionDenied => DxError::PermissionDenied {
                path: PathBuf::from("<unknown>"),
            },
            _ => DxError::Io {
                message: err.to_string(),
            },
        }
    }
}
