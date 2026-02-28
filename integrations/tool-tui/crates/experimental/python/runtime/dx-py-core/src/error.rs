//! Runtime Error Types
//!
//! This module defines all error types for the DX-Py runtime.
//! All public functions return Result types instead of panicking.

use thiserror::Error;

/// Source location information for error context
#[derive(Debug, Clone, Default)]
pub struct SourceLocation {
    /// File path where the error occurred
    pub file: Option<String>,
    /// Line number (1-indexed)
    pub line: Option<usize>,
    /// Column number (1-indexed)
    pub column: Option<usize>,
    /// Function or method name
    pub function: Option<String>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the file path
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set the line number
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set the function name
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }

    /// Check if any location information is present
    pub fn has_info(&self) -> bool {
        self.file.is_some() || self.line.is_some() || self.function.is_some()
    }

    /// Format the location as a string
    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref file) = self.file {
            parts.push(format!("File \"{}\"", file));
        }

        if let Some(line) = self.line {
            if let Some(col) = self.column {
                parts.push(format!("line {}, column {}", line, col));
            } else {
                parts.push(format!("line {}", line));
            }
        }

        if let Some(ref func) = self.function {
            parts.push(format!("in {}", func));
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join(", ")
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Error context with suggestions for fixing the error
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// Source location where the error occurred
    pub location: SourceLocation,
    /// Additional context message
    pub context: Option<String>,
    /// Suggestions for fixing the error
    pub suggestions: Vec<String>,
    /// Related notes
    pub notes: Vec<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }

    /// Set the context message
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Check if any context information is present
    pub fn has_info(&self) -> bool {
        self.location.has_info()
            || self.context.is_some()
            || !self.suggestions.is_empty()
            || !self.notes.is_empty()
    }

    /// Format the context as a string
    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        if self.location.has_info() {
            parts.push(self.location.format());
        }

        if let Some(ref ctx) = self.context {
            parts.push(format!("Context: {}", ctx));
        }

        for note in &self.notes {
            parts.push(format!("Note: {}", note));
        }

        for (i, suggestion) in self.suggestions.iter().enumerate() {
            if self.suggestions.len() == 1 {
                parts.push(format!("Suggestion: {}", suggestion));
            } else {
                parts.push(format!("Suggestion {}: {}", i + 1, suggestion));
            }
        }

        parts.join("\n")
    }
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Runtime errors that can occur during execution
#[derive(Error, Debug, Clone)]
pub enum RuntimeError {
    /// Type mismatch error
    #[error("TypeError: expected {expected}, got {actual}")]
    TypeError { expected: String, actual: String },

    /// Index out of bounds
    #[error("IndexError: index {index} out of range for length {length}")]
    IndexError { index: i64, length: usize },

    /// Key not found in dictionary
    #[error("KeyError: {key}")]
    KeyError { key: String },

    /// Division by zero
    #[error("ZeroDivisionError: division by zero")]
    ZeroDivisionError,

    /// Arithmetic overflow
    #[error("OverflowError: {operation} overflow")]
    OverflowError { operation: String },

    /// Name not found in scope
    #[error("NameError: name '{name}' is not defined")]
    NameError { name: String },

    /// Attribute not found on object
    #[error("AttributeError: '{type_name}' object has no attribute '{attr}'")]
    AttributeError { attr: String, type_name: String },

    /// Module import failed
    #[error("ImportError: No module named '{module}'")]
    ImportError { module: String },

    /// Value error (invalid value for operation)
    #[error("ValueError: {message}")]
    ValueError { message: String },

    /// Runtime assertion failed
    #[error("AssertionError: {message}")]
    AssertionError { message: String },

    /// Stop iteration (used internally)
    #[error("StopIteration")]
    StopIteration,

    /// Memory allocation failed
    #[error("MemoryError: {message}")]
    MemoryError { message: String },

    /// Recursion limit exceeded
    #[error("RecursionError: maximum recursion depth exceeded")]
    RecursionError,

    /// I/O error
    #[error("IOError: {message}")]
    IoError { message: String },

    /// OS error
    #[error("OSError: {message}")]
    OsError { message: String },

    /// File not found
    #[error("FileNotFoundError: {path}")]
    FileNotFoundError { path: String },

    /// Permission denied
    #[error("PermissionError: {path}")]
    PermissionError { path: String },

    /// Unicode decode error
    #[error("UnicodeDecodeError: {message}")]
    UnicodeDecodeError { message: String },

    /// Unicode encode error
    #[error("UnicodeEncodeError: {message}")]
    UnicodeEncodeError { message: String },

    /// JSON decode error
    #[error("JSONDecodeError: {message}: line {line} column {column} (char {char_pos})")]
    JSONDecodeError {
        message: String,
        line: usize,
        column: usize,
        char_pos: usize,
    },

    /// Syntax error (for parser)
    #[error("SyntaxError: {message} at line {line}, column {column}")]
    SyntaxError {
        message: String,
        line: usize,
        column: usize,
    },

    /// Internal runtime error
    #[error("InternalError: {message}")]
    InternalError { message: String },

    /// Not implemented
    #[error("NotImplementedError: {feature}")]
    NotImplementedError { feature: String },

    /// Error with additional context (file, line, suggestions)
    #[error("{error}\n{context}")]
    WithContext {
        error: Box<RuntimeError>,
        context: ErrorContext,
    },
}

/// Result type for runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;

impl RuntimeError {
    /// Create a type error
    pub fn type_error(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::TypeError {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create an index error
    pub fn index_error(index: i64, length: usize) -> Self {
        Self::IndexError { index, length }
    }

    /// Create a key error
    pub fn key_error(key: impl Into<String>) -> Self {
        Self::KeyError { key: key.into() }
    }

    /// Create a name error
    pub fn name_error(name: impl Into<String>) -> Self {
        Self::NameError { name: name.into() }
    }

    /// Create a name error with context and suggestions
    pub fn name_error_with_suggestions(
        name: impl Into<String>,
        file: Option<&str>,
        line: Option<usize>,
        similar_names: Vec<String>,
    ) -> Self {
        let name_str = name.into();
        let mut ctx = ErrorContext::new();

        if let Some(f) = file {
            ctx.location = ctx.location.with_file(f);
        }
        if let Some(l) = line {
            ctx.location = ctx.location.with_line(l);
        }

        for similar in similar_names {
            ctx.suggestions.push(format!("Did you mean '{}'?", similar));
        }

        Self::WithContext {
            error: Box::new(Self::NameError { name: name_str }),
            context: ctx,
        }
    }

    /// Create an attribute error
    pub fn attribute_error(type_name: impl Into<String>, attr: impl Into<String>) -> Self {
        Self::AttributeError {
            type_name: type_name.into(),
            attr: attr.into(),
        }
    }

    /// Create an attribute error with suggestions
    pub fn attribute_error_with_suggestions(
        type_name: impl Into<String>,
        attr: impl Into<String>,
        similar_attrs: Vec<String>,
    ) -> Self {
        let mut ctx = ErrorContext::new();
        for similar in similar_attrs {
            ctx.suggestions.push(format!("Did you mean '{}'?", similar));
        }

        Self::WithContext {
            error: Box::new(Self::AttributeError {
                type_name: type_name.into(),
                attr: attr.into(),
            }),
            context: ctx,
        }
    }

    /// Create a value error
    pub fn value_error(message: impl Into<String>) -> Self {
        Self::ValueError {
            message: message.into(),
        }
    }

    /// Create an overflow error
    pub fn overflow_error(operation: impl Into<String>) -> Self {
        Self::OverflowError {
            operation: operation.into(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// Create a not implemented error
    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self::NotImplementedError {
            feature: feature.into(),
        }
    }

    /// Create an import error
    pub fn import_error(module: impl Into<String>) -> Self {
        Self::ImportError {
            module: module.into(),
        }
    }

    /// Create an import error with suggestions
    pub fn import_error_with_suggestions(
        module: impl Into<String>,
        similar_modules: Vec<String>,
    ) -> Self {
        let mut ctx = ErrorContext::new();
        for similar in similar_modules {
            ctx.suggestions.push(format!("Did you mean '{}'?", similar));
        }
        ctx.notes
            .push("Make sure the module is installed and in your PYTHONPATH".to_string());

        Self::WithContext {
            error: Box::new(Self::ImportError {
                module: module.into(),
            }),
            context: ctx,
        }
    }

    /// Create a JSON decode error
    pub fn json_decode_error(
        message: impl Into<String>,
        line: usize,
        column: usize,
        char_pos: usize,
    ) -> Self {
        Self::JSONDecodeError {
            message: message.into(),
            line,
            column,
            char_pos,
        }
    }

    /// Create a JSON decode error from a string and position
    pub fn json_decode_error_at_pos(message: impl Into<String>, s: &str, pos: usize) -> Self {
        // Calculate line and column from position
        let (line, column) = Self::pos_to_line_col(s, pos);
        Self::JSONDecodeError {
            message: message.into(),
            line,
            column,
            char_pos: pos,
        }
    }

    /// Convert a character position to line and column numbers
    fn pos_to_line_col(s: &str, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, c) in s.chars().enumerate() {
            if i >= pos {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Add context to an existing error
    pub fn with_context(self, context: ErrorContext) -> Self {
        Self::WithContext {
            error: Box::new(self),
            context,
        }
    }

    /// Add location information to an existing error
    pub fn with_location(self, file: &str, line: usize) -> Self {
        let location = SourceLocation::new().with_file(file).with_line(line);
        let context = ErrorContext::new().with_location(location);
        self.with_context(context)
    }

    /// Get the Python exception name for this error
    pub fn exception_name(&self) -> &'static str {
        match self {
            Self::TypeError { .. } => "TypeError",
            Self::IndexError { .. } => "IndexError",
            Self::KeyError { .. } => "KeyError",
            Self::ZeroDivisionError => "ZeroDivisionError",
            Self::OverflowError { .. } => "OverflowError",
            Self::NameError { .. } => "NameError",
            Self::AttributeError { .. } => "AttributeError",
            Self::ImportError { .. } => "ImportError",
            Self::ValueError { .. } => "ValueError",
            Self::AssertionError { .. } => "AssertionError",
            Self::StopIteration => "StopIteration",
            Self::MemoryError { .. } => "MemoryError",
            Self::RecursionError => "RecursionError",
            Self::IoError { .. } => "IOError",
            Self::OsError { .. } => "OSError",
            Self::FileNotFoundError { .. } => "FileNotFoundError",
            Self::PermissionError { .. } => "PermissionError",
            Self::UnicodeDecodeError { .. } => "UnicodeDecodeError",
            Self::UnicodeEncodeError { .. } => "UnicodeEncodeError",
            Self::JSONDecodeError { .. } => "JSONDecodeError",
            Self::SyntaxError { .. } => "SyntaxError",
            Self::InternalError { .. } => "RuntimeError",
            Self::NotImplementedError { .. } => "NotImplementedError",
            Self::WithContext { error, .. } => error.exception_name(),
        }
    }

    /// Get the error context if present
    pub fn get_context(&self) -> Option<&ErrorContext> {
        match self {
            Self::WithContext { context, .. } => Some(context),
            _ => None,
        }
    }

    /// Get the inner error (unwrapping WithContext if present)
    pub fn inner(&self) -> &RuntimeError {
        match self {
            Self::WithContext { error, .. } => error.inner(),
            _ => self,
        }
    }
}

impl From<std::io::Error> for RuntimeError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::FileNotFoundError {
                path: err.to_string(),
            },
            std::io::ErrorKind::PermissionDenied => Self::PermissionError {
                path: err.to_string(),
            },
            _ => Self::IoError {
                message: err.to_string(),
            },
        }
    }
}

impl From<std::num::ParseIntError> for RuntimeError {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::ValueError {
            message: format!("invalid literal for int(): {}", err),
        }
    }
}

impl From<std::num::ParseFloatError> for RuntimeError {
    fn from(err: std::num::ParseFloatError) -> Self {
        Self::ValueError {
            message: format!("could not convert string to float: {}", err),
        }
    }
}

impl From<std::str::Utf8Error> for RuntimeError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::UnicodeDecodeError {
            message: err.to_string(),
        }
    }
}

impl From<std::string::FromUtf8Error> for RuntimeError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::UnicodeDecodeError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_error() {
        let err = RuntimeError::type_error("int", "str");
        assert_eq!(err.exception_name(), "TypeError");
        assert!(err.to_string().contains("expected int"));
        assert!(err.to_string().contains("got str"));
    }

    #[test]
    fn test_index_error() {
        let err = RuntimeError::index_error(10, 5);
        assert_eq!(err.exception_name(), "IndexError");
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn test_key_error() {
        let err = RuntimeError::key_error("missing_key");
        assert_eq!(err.exception_name(), "KeyError");
        assert!(err.to_string().contains("missing_key"));
    }

    #[test]
    fn test_name_error() {
        let err = RuntimeError::name_error("undefined_var");
        assert_eq!(err.exception_name(), "NameError");
        assert!(err.to_string().contains("undefined_var"));
    }

    #[test]
    fn test_attribute_error() {
        let err = RuntimeError::attribute_error("int", "foo");
        assert_eq!(err.exception_name(), "AttributeError");
        assert!(err.to_string().contains("int"));
        assert!(err.to_string().contains("foo"));
    }

    #[test]
    fn test_zero_division() {
        let err = RuntimeError::ZeroDivisionError;
        assert_eq!(err.exception_name(), "ZeroDivisionError");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: RuntimeError = io_err.into();
        assert_eq!(err.exception_name(), "FileNotFoundError");
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_err: Result<i64, _> = "not_a_number".parse();
        let err: RuntimeError = parse_err.unwrap_err().into();
        assert_eq!(err.exception_name(), "ValueError");
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new()
            .with_file("test.py")
            .with_line(42)
            .with_column(10)
            .with_function("my_function");

        assert!(loc.has_info());
        let formatted = loc.format();
        assert!(formatted.contains("test.py"));
        assert!(formatted.contains("42"));
        assert!(formatted.contains("10"));
        assert!(formatted.contains("my_function"));
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_location(SourceLocation::new().with_file("test.py").with_line(10))
            .with_context("While processing user input")
            .with_suggestion("Check if the variable is defined")
            .with_note("Variables must be defined before use");

        assert!(ctx.has_info());
        let formatted = ctx.format();
        assert!(formatted.contains("test.py"));
        assert!(formatted.contains("While processing"));
        assert!(formatted.contains("Suggestion"));
        assert!(formatted.contains("Note"));
    }

    #[test]
    fn test_error_with_context() {
        let err = RuntimeError::name_error("undefined_var").with_location("test.py", 42);

        let msg = err.to_string();
        assert!(msg.contains("undefined_var"));
        assert!(msg.contains("test.py"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn test_name_error_with_suggestions() {
        let err = RuntimeError::name_error_with_suggestions(
            "prnt",
            Some("test.py"),
            Some(10),
            vec!["print".to_string()],
        );

        let msg = err.to_string();
        assert!(msg.contains("prnt"));
        assert!(msg.contains("test.py"));
        assert!(msg.contains("Did you mean 'print'"));
    }

    #[test]
    fn test_attribute_error_with_suggestions() {
        let err = RuntimeError::attribute_error_with_suggestions(
            "str",
            "uper",
            vec!["upper".to_string()],
        );

        let msg = err.to_string();
        assert!(msg.contains("uper"));
        assert!(msg.contains("Did you mean 'upper'"));
    }

    #[test]
    fn test_import_error_with_suggestions() {
        let err = RuntimeError::import_error_with_suggestions("numpyy", vec!["numpy".to_string()]);

        let msg = err.to_string();
        assert!(msg.contains("numpyy"));
        assert!(msg.contains("Did you mean 'numpy'"));
        assert!(msg.contains("PYTHONPATH"));
    }

    #[test]
    fn test_inner_error() {
        let err = RuntimeError::name_error("test").with_location("file.py", 10);

        assert!(matches!(err.inner(), RuntimeError::NameError { .. }));
    }

    #[test]
    fn test_json_decode_error() {
        let err = RuntimeError::json_decode_error("Expecting value", 1, 5, 4);
        assert_eq!(err.exception_name(), "JSONDecodeError");
        let msg = err.to_string();
        assert!(msg.contains("Expecting value"));
        assert!(msg.contains("line 1"));
        assert!(msg.contains("column 5"));
        assert!(msg.contains("char 4"));
    }

    #[test]
    fn test_json_decode_error_at_pos() {
        let json = "{\n  \"key\": invalid\n}";
        let err = RuntimeError::json_decode_error_at_pos("Unexpected character", json, 12);
        assert_eq!(err.exception_name(), "JSONDecodeError");
        let msg = err.to_string();
        assert!(msg.contains("Unexpected character"));
        // Position 12 is on line 2
        assert!(msg.contains("line 2"));
    }

    #[test]
    fn test_pos_to_line_col() {
        // Test single line
        let (line, col) = RuntimeError::pos_to_line_col("hello", 3);
        assert_eq!(line, 1);
        assert_eq!(col, 4);

        // Test multi-line
        let (line, col) = RuntimeError::pos_to_line_col("hello\nworld", 8);
        assert_eq!(line, 2);
        assert_eq!(col, 3);

        // Test at newline
        let (line, col) = RuntimeError::pos_to_line_col("hello\nworld", 5);
        assert_eq!(line, 1);
        assert_eq!(col, 6);
    }
}
