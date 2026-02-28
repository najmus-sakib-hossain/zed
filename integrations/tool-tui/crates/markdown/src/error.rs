//! Error types for DX Markdown parsing and conversion.
//!
//! This module provides detailed error types with location information
//! and suggestions for fixing issues.

use std::fmt;
use thiserror::Error;

/// Parse error with location information.
///
/// Provides detailed context about where and why parsing failed,
/// including suggestions for fixing the issue.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Error message describing what went wrong
    pub message: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Snippet of the problematic content
    pub snippet: String,
    /// Suggestions for fixing the error
    pub suggestions: Vec<String>,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
            snippet: String::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add a snippet of the problematic content.
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = snippet.into();
        self
    }

    /// Add a suggestion for fixing the error.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Add multiple suggestions.
    pub fn with_suggestions(mut self, suggestions: impl IntoIterator<Item = String>) -> Self {
        self.suggestions.extend(suggestions);
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)?;
        if !self.snippet.is_empty() {
            write!(f, "\n  | {}", self.snippet)?;
        }
        for suggestion in &self.suggestions {
            write!(f, "\n  hint: {}", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Specific parse error variants.
#[derive(Debug, Clone, Error)]
pub enum ParseErrorKind {
    /// Invalid syntax at location
    #[error("invalid syntax: {message}")]
    InvalidSyntax {
        message: String,
        line: usize,
        column: usize,
        snippet: String,
    },

    /// Undefined reference
    #[error("undefined reference '{key}' at line {line}")]
    UndefinedReference {
        key: String,
        line: usize,
        defined_refs: Vec<String>,
    },

    /// Invalid header level
    #[error("invalid header level {level} at line {line} (must be 1-6)")]
    InvalidHeaderLevel { level: u8, line: usize },

    /// Table column mismatch
    #[error("table row has {actual} columns but schema defines {expected} at line {line}")]
    TableColumnMismatch {
        expected: usize,
        actual: usize,
        line: usize,
    },

    /// Invalid UTF-8
    #[error("invalid UTF-8 sequence at byte position {position}")]
    InvalidUtf8 { position: usize },

    /// Input too large
    #[error("input size {size} bytes exceeds maximum {max} bytes")]
    InputTooLarge { size: usize, max: usize },

    /// Recursion limit exceeded
    #[error("recursion depth {depth} exceeds maximum {max}")]
    RecursionLimitExceeded { depth: usize, max: usize },

    /// Unexpected end of input
    #[error("unexpected end of input at line {line}")]
    UnexpectedEof { line: usize },

    /// Invalid escape sequence
    #[error("invalid escape sequence '\\{char}' at line {line}")]
    InvalidEscape { char: char, line: usize },
}

impl ParseErrorKind {
    /// Convert to a ParseError with full context.
    pub fn into_parse_error(self) -> ParseError {
        match self {
            Self::InvalidSyntax {
                message,
                line,
                column,
                snippet,
            } => ParseError::new(message, line, column).with_snippet(snippet),

            Self::UndefinedReference {
                key,
                line,
                defined_refs,
            } => {
                let mut err = ParseError::new(format!("undefined reference '{key}'"), line, 1);
                if !defined_refs.is_empty() {
                    err = err.with_suggestion(format!(
                        "defined references: {}",
                        defined_refs.join(", ")
                    ));
                }
                err
            }

            Self::InvalidHeaderLevel { level, line } => {
                ParseError::new(format!("invalid header level {level} (must be 1-6)"), line, 1)
                    .with_suggestion("use levels 1-6 (e.g., '1|Title' or '2|Section')")
            }

            Self::TableColumnMismatch {
                expected,
                actual,
                line,
            } => ParseError::new(
                format!("table row has {actual} columns but schema defines {expected}"),
                line,
                1,
            )
            .with_suggestion(format!("ensure each row has exactly {expected} columns")),

            Self::InvalidUtf8 { position } => {
                ParseError::new("invalid UTF-8 sequence", 1, position)
                    .with_suggestion("ensure the file is saved with UTF-8 encoding")
            }

            Self::InputTooLarge { size, max } => ParseError::new(
                format!("input size {size} bytes exceeds maximum {max} bytes"),
                1,
                1,
            )
            .with_suggestion("split the document into smaller files"),

            Self::RecursionLimitExceeded { depth, max } => {
                ParseError::new(format!("recursion depth {depth} exceeds maximum {max}"), 1, 1)
                    .with_suggestion("reduce nesting depth in the document")
            }

            Self::UnexpectedEof { line } => ParseError::new("unexpected end of input", line, 1)
                .with_suggestion("check for unclosed blocks or missing content"),

            Self::InvalidEscape { char, line } => {
                ParseError::new(format!("invalid escape sequence '\\{char}'"), line, 1)
                    .with_suggestion("valid escapes: \\!, \\/, \\~, \\@, \\^, \\#, \\\\")
            }
        }
    }
}

/// Conversion error for format transformations.
#[derive(Debug, Clone, Error)]
pub enum ConvertError {
    /// Parse error during conversion
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    /// Unsupported Markdown feature
    #[error("unsupported feature '{feature}': {suggestion}")]
    UnsupportedFeature { feature: String, suggestion: String },

    /// Binary format error
    #[error("binary format error: {message}")]
    BinaryError { message: String },

    /// Invalid format specification
    #[error("invalid format: {0}")]
    InvalidFormat(String),

    /// Invalid UTF-8 encoding
    #[error("invalid UTF-8: {0}")]
    InvalidUtf8(String),

    /// Unknown format
    #[error("unknown format: could not detect input format")]
    UnknownFormat,

    /// Generic parse error string
    #[error("parse error: {0}")]
    ParseError(String),
}

impl ConvertError {
    /// Create an unsupported feature error.
    pub fn unsupported(feature: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::UnsupportedFeature {
            feature: feature.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create a binary error.
    pub fn binary(message: impl Into<String>) -> Self {
        Self::BinaryError {
            message: message.into(),
        }
    }
}

/// Binary format error.
#[derive(Debug, Clone, Error)]
pub enum BinaryError {
    /// Invalid magic number
    #[error("invalid magic number: expected 'DXMB', got {0:?}")]
    InvalidMagic([u8; 4]),

    /// Unsupported version
    #[error("unsupported binary format version {0}")]
    UnsupportedVersion(u16),

    /// Corrupted data
    #[error("corrupted binary data: {0}")]
    CorruptedData(String),

    /// Buffer too small
    #[error("buffer too small: need {needed} bytes, have {available}")]
    BufferTooSmall { needed: usize, available: usize },

    /// Invalid offset
    #[error("invalid offset {offset} (max: {max})")]
    InvalidOffset { offset: usize, max: usize },

    /// Invalid node type
    #[error("invalid node type tag: {0}")]
    InvalidNodeType(u8),

    /// Invalid format
    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

impl From<BinaryError> for ConvertError {
    fn from(err: BinaryError) -> Self {
        Self::BinaryError {
            message: err.to_string(),
        }
    }
}

/// Result type alias for parse operations.
pub type ParseResult<T> = Result<T, ParseError>;

/// Result type alias for conversion operations.
pub type ConvertResult<T> = Result<T, ConvertError>;

/// Result type alias for binary operations.
pub type BinaryResult<T> = Result<T, BinaryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::new("unexpected token", 10, 5)
            .with_snippet("let x = @invalid")
            .with_suggestion("did you mean 'let x = value'?");

        let display = err.to_string();
        assert!(display.contains("10:5"));
        assert!(display.contains("unexpected token"));
        assert!(display.contains("@invalid"));
        assert!(display.contains("hint:"));
    }

    #[test]
    fn test_undefined_reference_error() {
        let err = ParseErrorKind::UndefinedReference {
            key: "foo".to_string(),
            line: 5,
            defined_refs: vec!["bar".to_string(), "baz".to_string()],
        }
        .into_parse_error();

        let display = err.to_string();
        assert!(display.contains("undefined reference 'foo'"));
        assert!(display.contains("bar, baz"));
    }

    #[test]
    fn test_binary_error() {
        let err = BinaryError::InvalidMagic([0x00, 0x01, 0x02, 0x03]);
        assert!(err.to_string().contains("invalid magic number"));
    }

    #[test]
    fn test_convert_error_from_parse() {
        let parse_err = ParseError::new("test error", 1, 1);
        let convert_err: ConvertError = parse_err.into();
        assert!(matches!(convert_err, ConvertError::Parse(_)));
    }
}

// =============================================================================
// Context Compiler Error Types
// =============================================================================

/// Compilation error for the context compiler.
#[derive(Debug, Clone, Error)]
pub enum CompileError {
    /// Input too large
    #[error("input size {size} bytes exceeds maximum {max} bytes")]
    InputTooLarge { size: usize, max: usize },

    /// Invalid UTF-8 encoding
    #[error("invalid UTF-8 at byte position {position}")]
    InvalidUtf8 { position: usize },

    /// IO error (for streaming)
    #[error("IO error: {message}")]
    Io { message: String },

    /// Git error
    #[error("git error: {message}")]
    Git { message: String },

    /// Tokenizer error
    #[error("tokenizer error: {message}")]
    Tokenizer { message: String },

    /// Recursion limit exceeded
    #[error("recursion depth {depth} exceeds maximum {max}")]
    RecursionLimit { depth: usize, max: usize },

    /// Code minification error
    #[error("code minification error for {language}: {message}")]
    Minification { language: String, message: String },
}

impl CompileError {
    /// Create an input too large error.
    pub fn input_too_large(size: usize, max: usize) -> Self {
        Self::InputTooLarge { size, max }
    }

    /// Create an invalid UTF-8 error.
    pub fn invalid_utf8(position: usize) -> Self {
        Self::InvalidUtf8 { position }
    }

    /// Create an IO error.
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// Create a git error.
    pub fn git(message: impl Into<String>) -> Self {
        Self::Git {
            message: message.into(),
        }
    }

    /// Create a tokenizer error.
    pub fn tokenizer(message: impl Into<String>) -> Self {
        Self::Tokenizer {
            message: message.into(),
        }
    }

    /// Create a recursion limit error.
    pub fn recursion_limit(depth: usize, max: usize) -> Self {
        Self::RecursionLimit { depth, max }
    }

    /// Create a minification error.
    pub fn minification(language: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Minification {
            language: language.into(),
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for CompileError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
        }
    }
}

impl From<std::str::Utf8Error> for CompileError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::InvalidUtf8 {
            position: err.valid_up_to(),
        }
    }
}

/// Result type alias for compilation operations.
pub type CompilerResult<T> = Result<T, CompileError>;

#[cfg(test)]
mod compile_error_tests {
    use super::*;

    #[test]
    fn test_input_too_large_error() {
        let err = CompileError::input_too_large(200_000_000, 100_000_000);
        assert!(err.to_string().contains("200000000"));
        assert!(err.to_string().contains("100000000"));
    }

    #[test]
    fn test_invalid_utf8_error() {
        let err = CompileError::invalid_utf8(42);
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let compile_err: CompileError = io_err.into();
        assert!(matches!(compile_err, CompileError::Io { .. }));
    }

    #[test]
    fn test_minification_error() {
        let err = CompileError::minification("javascript", "syntax error");
        assert!(err.to_string().contains("javascript"));
        assert!(err.to_string().contains("syntax error"));
    }
}
