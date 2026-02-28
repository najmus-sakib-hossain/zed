//! Error types for extension loading

use std::path::PathBuf;
use thiserror::Error;

use crate::abi::AbiVersion;

/// Result type for extension operations
pub type ExtensionResult<T> = Result<T, ExtensionError>;

/// Errors that can occur during extension loading
#[derive(Debug, Error)]
pub enum ExtensionError {
    /// Extension file not found
    #[error("Extension not found: {name}. Searched paths: {searched_paths:?}")]
    NotFound {
        name: String,
        searched_paths: Vec<PathBuf>,
    },

    /// ABI version mismatch
    #[error("ABI mismatch for {name}: expected {expected}, found {found}")]
    AbiMismatch {
        name: String,
        expected: AbiVersion,
        found: AbiVersion,
    },

    /// Failed to load the shared library
    #[error("Failed to load extension {name} from {path}: {reason}")]
    LoadFailure {
        name: String,
        path: PathBuf,
        reason: String,
    },

    /// Module initialization failed
    #[error("Module initialization failed for {name}: {reason}")]
    InitFailure { name: String, reason: String },

    /// Invalid extension file
    #[error("Invalid extension file {path}: {reason}")]
    InvalidExtension { path: PathBuf, reason: String },

    /// Unsupported API functions used
    #[error("Extension {name} uses unsupported API functions: {message}")]
    UnsupportedApi {
        name: String,
        functions: Vec<String>,
        message: String,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Library loading error
    #[error("Library error: {0}")]
    LibLoading(String),
}

impl ExtensionError {
    /// Get the extension name if available
    pub fn extension_name(&self) -> Option<&str> {
        match self {
            ExtensionError::NotFound { name, .. } => Some(name),
            ExtensionError::AbiMismatch { name, .. } => Some(name),
            ExtensionError::LoadFailure { name, .. } => Some(name),
            ExtensionError::InitFailure { name, .. } => Some(name),
            ExtensionError::UnsupportedApi { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ExtensionError::NotFound { .. })
    }

    /// Check if this is an unsupported API error
    pub fn is_unsupported_api(&self) -> bool {
        matches!(self, ExtensionError::UnsupportedApi { .. })
    }

    /// Get the list of unsupported functions if this is an UnsupportedApi error
    pub fn unsupported_functions(&self) -> Option<&[String]> {
        match self {
            ExtensionError::UnsupportedApi { functions, .. } => Some(functions),
            _ => None,
        }
    }
}
