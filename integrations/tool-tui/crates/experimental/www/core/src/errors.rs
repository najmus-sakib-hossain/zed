//! # Error Module - Structured Error Types with Improved Messages
//!
//! Provides comprehensive error types for the dx-www compiler with:
//! - File path, line, and column information for parse errors
//! - Contextual information about what was being compiled
//! - Suggestions for common mistakes
//! - Component names in runtime errors
//!
//! ## Error Categories
//!
//! - **ParseError**: Syntax and parsing errors with location info
//! - **CompilationError**: Errors during template/binding generation
//! - **RuntimeError**: Errors during component execution
//! - **SecurityError**: Security violations (banned keywords)

use std::path::PathBuf;
use thiserror::Error;

/// Common mistakes and their suggested fixes
pub mod suggestions {
    /// Get a suggestion for a common parse error
    pub fn for_parse_error(message: &str) -> Option<&'static str> {
        // Check for common JSX mistakes
        if message.contains("Unexpected token") && message.contains("<") {
            return Some(
                "JSX expressions must be wrapped in a single parent element. Try wrapping with <> ... </> or a <div>.",
            );
        }
        if message.contains("Expected") && message.contains("}") {
            return Some("Check for unbalanced braces in your JSX expressions.");
        }
        if message.contains("Unterminated string") {
            return Some("Make sure all string literals are properly closed with matching quotes.");
        }
        if message.contains("import") && message.contains("export") {
            return Some(
                "ES modules use 'import' and 'export'. Make sure you're not mixing CommonJS (require/module.exports) syntax.",
            );
        }
        if message.contains("async") || message.contains("await") {
            return Some(
                "'await' can only be used inside an 'async' function. Make sure your component or function is marked as 'async'.",
            );
        }
        if message.contains("useState") || message.contains("useEffect") {
            return Some(
                "React hooks must be called at the top level of your component, not inside loops, conditions, or nested functions.",
            );
        }
        None
    }

    /// Get a suggestion for a common compilation error
    pub fn for_compilation_error(context: &str, message: &str) -> Option<String> {
        if context.contains("template") && message.contains("slot") {
            return Some(
                "Template slots must have unique IDs. Check for duplicate dynamic expressions."
                    .to_string(),
            );
        }
        if context.contains("binding") && message.contains("dirty_bit") {
            return Some("Components are limited to 64 state fields. Consider splitting into smaller components.".to_string());
        }
        if context.contains("state") && message.contains("type") {
            return Some("State types must be serializable. Avoid using functions or complex objects as state.".to_string());
        }
        if message.contains("circular") {
            return Some(
                "Circular dependencies detected. Reorganize your imports to break the cycle."
                    .to_string(),
            );
        }
        None
    }

    /// Get a suggestion for a security violation
    pub fn for_security_violation(keyword: &str) -> &'static str {
        match keyword {
            "eval" => {
                "Use JSON.parse() for parsing JSON, or consider a safer alternative for dynamic code execution."
            }
            "innerHTML" => {
                "Use textContent for plain text, or the framework's safe HTML rendering utilities."
            }
            "outerHTML" => "Use DOM manipulation methods like appendChild() instead.",
            "document.write" => {
                "Use DOM manipulation methods like appendChild() or innerHTML alternatives."
            }
            "Function(" => {
                "Avoid dynamic function creation. Use predefined functions or closures instead."
            }
            "dangerouslySetInnerHTML" => {
                "Use the framework's safe HTML rendering utilities instead."
            }
            "javascript:" => "Use event handlers instead of javascript: URLs.",
            "data:text/html" => {
                "Avoid data URLs with HTML content. Use proper asset loading instead."
            }
            _ => {
                "This keyword is prohibited for security reasons. See the security documentation for alternatives."
            }
        }
    }
}

/// Source location information for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// File path where the error occurred
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Optional end location for ranges
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: impl Into<PathBuf>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            end_line: None,
            end_column: None,
        }
    }

    /// Create a source location with a range
    pub fn with_range(
        file: impl Into<PathBuf>,
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            end_line: Some(end_line),
            end_column: Some(end_column),
        }
    }

    /// Calculate line and column from byte offset in source
    pub fn from_offset(file: impl Into<PathBuf>, source: &str, offset: usize) -> Self {
        let mut line = 1;
        let mut column = 1;
        let mut current_offset = 0;

        for ch in source.chars() {
            if current_offset >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            current_offset += ch.len_utf8();
        }

        Self::new(file, line, column)
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)?;
        if let (Some(end_line), Some(end_col)) = (self.end_line, self.end_column) {
            write!(f, "-{}:{}", end_line, end_col)?;
        }
        Ok(())
    }
}

/// Main error type for the dx-www compiler
#[derive(Debug, Error)]
pub enum DxError {
    /// Parse error with location information
    #[error("Parse error at {location}: {message}")]
    ParseError {
        location: SourceLocation,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        suggestion: Option<String>,
    },

    /// Security violation (banned keywords)
    #[error("Security violation in {file}: banned keyword '{keyword}' detected")]
    SecurityViolation {
        file: PathBuf,
        keyword: String,
        line: Option<usize>,
        column: Option<usize>,
        suggestion: String,
    },

    /// Compilation error with context
    #[error("Compilation error while {context}: {message}")]
    CompilationError {
        context: String,
        message: String,
        file: Option<PathBuf>,
        component: Option<String>,
        suggestion: Option<String>,
    },

    /// Runtime error with component context
    #[error("Runtime error in component '{component}': {message}")]
    RuntimeError {
        component: String,
        message: String,
        stack_trace: Option<String>,
    },

    /// Binary validation error
    #[error("Binary validation failed: {reason}")]
    ValidationError {
        reason: String,
        expected: Option<String>,
        actual: Option<String>,
    },

    /// IO error wrapper
    #[error("IO error: {message}")]
    IoError {
        message: String,
        path: Option<PathBuf>,
        #[source]
        source: Option<std::io::Error>,
    },
}

impl DxError {
    /// Create a parse error with automatic suggestion lookup
    pub fn parse_error(
        file: impl Into<PathBuf>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestions::for_parse_error(&message).map(String::from);
        Self::ParseError {
            location: SourceLocation::new(file, line, column),
            message,
            source: None,
            suggestion,
        }
    }

    /// Create a parse error from a byte offset
    pub fn parse_error_from_offset(
        file: impl Into<PathBuf>,
        source: &str,
        offset: usize,
        message: impl Into<String>,
    ) -> Self {
        let file = file.into();
        let location = SourceLocation::from_offset(&file, source, offset);
        let message = message.into();
        let suggestion = suggestions::for_parse_error(&message).map(String::from);
        Self::ParseError {
            location,
            message,
            source: None,
            suggestion,
        }
    }

    /// Create a security violation error
    pub fn security_violation(
        file: impl Into<PathBuf>,
        keyword: impl Into<String>,
        source_text: Option<&str>,
    ) -> Self {
        let file = file.into();
        let keyword = keyword.into();
        let suggestion = suggestions::for_security_violation(&keyword).to_string();

        // Try to find line/column if source is provided
        let (line, column) = if let Some(src) = source_text {
            find_keyword_location(src, &keyword)
        } else {
            (None, None)
        };

        Self::SecurityViolation {
            file,
            keyword,
            line,
            column,
            suggestion,
        }
    }

    /// Create a compilation error
    pub fn compilation_error(context: impl Into<String>, message: impl Into<String>) -> Self {
        let context = context.into();
        let message = message.into();
        let suggestion = suggestions::for_compilation_error(&context, &message);
        Self::CompilationError {
            context,
            message,
            file: None,
            component: None,
            suggestion,
        }
    }

    /// Create a compilation error with file context
    pub fn compilation_error_in_file(
        context: impl Into<String>,
        message: impl Into<String>,
        file: impl Into<PathBuf>,
    ) -> Self {
        let context = context.into();
        let message = message.into();
        let suggestion = suggestions::for_compilation_error(&context, &message);
        Self::CompilationError {
            context,
            message,
            file: Some(file.into()),
            component: None,
            suggestion,
        }
    }

    /// Create a compilation error with component context
    pub fn compilation_error_in_component(
        context: impl Into<String>,
        message: impl Into<String>,
        component: impl Into<String>,
    ) -> Self {
        let context = context.into();
        let message = message.into();
        let suggestion = suggestions::for_compilation_error(&context, &message);
        Self::CompilationError {
            context,
            message,
            file: None,
            component: Some(component.into()),
            suggestion,
        }
    }

    /// Create a runtime error
    pub fn runtime_error(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self::RuntimeError {
            component: component.into(),
            message: message.into(),
            stack_trace: None,
        }
    }

    /// Create a runtime error with stack trace
    pub fn runtime_error_with_trace(
        component: impl Into<String>,
        message: impl Into<String>,
        stack_trace: impl Into<String>,
    ) -> Self {
        Self::RuntimeError {
            component: component.into(),
            message: message.into(),
            stack_trace: Some(stack_trace.into()),
        }
    }

    /// Create a validation error
    pub fn validation_error(reason: impl Into<String>) -> Self {
        Self::ValidationError {
            reason: reason.into(),
            expected: None,
            actual: None,
        }
    }

    /// Create a validation error with expected/actual values
    pub fn validation_mismatch(
        reason: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::ValidationError {
            reason: reason.into(),
            expected: Some(expected.into()),
            actual: Some(actual.into()),
        }
    }

    /// Create an IO error
    pub fn io_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::IoError {
            message: message.into(),
            path,
            source: None,
        }
    }

    /// Wrap a std::io::Error
    pub fn from_io_error(err: std::io::Error, path: Option<PathBuf>) -> Self {
        Self::IoError {
            message: err.to_string(),
            path,
            source: Some(err),
        }
    }

    /// Get the suggestion for this error, if any
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::ParseError { suggestion, .. } => suggestion.as_deref(),
            Self::SecurityViolation { suggestion, .. } => Some(suggestion),
            Self::CompilationError { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        }
    }

    /// Get the file path associated with this error, if any
    pub fn file(&self) -> Option<&PathBuf> {
        match self {
            Self::ParseError { location, .. } => Some(&location.file),
            Self::SecurityViolation { file, .. } => Some(file),
            Self::CompilationError { file, .. } => file.as_ref(),
            Self::IoError { path, .. } => path.as_ref(),
            _ => None,
        }
    }

    /// Get the component name associated with this error, if any
    pub fn component(&self) -> Option<&str> {
        match self {
            Self::CompilationError { component, .. } => component.as_deref(),
            Self::RuntimeError { component, .. } => Some(component),
            _ => None,
        }
    }

    /// Format the error with full details for display
    pub fn format_detailed(&self) -> String {
        let mut output = format!("error: {}\n", self);

        // Add location info
        if let Self::ParseError { location, .. } = self {
            output.push_str(&format!("  --> {}\n", location));
        } else if let Self::SecurityViolation {
            file, line, column, ..
        } = self
        {
            if let (Some(l), Some(c)) = (line, column) {
                output.push_str(&format!("  --> {}:{}:{}\n", file.display(), l, c));
            } else {
                output.push_str(&format!("  --> {}\n", file.display()));
            }
        } else if let Some(file) = self.file() {
            output.push_str(&format!("  --> {}\n", file.display()));
        }

        // Add component context
        if let Some(component) = self.component() {
            output.push_str(&format!("  in component: {}\n", component));
        }

        // Add stack trace for runtime errors
        if let Self::RuntimeError {
            stack_trace: Some(trace),
            ..
        } = self
        {
            output.push_str("\nStack trace:\n");
            for line in trace.lines() {
                output.push_str(&format!("    {}\n", line));
            }
        }

        // Add suggestion
        if let Some(suggestion) = self.suggestion() {
            output.push_str(&format!("\nhelp: {}\n", suggestion));
        }

        output
    }
}

/// Find the line and column of a keyword in source text
fn find_keyword_location(source: &str, keyword: &str) -> (Option<usize>, Option<usize>) {
    if let Some(offset) = source.find(keyword) {
        let mut line = 1;
        let mut column = 1;
        for (i, ch) in source.char_indices() {
            if i >= offset {
                return (Some(line), Some(column));
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
    }
    (None, None)
}

/// Result type alias for DxError
pub type DxResult<T> = Result<T, DxError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_from_offset() {
        let source = "line 1\nline 2\nline 3";

        // Start of file
        let loc = SourceLocation::from_offset("test.tsx", source, 0);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);

        // Middle of line 1
        let loc = SourceLocation::from_offset("test.tsx", source, 3);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 4);

        // Start of line 2
        let loc = SourceLocation::from_offset("test.tsx", source, 7);
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 1);

        // Middle of line 3
        let loc = SourceLocation::from_offset("test.tsx", source, 16);
        assert_eq!(loc.line, 3);
        assert_eq!(loc.column, 3);
    }

    #[test]
    fn test_parse_error_with_suggestion() {
        let err = DxError::parse_error("test.tsx", 10, 5, "Unexpected token <");

        assert!(err.suggestion().is_some());
        assert!(err.suggestion().unwrap().contains("parent element"));
    }

    #[test]
    fn test_security_violation() {
        let source = "function App() {\n  eval('code');\n}";
        let err = DxError::security_violation("test.tsx", "eval", Some(source));

        if let DxError::SecurityViolation {
            line, suggestion, ..
        } = &err
        {
            assert_eq!(*line, Some(2));
            assert!(suggestion.contains("JSON.parse"));
        } else {
            panic!("Expected SecurityViolation");
        }
    }

    #[test]
    fn test_compilation_error_with_context() {
        let err = DxError::compilation_error_in_component(
            "generating template",
            "Too many state fields",
            "MyComponent",
        );

        assert_eq!(err.component(), Some("MyComponent"));
    }

    #[test]
    fn test_runtime_error_with_trace() {
        let err = DxError::runtime_error_with_trace(
            "Counter",
            "State update failed",
            "at Counter.increment\nat onClick",
        );

        if let DxError::RuntimeError { stack_trace, .. } = &err {
            assert!(stack_trace.is_some());
            assert!(stack_trace.as_ref().unwrap().contains("increment"));
        } else {
            panic!("Expected RuntimeError");
        }
    }

    #[test]
    fn test_format_detailed() {
        let err = DxError::parse_error("src/App.tsx", 15, 10, "Unexpected token <");

        let formatted = err.format_detailed();
        assert!(formatted.contains("error:"));
        assert!(formatted.contains("src/App.tsx:15:10"));
        assert!(formatted.contains("help:"));
    }

    #[test]
    fn test_find_keyword_location() {
        let source = "line 1\neval('bad');\nline 3";
        let (line, col) = find_keyword_location(source, "eval");
        assert_eq!(line, Some(2));
        assert_eq!(col, Some(1));
    }
}
