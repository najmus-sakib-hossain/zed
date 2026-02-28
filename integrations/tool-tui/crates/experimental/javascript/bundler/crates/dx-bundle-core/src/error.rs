//! Error types for DX JS Bundler

use std::path::PathBuf;
use thiserror::Error;

/// Bundler error types
#[derive(Error, Debug)]
pub enum BundleError {
    #[error("IO error: {message}\n  Path: {}\n\n  💡 Suggestions:\n     - Check file/directory permissions\n     - Ensure the file exists\n     - Verify the path is correct",
        path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown>".to_string()))]
    Io {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Module not found: {path}\n\n  💡 Suggestions:\n     - Check if the module is installed: dx add <package>\n     - Verify the import path is correct\n     - Check for typos in the module name")]
    ModuleNotFound { path: PathBuf },

    #[error("Parse error at {file}:{line}:{column}: {message}\n\n  💡 Check the syntax at the specified location.\n     Common issues: missing semicolons, unmatched brackets, invalid syntax.")]
    ParseError {
        file: PathBuf,
        line: u32,
        column: u32,
        message: String,
    },

    #[error("Transform error: {message}\n\n  💡 This error occurred during code transformation.\n     Check if the source code uses unsupported syntax.")]
    TransformError { message: String },

    #[error("Circular dependency detected: {chain}\n\n  💡 Circular dependencies can cause issues.\n     Consider refactoring to break the cycle.")]
    CircularDependency { chain: String },

    #[error("Arena exhausted: requested {requested} bytes, {available} available\n\n  💡 The bundler ran out of memory.\n     Try bundling smaller chunks or increasing memory limits.")]
    ArenaExhausted { requested: usize, available: usize },

    #[error("Cache corruption: {message}\n\n  💡 The cache may be corrupted.\n     Try clearing the cache: rm -rf .dx/cache")]
    CacheCorruption { message: String },

    #[error("Invalid binary format: expected {expected}, got {got}\n\n  💡 The file format is not recognized.\n     This may indicate a corrupted or incompatible file.")]
    InvalidFormat { expected: String, got: String },

    #[error("Resolution failed for '{specifier}' from '{from}'\n\n  💡 Suggestions:\n     - Check if the package is installed\n     - Verify the import path is correct\n     - Check for typos in the module specifier")]
    ResolutionFailed { specifier: String, from: PathBuf },

    #[error("Unsupported feature: {feature}\n\n  💡 This feature is not yet supported by the bundler.\n     Consider using an alternative approach or filing a feature request.")]
    UnsupportedFeature { feature: String },

    #[error("Permission denied: {operation} on {path}\n\n  💡 Suggestions:\n     - Check file/directory permissions\n     - Run with appropriate privileges\n     - Verify the path is not read-only")]
    PermissionDenied { operation: String, path: PathBuf },
}

impl From<std::io::Error> for BundleError {
    fn from(err: std::io::Error) -> Self {
        BundleError::Io {
            message: err.to_string(),
            path: None,
        }
    }
}

impl BundleError {
    pub fn module_not_found(path: impl Into<PathBuf>) -> Self {
        Self::ModuleNotFound { path: path.into() }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            file: PathBuf::new(),
            line: 0,
            column: 0,
            message: message.into(),
        }
    }

    pub fn parse_error_at(
        file: impl Into<PathBuf>,
        line: u32,
        column: u32,
        message: impl Into<String>,
    ) -> Self {
        Self::ParseError {
            file: file.into(),
            line,
            column,
            message: message.into(),
        }
    }

    pub fn transform_error(message: impl Into<String>) -> Self {
        Self::TransformError {
            message: message.into(),
        }
    }

    pub fn arena_exhausted(requested: usize, available: usize) -> Self {
        Self::ArenaExhausted {
            requested,
            available,
        }
    }

    pub fn cache_corruption(message: impl Into<String>) -> Self {
        Self::CacheCorruption {
            message: message.into(),
        }
    }

    pub fn resolution_failed(specifier: impl Into<String>, from: impl Into<PathBuf>) -> Self {
        Self::ResolutionFailed {
            specifier: specifier.into(),
            from: from.into(),
        }
    }

    pub fn io_with_path(err: std::io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Io {
            message: err.to_string(),
            path: Some(path.into()),
        }
    }

    pub fn permission_denied(operation: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
            path: path.into(),
        }
    }
}

/// Result type alias
pub type BundleResult<T> = Result<T, BundleError>;
