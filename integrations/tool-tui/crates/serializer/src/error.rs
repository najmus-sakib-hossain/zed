//! Comprehensive error types for dx-serializer
//!
//! This module provides a unified error handling system with detailed
//! location information for debugging and user-friendly error messages.

use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, DxError>;

/// Maximum length for error snippets (characters of context around error)
pub const MAX_SNIPPET_LENGTH: usize = 50;

/// Source location information for parse errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset from start of input
    pub offset: usize,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    /// Create a source location from a byte offset and input
    pub fn from_offset(input: &[u8], offset: usize) -> Self {
        let mut line = 1;
        let mut column = 1;

        for (i, &byte) in input.iter().enumerate() {
            if i >= offset {
                break;
            }
            if byte == b'\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        Self {
            line,
            column,
            offset,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Extract a snippet of input around the error location
///
/// Returns up to MAX_SNIPPET_LENGTH characters centered around the offset,
/// with invalid UTF-8 bytes replaced with replacement characters.
pub fn extract_snippet(input: &[u8], offset: usize) -> String {
    if input.is_empty() {
        return String::new();
    }

    // Clamp offset to valid range
    let offset = offset.min(input.len().saturating_sub(1));

    // Calculate start and end positions for the snippet
    let half_len = MAX_SNIPPET_LENGTH / 2;
    let start = offset.saturating_sub(half_len);
    let end = (offset + half_len).min(input.len());

    // Extract the slice
    let slice = &input[start..end];

    // Convert to string, replacing invalid UTF-8
    let snippet: String = String::from_utf8_lossy(slice)
        .chars()
        .filter(|c| !c.is_control() || *c == ' ' || *c == '\t')
        .collect();

    // If snippet is empty or only whitespace, show a placeholder
    if snippet.trim().is_empty() && !slice.is_empty() {
        return format!("<{} bytes>", slice.len());
    }

    snippet
}

/// Magic bytes for DX binary format
pub const DX_MAGIC: [u8; 2] = [0x5A, 0x44]; // "ZD" in ASCII

/// Current binary format version
pub const DX_VERSION: u8 = 1;

/// Maximum input size (100 MB) - prevents memory exhaustion attacks
pub const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Maximum recursion depth for nested structures - prevents stack overflow
pub const MAX_RECURSION_DEPTH: usize = 1000;

/// Maximum table row count - prevents memory exhaustion
pub const MAX_TABLE_ROWS: usize = 10_000_000;

/// Comprehensive error type for all DX serializer operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DxError {
    // === Parse Errors ===
    /// Unexpected end of input during parsing
    #[error("Unexpected end of input at position {0}")]
    UnexpectedEof(usize),

    /// Parse error with location information and snippet
    #[error("Parse error at {location}: {message}\n  --> {snippet}")]
    ParseError {
        location: SourceLocation,
        message: String,
        /// Snippet of the problematic input (up to 50 chars)
        snippet: String,
    },

    /// Invalid syntax at a specific position
    #[error("Invalid syntax at position {pos}: {msg}")]
    InvalidSyntax { pos: usize, msg: String },

    // === Schema Errors ===
    /// Schema validation error
    #[error("Schema error: {0}")]
    SchemaError(String),

    /// Type mismatch during parsing or conversion
    #[error("Type mismatch: expected {expected}, found {actual}")]
    TypeMismatch { expected: String, actual: String },

    // === Reference Errors ===
    /// Unknown alias reference
    #[error("Unknown alias: {0}")]
    UnknownAlias(String),

    /// Unknown anchor reference
    #[error("Unknown anchor: {0}")]
    UnknownAnchor(String),

    // === Type Errors ===
    /// Invalid type hint in schema
    #[error("Invalid type hint: {0}")]
    InvalidTypeHint(String),

    /// Invalid number format
    #[error("Invalid number format: {0}")]
    InvalidNumber(String),

    // === Encoding Errors ===
    /// Invalid UTF-8 sequence
    #[error("Invalid UTF-8 at byte offset {offset}")]
    Utf8Error { offset: usize },

    /// Invalid Base62 character
    #[error("Invalid Base62 character '{char}' at position {position}: {message}")]
    Base62Error {
        char: char,
        position: usize,
        message: String,
    },

    /// Integer overflow during encoding/decoding
    #[error("Integer overflow")]
    IntegerOverflow,

    // === Binary Format Errors ===
    /// Invalid magic bytes in binary header
    #[error("Invalid magic bytes: expected [0x5A, 0x44], got [{0:#04X}, {1:#04X}]")]
    InvalidMagic(u8, u8),

    /// Unsupported binary format version
    #[error("Unsupported version {found}, expected {expected}")]
    UnsupportedVersion { found: u8, expected: u8 },

    /// Buffer too small for operation
    #[error("Buffer too small: need {required} bytes, have {available}")]
    BufferTooSmall { required: usize, available: usize },

    // === Compression Errors ===
    /// Compression operation failed
    #[error("Compression error: {0}")]
    CompressionError(String),

    /// Decompression operation failed
    #[error("Decompression error: {0}")]
    DecompressionError(String),

    // === I/O Errors ===
    /// General I/O error (wraps std::io::Error message)
    #[error("IO error: {0}")]
    Io(String),

    /// Platform not supported for operation
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    // === Conversion Errors ===
    /// Format conversion error
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// Ditto operator without previous value
    #[error("Ditto without previous value at position {0}")]
    DittoNoPrevious(usize),

    /// Prefix inheritance failed
    #[error("Prefix inheritance failed: {0}")]
    PrefixError(String),

    // === Resource Limit Errors ===
    /// Input size exceeds maximum allowed
    #[error("Input too large: {size} bytes exceeds maximum of {max} bytes")]
    InputTooLarge { size: usize, max: usize },

    /// Recursion depth exceeds maximum allowed
    #[error("Recursion limit exceeded: depth {depth} exceeds maximum of {max}")]
    RecursionLimitExceeded { depth: usize, max: usize },

    /// Table row count exceeds maximum allowed
    #[error("Table too large: {rows} rows exceeds maximum of {max} rows")]
    TableTooLarge { rows: usize, max: usize },
}

impl DxError {
    /// Create a parse error with location information and snippet
    pub fn parse_error(input: &[u8], offset: usize, message: impl Into<String>) -> Self {
        DxError::ParseError {
            location: SourceLocation::from_offset(input, offset),
            message: message.into(),
            snippet: extract_snippet(input, offset),
        }
    }

    /// Create a parse error with explicit location and snippet
    pub fn parse_error_with_location(
        location: SourceLocation,
        message: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        DxError::ParseError {
            location,
            message: message.into(),
            snippet: snippet.into(),
        }
    }

    /// Create a type mismatch error
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        DxError::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a UTF-8 error at a specific offset
    pub fn utf8_error(offset: usize) -> Self {
        DxError::Utf8Error { offset }
    }

    /// Create a Base62 error with position
    pub fn base62_error(char: char, position: usize, message: impl Into<String>) -> Self {
        DxError::Base62Error {
            char,
            position,
            message: message.into(),
        }
    }

    /// Create an invalid magic error
    pub fn invalid_magic(byte0: u8, byte1: u8) -> Self {
        DxError::InvalidMagic(byte0, byte1)
    }

    /// Create an unsupported version error
    pub fn unsupported_version(found: u8) -> Self {
        DxError::UnsupportedVersion {
            found,
            expected: DX_VERSION,
        }
    }

    /// Create a buffer too small error
    pub fn buffer_too_small(required: usize, available: usize) -> Self {
        DxError::BufferTooSmall {
            required,
            available,
        }
    }

    /// Create an input too large error
    pub fn input_too_large(size: usize) -> Self {
        DxError::InputTooLarge {
            size,
            max: MAX_INPUT_SIZE,
        }
    }

    /// Create a recursion limit exceeded error
    pub fn recursion_limit_exceeded(depth: usize) -> Self {
        DxError::RecursionLimitExceeded {
            depth,
            max: MAX_RECURSION_DEPTH,
        }
    }

    /// Create a table too large error
    pub fn table_too_large(rows: usize) -> Self {
        DxError::TableTooLarge {
            rows,
            max: MAX_TABLE_ROWS,
        }
    }

    /// Get the byte offset if available
    pub fn offset(&self) -> Option<usize> {
        match self {
            DxError::UnexpectedEof(offset) => Some(*offset),
            DxError::ParseError { location, .. } => Some(location.offset),
            DxError::InvalidSyntax { pos, .. } => Some(*pos),
            DxError::Utf8Error { offset } => Some(*offset),
            DxError::Base62Error { position, .. } => Some(*position),
            DxError::DittoNoPrevious(pos) => Some(*pos),
            _ => None,
        }
    }

    /// Get the source location if available
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            DxError::ParseError { location, .. } => Some(location),
            _ => None,
        }
    }

    /// Get the snippet if available
    pub fn snippet(&self) -> Option<&str> {
        match self {
            DxError::ParseError { snippet, .. } => Some(snippet),
            _ => None,
        }
    }

    /// Get line number if available (1-indexed)
    pub fn line(&self) -> Option<usize> {
        self.location().map(|loc| loc.line)
    }

    /// Get column number if available (1-indexed)
    pub fn column(&self) -> Option<usize> {
        self.location().map(|loc| loc.column)
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            DxError::UnknownAlias(_) | DxError::UnknownAnchor(_) | DxError::TypeMismatch { .. }
        )
    }
}

impl From<std::io::Error> for DxError {
    fn from(err: std::io::Error) -> Self {
        DxError::Io(err.to_string())
    }
}

impl From<std::str::Utf8Error> for DxError {
    fn from(err: std::str::Utf8Error) -> Self {
        DxError::Utf8Error {
            offset: err.valid_up_to(),
        }
    }
}

impl From<std::string::FromUtf8Error> for DxError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        DxError::Utf8Error {
            offset: err.utf8_error().valid_up_to(),
        }
    }
}

impl From<crate::llm::parser::ParseError> for DxError {
    /// Convert a ParseError from the LLM parser into a DxError.
    ///
    /// This conversion preserves position information where available,
    /// mapping each ParseError variant to the most appropriate DxError variant.
    fn from(err: crate::llm::parser::ParseError) -> Self {
        use crate::llm::parser::ParseError;

        match err {
            ParseError::UnexpectedChar { ch, pos } => DxError::InvalidSyntax {
                pos,
                msg: format!("Unexpected character '{}'", ch),
            },
            ParseError::UnexpectedEof => DxError::UnexpectedEof(0),
            ParseError::InvalidValue { value } => DxError::InvalidSyntax {
                pos: 0,
                msg: format!("Invalid value format: {}", value),
            },
            ParseError::SchemaMismatch { expected, got } => DxError::SchemaError(format!(
                "Schema mismatch: expected {} columns, got {}",
                expected, got
            )),
            ParseError::Utf8Error { offset } => DxError::Utf8Error { offset },
            ParseError::InputTooLarge { size, max } => DxError::InputTooLarge { size, max },
            ParseError::UnclosedBracket { pos } => DxError::InvalidSyntax {
                pos,
                msg: "Unclosed bracket".to_string(),
            },
            ParseError::UnclosedParen { pos } => DxError::InvalidSyntax {
                pos,
                msg: "Unclosed parenthesis".to_string(),
            },
            ParseError::MissingValue { pos } => DxError::InvalidSyntax {
                pos,
                msg: "Missing value after '='".to_string(),
            },
            ParseError::InvalidTable { msg } => {
                DxError::SchemaError(format!("Invalid table format: {}", msg))
            }
        }
    }
}

impl From<crate::llm::convert::ConvertError> for DxError {
    /// Convert a ConvertError into a DxError.
    ///
    /// This conversion handles all ConvertError variants:
    /// - `LlmParse`: Converts the underlying ParseError to DxError
    /// - `HumanParse`: Converts to ConversionError with the error message
    /// - `MachineFormat`: Converts to ConversionError with the error message
    fn from(err: crate::llm::convert::ConvertError) -> Self {
        use crate::llm::convert::ConvertError;

        match err {
            ConvertError::LlmParse(parse_err) => parse_err.into(),
            ConvertError::HumanParse(human_err) => DxError::ConversionError(human_err.to_string()),
            ConvertError::MachineFormat { msg } => {
                DxError::ConversionError(format!("Machine format error: {}", msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_from_offset() {
        let input = b"line1\nline2\nline3";

        // First line
        let loc = SourceLocation::from_offset(input, 0);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);

        // Middle of first line
        let loc = SourceLocation::from_offset(input, 3);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 4);

        // Start of second line
        let loc = SourceLocation::from_offset(input, 6);
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 1);

        // Middle of third line
        let loc = SourceLocation::from_offset(input, 14);
        assert_eq!(loc.line, 3);
        assert_eq!(loc.column, 3);
    }

    #[test]
    fn test_parse_error_with_location() {
        let input = b"key: value\nbad line here";
        let err = DxError::parse_error(input, 15, "unexpected token");

        if let DxError::ParseError {
            location,
            message,
            snippet,
        } = &err
        {
            assert_eq!(location.line, 2);
            assert_eq!(location.column, 5);
            assert_eq!(message, "unexpected token");
            assert!(!snippet.is_empty());
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_parse_error_snippet() {
        let input = b"key: value\nbad line here with more content";
        let err = DxError::parse_error(input, 15, "unexpected token");

        let snippet = err.snippet().unwrap();
        assert!(!snippet.is_empty());
        assert!(snippet.len() <= MAX_SNIPPET_LENGTH);
    }

    #[test]
    fn test_extract_snippet() {
        let input = b"hello world this is a test";
        let snippet = extract_snippet(input, 6);
        assert!(!snippet.is_empty());
        assert!(snippet.contains("world"));
    }

    #[test]
    fn test_extract_snippet_empty_input() {
        let input = b"";
        let snippet = extract_snippet(input, 0);
        assert!(snippet.is_empty());
    }

    #[test]
    fn test_type_mismatch_error() {
        let err = DxError::type_mismatch("int", "string");
        let msg = err.to_string();
        assert!(msg.contains("expected int"));
        assert!(msg.contains("found string"));
    }

    #[test]
    fn test_invalid_magic() {
        let err = DxError::invalid_magic(0x00, 0x01);
        assert!(err.to_string().contains("0x00"));
        assert!(err.to_string().contains("0x01"));
    }

    #[test]
    fn test_buffer_too_small() {
        let err = DxError::buffer_too_small(100, 50);
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_error_offset() {
        assert_eq!(DxError::UnexpectedEof(42).offset(), Some(42));
        assert_eq!(DxError::utf8_error(10).offset(), Some(10));
        assert_eq!(DxError::SchemaError("test".into()).offset(), None);
    }

    #[test]
    fn test_error_line_column() {
        let input = b"line1\nline2\nline3";
        let err = DxError::parse_error(input, 8, "test");

        assert_eq!(err.line(), Some(2));
        assert_eq!(err.column(), Some(3));
    }

    #[test]
    fn test_error_line_column_none() {
        let err = DxError::SchemaError("test".into());
        assert_eq!(err.line(), None);
        assert_eq!(err.column(), None);
    }

    #[test]
    fn test_std_error_implementation() {
        // Verify that DxError implements std::error::Error
        fn assert_error<E: std::error::Error>(_: &E) {}

        let err = DxError::parse_error(b"test input", 5, "test error");
        assert_error(&err);

        let err = DxError::type_mismatch("int", "string");
        assert_error(&err);

        let err = DxError::utf8_error(10);
        assert_error(&err);

        let err = DxError::Io("file not found".to_string());
        assert_error(&err);
    }

    #[test]
    fn test_error_display_is_actionable() {
        // Parse error should include location and snippet
        let input = b"key: value\nbad line here";
        let err = DxError::parse_error(input, 15, "unexpected token");
        let msg = err.to_string();
        assert!(msg.contains("line"));
        assert!(msg.contains("column"));
        assert!(msg.contains("unexpected token"));

        // Type mismatch should include both types
        let err = DxError::type_mismatch("integer", "string");
        let msg = err.to_string();
        assert!(msg.contains("integer"));
        assert!(msg.contains("string"));

        // Buffer error should include sizes
        let err = DxError::buffer_too_small(100, 50);
        let msg = err.to_string();
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));

        // Security limit errors should include limits
        let err = DxError::input_too_large(200_000_000);
        let msg = err.to_string();
        assert!(msg.contains("200000000"));
        assert!(msg.contains(&MAX_INPUT_SIZE.to_string()));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let dx_err: DxError = io_err.into();
        match dx_err {
            DxError::Io(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_error_from_utf8() {
        // Create invalid UTF-8
        let invalid = vec![0xFF, 0xFE];
        let result = std::str::from_utf8(&invalid);
        if let Err(utf8_err) = result {
            let dx_err: DxError = utf8_err.into();
            match dx_err {
                DxError::Utf8Error { offset } => assert_eq!(offset, 0),
                _ => panic!("Expected Utf8Error"),
            }
        }
    }

    #[test]
    fn test_error_from_parse_error_unexpected_char() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::UnexpectedChar { ch: '@', pos: 42 };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::InvalidSyntax { pos, msg } => {
                assert_eq!(pos, 42);
                assert!(msg.contains('@'));
            }
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_unexpected_eof() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::UnexpectedEof;
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::UnexpectedEof(pos) => assert_eq!(pos, 0),
            _ => panic!("Expected UnexpectedEof error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_utf8() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::Utf8Error { offset: 123 };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::Utf8Error { offset } => assert_eq!(offset, 123),
            _ => panic!("Expected Utf8Error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_input_too_large() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::InputTooLarge {
            size: 200_000_000,
            max: 100_000_000,
        };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::InputTooLarge { size, max } => {
                assert_eq!(size, 200_000_000);
                assert_eq!(max, 100_000_000);
            }
            _ => panic!("Expected InputTooLarge error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_schema_mismatch() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::SchemaMismatch {
            expected: 5,
            got: 3,
        };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::SchemaError(msg) => {
                assert!(msg.contains("5"));
                assert!(msg.contains("3"));
            }
            _ => panic!("Expected SchemaError"),
        }
    }

    #[test]
    fn test_error_from_parse_error_unclosed_bracket() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::UnclosedBracket { pos: 10 };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::InvalidSyntax { pos, msg } => {
                assert_eq!(pos, 10);
                assert!(msg.contains("bracket"));
            }
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_unclosed_paren() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::UnclosedParen { pos: 20 };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::InvalidSyntax { pos, msg } => {
                assert_eq!(pos, 20);
                assert!(msg.contains("parenthesis"));
            }
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_missing_value() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::MissingValue { pos: 15 };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::InvalidSyntax { pos, msg } => {
                assert_eq!(pos, 15);
                assert!(msg.contains("Missing value"));
            }
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_invalid_table() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::InvalidTable {
            msg: "Empty schema".to_string(),
        };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::SchemaError(msg) => {
                assert!(msg.contains("Empty schema"));
            }
            _ => panic!("Expected SchemaError"),
        }
    }

    #[test]
    fn test_error_from_parse_error_invalid_value() {
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::InvalidValue {
            value: "bad_value".to_string(),
        };
        let dx_err: DxError = parse_err.into();

        match dx_err {
            DxError::InvalidSyntax { pos, msg } => {
                assert_eq!(pos, 0);
                assert!(msg.contains("bad_value"));
            }
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_error_from_parse_error_preserves_position() {
        use crate::llm::parser::ParseError;

        // Test that position information is preserved for all variants that have it
        let test_cases: Vec<(ParseError, Option<usize>)> = vec![
            (ParseError::UnexpectedChar { ch: 'x', pos: 100 }, Some(100)),
            (ParseError::UnclosedBracket { pos: 200 }, Some(200)),
            (ParseError::UnclosedParen { pos: 300 }, Some(300)),
            (ParseError::MissingValue { pos: 400 }, Some(400)),
            (ParseError::Utf8Error { offset: 500 }, Some(500)),
        ];

        for (parse_err, expected_pos) in test_cases {
            let dx_err: DxError = parse_err.into();
            assert_eq!(
                dx_err.offset(),
                expected_pos,
                "Position not preserved for error: {:?}",
                dx_err
            );
        }
    }

    // Tests for From<ConvertError> for DxError

    #[test]
    fn test_error_from_convert_error_llm_parse() {
        use crate::llm::convert::ConvertError;
        use crate::llm::parser::ParseError;

        // ConvertError::LlmParse should delegate to the ParseError conversion
        let parse_err = ParseError::UnexpectedChar { ch: '#', pos: 50 };
        let convert_err = ConvertError::LlmParse(parse_err);
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::InvalidSyntax { pos, msg } => {
                assert_eq!(pos, 50);
                assert!(msg.contains('#'));
            }
            _ => panic!("Expected InvalidSyntax error, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_llm_parse_eof() {
        use crate::llm::convert::ConvertError;
        use crate::llm::parser::ParseError;

        let parse_err = ParseError::UnexpectedEof;
        let convert_err = ConvertError::LlmParse(parse_err);
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::UnexpectedEof(pos) => assert_eq!(pos, 0),
            _ => panic!("Expected UnexpectedEof error, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_human_parse() {
        use crate::llm::convert::ConvertError;
        use crate::llm::human_parser::HumanParseError;

        let human_err = HumanParseError::InvalidSectionHeader {
            msg: "Missing closing bracket".to_string(),
        };
        let convert_err = ConvertError::HumanParse(human_err);
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::ConversionError(msg) => {
                assert!(msg.contains("Invalid section header"));
                assert!(msg.contains("Missing closing bracket"));
            }
            _ => panic!("Expected ConversionError, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_human_parse_invalid_key_value() {
        use crate::llm::convert::ConvertError;
        use crate::llm::human_parser::HumanParseError;

        let human_err = HumanParseError::InvalidKeyValue {
            msg: "No equals sign found".to_string(),
        };
        let convert_err = ConvertError::HumanParse(human_err);
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::ConversionError(msg) => {
                assert!(msg.contains("Invalid key-value pair"));
                assert!(msg.contains("No equals sign found"));
            }
            _ => panic!("Expected ConversionError, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_human_parse_invalid_table() {
        use crate::llm::convert::ConvertError;
        use crate::llm::human_parser::HumanParseError;

        let human_err = HumanParseError::InvalidTable {
            line: 42,
            msg: "Mismatched columns".to_string(),
        };
        let convert_err = ConvertError::HumanParse(human_err);
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::ConversionError(msg) => {
                assert!(msg.contains("Invalid table format"));
                assert!(msg.contains("42"));
                assert!(msg.contains("Mismatched columns"));
            }
            _ => panic!("Expected ConversionError, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_machine_format() {
        use crate::llm::convert::ConvertError;

        let convert_err = ConvertError::MachineFormat {
            msg: "Invalid magic number".to_string(),
        };
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::ConversionError(msg) => {
                assert!(msg.contains("Machine format error"));
                assert!(msg.contains("Invalid magic number"));
            }
            _ => panic!("Expected ConversionError, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_machine_format_eof() {
        use crate::llm::convert::ConvertError;

        let convert_err = ConvertError::MachineFormat {
            msg: "Unexpected end of data".to_string(),
        };
        let dx_err: DxError = convert_err.into();

        match dx_err {
            DxError::ConversionError(msg) => {
                assert!(msg.contains("Machine format error"));
                assert!(msg.contains("Unexpected end of data"));
            }
            _ => panic!("Expected ConversionError, got {:?}", dx_err),
        }
    }

    #[test]
    fn test_error_from_convert_error_all_variants_convertible() {
        use crate::llm::convert::ConvertError;
        use crate::llm::human_parser::HumanParseError;
        use crate::llm::parser::ParseError;

        // Test that all ConvertError variants can be converted to DxError
        let test_cases: Vec<ConvertError> = vec![
            ConvertError::LlmParse(ParseError::UnexpectedEof),
            ConvertError::LlmParse(ParseError::UnexpectedChar { ch: 'x', pos: 0 }),
            ConvertError::LlmParse(ParseError::InvalidValue {
                value: "test".to_string(),
            }),
            ConvertError::LlmParse(ParseError::SchemaMismatch {
                expected: 3,
                got: 2,
            }),
            ConvertError::LlmParse(ParseError::Utf8Error { offset: 10 }),
            ConvertError::LlmParse(ParseError::InputTooLarge {
                size: 200,
                max: 100,
            }),
            ConvertError::LlmParse(ParseError::UnclosedBracket { pos: 5 }),
            ConvertError::LlmParse(ParseError::UnclosedParen { pos: 6 }),
            ConvertError::LlmParse(ParseError::MissingValue { pos: 7 }),
            ConvertError::LlmParse(ParseError::InvalidTable {
                msg: "test".to_string(),
            }),
            ConvertError::HumanParse(HumanParseError::InvalidSectionHeader {
                msg: "test".to_string(),
            }),
            ConvertError::HumanParse(HumanParseError::InvalidKeyValue {
                msg: "test".to_string(),
            }),
            ConvertError::HumanParse(HumanParseError::InvalidTable {
                line: 1,
                msg: "test".to_string(),
            }),
            ConvertError::HumanParse(HumanParseError::UnexpectedContent {
                msg: "test".to_string(),
            }),
            ConvertError::HumanParse(HumanParseError::InputTooLarge {
                size: 200,
                max: 100,
            }),
            ConvertError::HumanParse(HumanParseError::TableTooLarge {
                rows: 200,
                max: 100,
            }),
            ConvertError::MachineFormat {
                msg: "test".to_string(),
            },
        ];

        for convert_err in test_cases {
            let dx_err: DxError = convert_err.into();
            // Just verify conversion doesn't panic and produces a non-empty error message
            let msg = dx_err.to_string();
            assert!(!msg.is_empty(), "Error message should not be empty");
        }
    }
}
