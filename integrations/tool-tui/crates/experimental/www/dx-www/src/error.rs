//! # Error Handling
//!
//! This module provides comprehensive error handling for the DX WWW Framework.
//! All errors are designed to provide helpful context and suggestions for resolution.

#![allow(unused)]

use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

// =============================================================================
// Main Error Type
// =============================================================================

/// Main error type for the DX WWW Framework.
#[derive(Debug, Error, Diagnostic)]
pub enum DxError {
    // -------------------------------------------------------------------------
    // Configuration Errors
    // -------------------------------------------------------------------------
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    #[diagnostic(
        code(dx::config::not_found),
        help(
            "Create a dx.config.toml file in your project root, or run `dx-www new` to create a new project"
        )
    )]
    ConfigNotFound {
        /// Path to the missing configuration file
        path: PathBuf,
    },

    /// Configuration parsing error
    #[error("Failed to parse configuration: {message}")]
    #[diagnostic(
        code(dx::config::parse_error),
        help("Check your dx.config.toml for syntax errors")
    )]
    ConfigParseError {
        /// Error message
        message: String,
        /// Source file
        #[source_code]
        src: Option<String>,
        /// Error span
        #[label("error here")]
        span: Option<miette::SourceSpan>,
    },

    /// Configuration validation error
    #[error("Configuration validation failed: {message}")]
    #[diagnostic(code(dx::config::validation_error))]
    ConfigValidationError {
        /// Error message
        message: String,
        /// Field that failed validation
        field: Option<String>,
    },

    // -------------------------------------------------------------------------
    // Project Structure Errors
    // -------------------------------------------------------------------------
    /// Project directory not found
    #[error("Project directory not found: {path}")]
    #[diagnostic(
        code(dx::project::not_found),
        help("Make sure you're in a DX WWW project directory")
    )]
    ProjectNotFound {
        /// Path to the missing directory
        path: PathBuf,
    },

    /// Invalid project structure
    #[error("Invalid project structure: {message}")]
    #[diagnostic(
        code(dx::project::invalid_structure),
        help("Run `dx-www new` to create a properly structured project")
    )]
    InvalidProjectStructure {
        /// Error message
        message: String,
    },

    // -------------------------------------------------------------------------
    // Routing Errors
    // -------------------------------------------------------------------------
    /// Route not found
    #[error("Route not found: {path}")]
    #[diagnostic(code(dx::router::not_found))]
    RouteNotFound {
        /// The requested path
        path: String,
    },

    /// Duplicate route
    #[error("Duplicate route detected: {path}")]
    #[diagnostic(code(dx::router::duplicate), help("Check for conflicting page files"))]
    DuplicateRoute {
        /// The duplicate path
        path: String,
        /// First file defining the route
        file1: PathBuf,
        /// Second file defining the route
        file2: PathBuf,
    },

    /// Invalid route pattern
    #[error("Invalid route pattern: {pattern}")]
    #[diagnostic(
        code(dx::router::invalid_pattern),
        help("Dynamic routes should use [param] or [...param] syntax")
    )]
    InvalidRoutePattern {
        /// The invalid pattern
        pattern: String,
    },

    // -------------------------------------------------------------------------
    // Parser Errors
    // -------------------------------------------------------------------------
    /// Component parse error
    #[error("Failed to parse component: {message}")]
    #[diagnostic(code(dx::parser::error))]
    ParseError {
        /// Error message
        message: String,
        /// Source file path
        file: PathBuf,
        /// Line number
        line: Option<usize>,
        /// Column number
        column: Option<usize>,
        /// Source code
        #[source_code]
        src: Option<String>,
        /// Error span
        #[label("error here")]
        span: Option<miette::SourceSpan>,
    },

    /// Missing required section in component
    #[error("Missing required section in component: {section}")]
    #[diagnostic(
        code(dx::parser::missing_section),
        help("Components must have a <template> section")
    )]
    MissingSection {
        /// The missing section name
        section: String,
        /// File path
        file: PathBuf,
    },

    /// Invalid script language
    #[error("Invalid script language: {language}")]
    #[diagnostic(
        code(dx::parser::invalid_language),
        help("Supported languages: rust, python, javascript, typescript, go")
    )]
    InvalidScriptLanguage {
        /// The invalid language
        language: String,
        /// File path
        file: PathBuf,
    },

    // -------------------------------------------------------------------------
    // Build Errors
    // -------------------------------------------------------------------------
    /// Build failed
    #[error("Build failed: {message}")]
    #[diagnostic(code(dx::build::failed))]
    BuildFailed {
        /// Error message
        message: String,
    },

    /// Compilation error
    #[error("Compilation error in {file}: {message}")]
    #[diagnostic(code(dx::build::compilation_error))]
    CompilationError {
        /// Error message
        message: String,
        /// File that failed to compile
        file: PathBuf,
        /// Source code
        #[source_code]
        src: Option<String>,
        /// Error span
        #[label("error here")]
        span: Option<miette::SourceSpan>,
    },

    /// Dependency resolution error
    #[error("Failed to resolve dependency: {dependency}")]
    #[diagnostic(code(dx::build::dependency_error))]
    DependencyError {
        /// The dependency that failed to resolve
        dependency: String,
        /// Reason for failure
        reason: String,
    },

    /// Syntax error in source code
    #[error("Syntax error: {message}")]
    #[diagnostic(code(dx::build::syntax_error))]
    SyntaxError {
        /// Error message
        message: String,
        /// File containing the error
        file: Option<PathBuf>,
        /// Line number
        line: Option<usize>,
        /// Column number
        column: Option<usize>,
    },

    /// Binary format error (DXOB, DXS1, DXT1, etc.)
    #[error("Binary format error: {message}")]
    #[diagnostic(code(dx::build::binary_format_error))]
    BinaryFormatError {
        /// Error message
        message: String,
    },

    /// Build cache error
    #[error("Cache error: {message}")]
    #[diagnostic(code(dx::build::cache_error))]
    CacheError {
        /// Error message
        message: String,
    },

    // -------------------------------------------------------------------------
    // Data Loader Errors
    // -------------------------------------------------------------------------
    /// Data loader error
    #[error("Data loader failed: {message}")]
    #[diagnostic(code(dx::data::loader_error))]
    DataLoaderError {
        /// Error message
        message: String,
        /// Route that triggered the error
        route: String,
    },

    /// Data loader timeout
    #[error("Data loader timeout for route: {route}")]
    #[diagnostic(
        code(dx::data::timeout),
        help("Consider optimizing your data loader or increasing the timeout")
    )]
    DataLoaderTimeout {
        /// Route that timed out
        route: String,
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    // -------------------------------------------------------------------------
    // API Route Errors
    // -------------------------------------------------------------------------
    /// API handler error
    #[error("API handler error: {message}")]
    #[diagnostic(code(dx::api::handler_error))]
    ApiHandlerError {
        /// Error message
        message: String,
        /// HTTP status code
        status: u16,
    },

    /// Invalid HTTP method
    #[error("Invalid HTTP method: {method}")]
    #[diagnostic(code(dx::api::invalid_method))]
    InvalidHttpMethod {
        /// The invalid method
        method: String,
        /// Allowed methods
        allowed: Vec<String>,
    },

    // -------------------------------------------------------------------------
    // Dev Server Errors
    // -------------------------------------------------------------------------
    /// Dev server error
    #[error("Dev server error: {message}")]
    #[diagnostic(code(dx::dev::server_error))]
    DevServerError {
        /// Error message
        message: String,
    },

    /// Port already in use
    #[error("Port {port} is already in use")]
    #[diagnostic(
        code(dx::dev::port_in_use),
        help("Try a different port with --port or kill the process using port {port}")
    )]
    PortInUse {
        /// The port that's in use
        port: u16,
    },

    /// Hot reload connection failed
    #[error("Hot reload connection failed")]
    #[diagnostic(code(dx::dev::hot_reload_failed))]
    HotReloadFailed {
        /// Error message
        message: String,
    },

    // -------------------------------------------------------------------------
    // Asset Errors
    // -------------------------------------------------------------------------
    /// Asset not found
    #[error("Asset not found: {path}")]
    #[diagnostic(code(dx::assets::not_found))]
    AssetNotFound {
        /// Path to the missing asset
        path: PathBuf,
    },

    /// Asset optimization failed
    #[error("Failed to optimize asset: {path}")]
    #[diagnostic(code(dx::assets::optimization_failed))]
    AssetOptimizationFailed {
        /// Path to the asset
        path: PathBuf,
        /// Reason for failure
        reason: String,
    },

    // -------------------------------------------------------------------------
    // IO Errors
    // -------------------------------------------------------------------------
    /// IO error
    #[error("IO error: {message}")]
    #[diagnostic(code(dx::io::error))]
    IoError {
        /// Error message
        message: String,
        /// Path involved in the error
        path: Option<PathBuf>,
    },

    /// File read error
    #[error("Failed to read file: {path}")]
    #[diagnostic(code(dx::io::read_error))]
    FileReadError {
        /// Path to the file
        path: PathBuf,
        /// Underlying error
        #[source]
        source: std::io::Error,
    },

    /// File write error
    #[error("Failed to write file: {path}")]
    #[diagnostic(code(dx::io::write_error))]
    FileWriteError {
        /// Path to the file
        path: PathBuf,
        /// Underlying error
        #[source]
        source: std::io::Error,
    },

    // -------------------------------------------------------------------------
    // Generic Errors
    // -------------------------------------------------------------------------
    /// Internal error
    #[error("Internal error: {message}")]
    #[diagnostic(code(dx::internal))]
    InternalError {
        /// Error message
        message: String,
    },

    /// Feature not implemented
    #[error("Feature not implemented: {feature}")]
    #[diagnostic(code(dx::not_implemented))]
    NotImplemented {
        /// The feature that's not implemented
        feature: String,
    },
}

// =============================================================================
// Result Type Alias
// =============================================================================

/// Result type for DX WWW operations.
pub type DxResult<T> = Result<T, DxError>;

// =============================================================================
// Error Conversions
// =============================================================================

impl From<std::io::Error> for DxError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            message: err.to_string(),
            path: None,
        }
    }
}

impl From<toml::de::Error> for DxError {
    fn from(err: toml::de::Error) -> Self {
        Self::ConfigParseError {
            message: err.to_string(),
            src: None,
            span: None,
        }
    }
}

impl From<serde_json::Error> for DxError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError {
            message: err.to_string(),
            file: PathBuf::new(),
            line: Some(err.line()),
            column: Some(err.column()),
            src: None,
            span: None,
        }
    }
}

impl From<crate::config::ConfigError> for DxError {
    fn from(err: crate::config::ConfigError) -> Self {
        Self::ConfigValidationError {
            message: err.to_string(),
            field: None,
        }
    }
}

// =============================================================================
// Error Builder Helpers
// =============================================================================

impl DxError {
    /// Create a parse error with source context.
    pub fn parse_error_with_context(
        message: impl Into<String>,
        file: PathBuf,
        src: &str,
        line: usize,
        column: usize,
    ) -> Self {
        let offset =
            src.lines().take(line.saturating_sub(1)).map(|l| l.len() + 1).sum::<usize>() + column;

        Self::ParseError {
            message: message.into(),
            file,
            line: Some(line),
            column: Some(column),
            src: Some(src.to_string()),
            span: Some(miette::SourceSpan::new(offset.into(), 1usize.into())),
        }
    }

    /// Create a compilation error with source context.
    pub fn compilation_error_with_context(
        message: impl Into<String>,
        file: PathBuf,
        src: &str,
        offset: usize,
        length: usize,
    ) -> Self {
        Self::CompilationError {
            message: message.into(),
            file,
            src: Some(src.to_string()),
            span: Some(miette::SourceSpan::new(offset.into(), length.into())),
        }
    }

    /// Create a config validation error.
    pub fn config_validation(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::ConfigValidationError {
            message: message.into(),
            field: Some(field.into()),
        }
    }
}

// =============================================================================
// Error Overlay Data
// =============================================================================

/// Data for rendering error overlays in development.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorOverlayData {
    /// Error type classification
    pub error_type: ErrorType,
    /// Error message
    pub message: String,
    /// File path (if applicable)
    pub file_path: Option<PathBuf>,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Column number (if applicable)
    pub column: Option<usize>,
    /// Code context around the error
    pub code_context: Option<String>,
    /// Stack trace frames
    pub stack_trace: Option<Vec<StackFrame>>,
    /// Suggested fixes
    pub suggestions: Vec<String>,
}

/// Error type classification for overlays.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// Compilation error
    Compilation,
    /// Runtime error
    Runtime,
    /// Data loading error
    DataLoad,
    /// API error
    Api,
    /// Configuration error
    Config,
}

/// Stack frame for error traces.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StackFrame {
    /// Function or method name
    pub function: String,
    /// File path
    pub file: Option<PathBuf>,
    /// Line number
    pub line: Option<usize>,
    /// Column number
    pub column: Option<usize>,
}

impl ErrorOverlayData {
    /// Create error overlay data from a DxError.
    pub fn from_error(error: &DxError) -> Self {
        match error {
            DxError::CompilationError {
                message,
                file,
                src,
                span,
            } => Self {
                error_type: ErrorType::Compilation,
                message: message.clone(),
                file_path: Some(file.clone()),
                line: span.map(|s| s.offset()),
                column: None,
                code_context: src.clone(),
                stack_trace: None,
                suggestions: vec![],
            },
            DxError::ParseError {
                message,
                file,
                line,
                column,
                src,
                ..
            } => Self {
                error_type: ErrorType::Compilation,
                message: message.clone(),
                file_path: Some(file.clone()),
                line: *line,
                column: *column,
                code_context: src.clone(),
                stack_trace: None,
                suggestions: vec![],
            },
            DxError::DataLoaderError { message, route } => Self {
                error_type: ErrorType::DataLoad,
                message: format!("{message} (route: {route})"),
                file_path: None,
                line: None,
                column: None,
                code_context: None,
                stack_trace: None,
                suggestions: vec![],
            },
            DxError::ConfigValidationError { message, field } => Self {
                error_type: ErrorType::Config,
                message: if let Some(f) = field {
                    format!("{message} (field: {f})")
                } else {
                    message.clone()
                },
                file_path: Some(PathBuf::from("dx.config.toml")),
                line: None,
                column: None,
                code_context: None,
                stack_trace: None,
                suggestions: vec!["Check your dx.config.toml configuration".to_string()],
            },
            _ => Self {
                error_type: ErrorType::Runtime,
                message: error.to_string(),
                file_path: None,
                line: None,
                column: None,
                code_context: None,
                stack_trace: None,
                suggestions: vec![],
            },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = DxError::RouteNotFound {
            path: "/test".to_string(),
        };
        assert!(error.to_string().contains("/test"));
    }

    #[test]
    fn test_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let dx_error: DxError = io_error.into();
        assert!(matches!(dx_error, DxError::IoError { .. }));
    }

    #[test]
    fn test_error_overlay_data() {
        let error = DxError::CompilationError {
            message: "syntax error".to_string(),
            file: PathBuf::from("test.pg"),
            src: Some("let x = ".to_string()),
            span: Some(miette::SourceSpan::new(0usize.into(), 1usize.into())),
        };

        let overlay = ErrorOverlayData::from_error(&error);
        assert!(matches!(overlay.error_type, ErrorType::Compilation));
        assert_eq!(overlay.message, "syntax error");
    }
}
