//! Core Traits for Tool Adapters
//!
//! Defines the `ToolAdapter` trait that all third-party tool integrations must implement.

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::fmt;
use std::path::{Path, PathBuf};

/// Capability flags for a tool adapter
#[derive(Debug, Clone, Copy, Default)]
pub struct ToolCapabilities {
    /// Can format source code
    pub can_format: bool,
    /// Can lint/analyze source code
    pub can_lint: bool,
    /// Can auto-fix issues
    pub can_fix: bool,
    /// Supports reading from stdin
    pub supports_stdin: bool,
    /// Supports configuration files
    pub supports_config: bool,
    /// Supports JSON output
    pub supports_json_output: bool,
    /// Supports caching
    pub supports_caching: bool,
}

impl ToolCapabilities {
    /// Create capabilities for a formatter-only tool
    #[must_use]
    pub fn formatter() -> Self {
        Self {
            can_format: true,
            can_lint: false,
            can_fix: false,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: false,
            supports_caching: false,
        }
    }

    /// Create capabilities for a linter-only tool
    #[must_use]
    pub fn linter() -> Self {
        Self {
            can_format: false,
            can_lint: true,
            can_fix: false,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: true,
            supports_caching: false,
        }
    }

    /// Create capabilities for a full-featured tool (format + lint + fix)
    #[must_use]
    pub fn full() -> Self {
        Self {
            can_format: true,
            can_lint: true,
            can_fix: true,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: true,
            supports_caching: true,
        }
    }
}

/// Result of a tool operation
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// List of diagnostics (lint errors, warnings, etc.)
    pub diagnostics: Vec<Diagnostic>,
    /// Formatted content (if formatting was performed)
    pub formatted_content: Option<Vec<u8>>,
    /// Exit code from the tool
    pub exit_code: i32,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Tool name that produced this result
    pub tool_name: String,
    /// Whether changes were made (for formatting)
    pub changed: bool,
}

impl ToolResult {
    /// Create an empty successful result
    pub fn success(tool_name: impl Into<String>) -> Self {
        Self {
            diagnostics: Vec::new(),
            formatted_content: None,
            exit_code: 0,
            duration_ms: 0,
            tool_name: tool_name.into(),
            changed: false,
        }
    }

    /// Create a result with diagnostics
    pub fn with_diagnostics(tool_name: impl Into<String>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics,
            formatted_content: None,
            exit_code: 0,
            duration_ms: 0,
            tool_name: tool_name.into(),
            changed: false,
        }
    }

    /// Create a result with formatted content
    pub fn with_formatted(tool_name: impl Into<String>, content: Vec<u8>, changed: bool) -> Self {
        Self {
            diagnostics: Vec::new(),
            formatted_content: Some(content),
            exit_code: 0,
            duration_ms: 0,
            tool_name: tool_name.into(),
            changed,
        }
    }

    /// Set the duration
    pub fn set_duration(&mut self, ms: u64) {
        self.duration_ms = ms;
    }

    /// Add a diagnostic
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Check if there are any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Check if there are any warnings or errors
    #[must_use]
    pub fn has_issues(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}

/// Error types for tool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorKind {
    /// Tool is not installed or not found
    NotFound,
    /// Tool execution failed
    ExecutionFailed,
    /// Tool output parsing failed
    ParseError,
    /// Tool configuration error
    ConfigError,
    /// Timeout while waiting for tool
    Timeout,
    /// Unsupported language/file type
    UnsupportedLanguage,
    /// I/O error
    IoError,
    /// Internal error
    Internal,
}

/// Error from tool operations
#[derive(Debug, Clone)]
pub struct ToolError {
    /// Error kind
    pub kind: ToolErrorKind,
    /// Error message
    pub message: String,
    /// Tool name (if known)
    pub tool: Option<String>,
    /// Source file (if relevant)
    pub file: Option<PathBuf>,
    /// Underlying error message
    pub cause: Option<String>,
}

impl ToolError {
    /// Create a new tool error
    pub fn new(kind: ToolErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            tool: None,
            file: None,
            cause: None,
        }
    }

    /// Set the tool name
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    /// Set the file path
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set the underlying cause
    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    /// Create a "not found" error
    pub fn not_found(tool: impl Into<String>) -> Self {
        let tool_name = tool.into();
        Self::new(
            ToolErrorKind::NotFound,
            format!("Tool '{tool_name}' not found in PATH or configured locations"),
        )
        .with_tool(tool_name)
    }

    /// Create an execution error
    pub fn execution_failed(tool: impl Into<String>, message: impl Into<String>) -> Self {
        let tool_name = tool.into();
        Self::new(ToolErrorKind::ExecutionFailed, message).with_tool(tool_name)
    }

    /// Create a parse error
    pub fn parse_error(tool: impl Into<String>, message: impl Into<String>) -> Self {
        let tool_name = tool.into();
        Self::new(ToolErrorKind::ParseError, message).with_tool(tool_name)
    }

    /// Convert to a Diagnostic for unified error reporting
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            file: self.file.clone().unwrap_or_default(),
            span: Span::new(0, 0),
            severity: DiagnosticSeverity::Error,
            rule_id: format!("tool/{}", self.tool.as_deref().unwrap_or("unknown")),
            message: self.message.clone(),
            suggestion: self.cause.clone(),
            related: Vec::new(),
            fix: None,
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(tool) = &self.tool {
            write!(f, "[{tool}] ")?;
        }
        write!(f, "{}", self.message)?;
        if let Some(cause) = &self.cause {
            write!(f, ": {cause}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ToolError {}

/// Adapter trait for third-party tool integration
///
/// All external tools (formatters, linters, analyzers) implement this trait.
/// This enables plug-and-play tool registration without modifying core code.
pub trait ToolAdapter: Send + Sync {
    /// Returns the tool name (e.g., "rustfmt", "ruff", "prettier")
    fn name(&self) -> &'static str;

    /// Returns supported file extensions (without the dot)
    fn extensions(&self) -> &[&'static str];

    /// Returns tool capabilities
    fn capabilities(&self) -> ToolCapabilities;

    /// Format a file, returning formatted content
    ///
    /// # Arguments
    /// * `path` - Path to the file (for error reporting)
    /// * `content` - File content as bytes
    ///
    /// # Returns
    /// * `Ok(ToolResult)` - Result with formatted content
    /// * `Err(ToolError)` - If formatting failed
    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError>;

    /// Lint a file, returning diagnostics
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - File content as bytes
    ///
    /// # Returns
    /// * `Ok(ToolResult)` - Result with diagnostics
    /// * `Err(ToolError)` - If linting failed
    fn lint(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError>;

    /// Check if the tool is available on this system
    fn is_available(&self) -> bool;

    /// Get the tool version (if available)
    fn version(&self) -> Option<String>;

    /// Get the path to the tool executable (if found)
    fn executable_path(&self) -> Option<PathBuf>;

    /// Fix issues in a file (if supported)
    ///
    /// Default implementation returns unsupported error.
    fn fix(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        if !self.capabilities().can_fix {
            return Err(ToolError::new(
                ToolErrorKind::UnsupportedLanguage,
                format!("{} does not support auto-fix", self.name()),
            ));
        }
        // Default: format is the fix
        self.format(path, content)
    }

    /// Get installation instructions for this tool
    fn install_instructions(&self) -> &'static str {
        "Please install the tool manually"
    }
}

/// Output types from external tools
#[derive(Debug, Clone)]
pub enum AdapterOutput {
    /// Plain text output
    Text(String),
    /// JSON output (already parsed)
    Json(serde_json::Value),
    /// Binary output
    Binary(Vec<u8>),
}

/// Trait for parsing tool output into diagnostics
pub trait OutputParser: Send + Sync {
    /// Parse the output into diagnostics
    fn parse(&self, output: &AdapterOutput, file: &Path) -> Result<Vec<Diagnostic>, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_capabilities() {
        let formatter = ToolCapabilities::formatter();
        assert!(formatter.can_format);
        assert!(!formatter.can_lint);

        let linter = ToolCapabilities::linter();
        assert!(!linter.can_format);
        assert!(linter.can_lint);

        let full = ToolCapabilities::full();
        assert!(full.can_format);
        assert!(full.can_lint);
        assert!(full.can_fix);
    }

    #[test]
    fn test_tool_result() {
        let result = ToolResult::success("test");
        assert!(!result.has_errors());
        assert!(!result.has_issues());
        assert_eq!(result.tool_name, "test");
    }

    #[test]
    fn test_tool_error() {
        let err = ToolError::not_found("rustfmt");
        assert_eq!(err.kind, ToolErrorKind::NotFound);
        assert!(err.message.contains("rustfmt"));
    }
}
