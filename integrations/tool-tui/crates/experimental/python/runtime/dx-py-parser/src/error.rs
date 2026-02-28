//! Parser error types with helpful suggestions
//!
//! This module provides detailed error messages with:
//! - Precise line/column information
//! - Helpful suggestions for common mistakes
//! - Context about what was expected

use thiserror::Error;

/// Location in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset in source
    pub offset: usize,
}

impl Location {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Suggestion for fixing an error
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Description of the fix
    pub message: String,
    /// Optional replacement text
    pub replacement: Option<String>,
}

/// Parse error with location information
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token at {location}: expected {expected}, got {actual}")]
    UnexpectedToken {
        location: Location,
        expected: String,
        actual: String,
    },

    #[error("Unexpected end of file at {location}: {message}")]
    UnexpectedEof { location: Location, message: String },

    #[error("Invalid syntax at {location}: {message}")]
    InvalidSyntax { location: Location, message: String },

    #[error("Indentation error at {location}: {message}")]
    IndentationError { location: Location, message: String },

    #[error("Invalid string literal at {location}: {message}")]
    InvalidString { location: Location, message: String },

    #[error("Invalid number literal at {location}: {message}")]
    InvalidNumber { location: Location, message: String },

    #[error("Invalid identifier at {location}: {message}")]
    InvalidIdentifier { location: Location, message: String },
}

impl ParseError {
    pub fn unexpected_token(location: Location, expected: &str, actual: &str) -> Self {
        Self::UnexpectedToken {
            location,
            expected: expected.to_string(),
            actual: actual.to_string(),
        }
    }

    pub fn unexpected_eof(location: Location, message: &str) -> Self {
        Self::UnexpectedEof {
            location,
            message: message.to_string(),
        }
    }

    pub fn invalid_syntax(location: Location, message: &str) -> Self {
        Self::InvalidSyntax {
            location,
            message: message.to_string(),
        }
    }

    pub fn indentation_error(location: Location, message: &str) -> Self {
        Self::IndentationError {
            location,
            message: message.to_string(),
        }
    }

    /// Get the location of the error
    pub fn location(&self) -> Location {
        match self {
            Self::UnexpectedToken { location, .. } => *location,
            Self::UnexpectedEof { location, .. } => *location,
            Self::InvalidSyntax { location, .. } => *location,
            Self::IndentationError { location, .. } => *location,
            Self::InvalidString { location, .. } => *location,
            Self::InvalidNumber { location, .. } => *location,
            Self::InvalidIdentifier { location, .. } => *location,
        }
    }
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

impl ParseError {
    /// Get a helpful suggestion for this error
    pub fn suggestion(&self) -> Option<Suggestion> {
        match self {
            Self::UnexpectedToken {
                expected, actual, ..
            } => suggest_for_unexpected_token(expected, actual),
            Self::IndentationError { message, .. } => {
                if message.contains("inconsistent") {
                    Some(Suggestion {
                        message: "Use consistent indentation (4 spaces recommended)".to_string(),
                        replacement: None,
                    })
                } else if message.contains("unexpected") {
                    Some(Suggestion {
                        message: "Check that your indentation matches the surrounding code"
                            .to_string(),
                        replacement: None,
                    })
                } else {
                    None
                }
            }
            Self::InvalidString { message, .. } => {
                if message.contains("unterminated") {
                    Some(Suggestion {
                        message: "Add a closing quote to terminate the string".to_string(),
                        replacement: None,
                    })
                } else if message.contains("escape") {
                    Some(Suggestion {
                        message: "Use a raw string (r\"...\") or escape the backslash (\\\\)"
                            .to_string(),
                        replacement: None,
                    })
                } else {
                    None
                }
            }
            Self::InvalidNumber { message, .. } => {
                if message.contains("leading zero") {
                    Some(Suggestion {
                        message: "Use 0o prefix for octal numbers (e.g., 0o755)".to_string(),
                        replacement: None,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Format error with source context
    pub fn format_with_source(&self, source: &str) -> String {
        let loc = self.location();
        let lines: Vec<&str> = source.lines().collect();

        let mut output = format!("Error: {}\n", self);

        // Show the problematic line
        if loc.line > 0 && loc.line <= lines.len() {
            let line_content = lines[loc.line - 1];
            output.push_str(&format!("\n  {} | {}\n", loc.line, line_content));

            // Show caret pointing to the error
            let padding = " ".repeat(loc.line.to_string().len() + 3 + loc.column.saturating_sub(1));
            output.push_str(&format!("{}^\n", padding));
        }

        // Add suggestion if available
        if let Some(suggestion) = self.suggestion() {
            output.push_str(&format!("\nHint: {}\n", suggestion.message));
        }

        output
    }
}

/// Generate suggestions for unexpected token errors
fn suggest_for_unexpected_token(expected: &str, actual: &str) -> Option<Suggestion> {
    // Common mistakes
    match (expected, actual) {
        (":", "=") => Some(Suggestion {
            message: "Use ':' for type annotations, not '='".to_string(),
            replacement: Some(":".to_string()),
        }),
        ("=", "==") => Some(Suggestion {
            message: "Use '=' for assignment, '==' is for comparison".to_string(),
            replacement: Some("=".to_string()),
        }),
        ("==", "=") => Some(Suggestion {
            message: "Use '==' for comparison, '=' is for assignment".to_string(),
            replacement: Some("==".to_string()),
        }),
        (")", "]") | ("]", ")") => Some(Suggestion {
            message: "Mismatched brackets - check your parentheses and brackets".to_string(),
            replacement: None,
        }),
        ("identifier", _) if actual.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
            Some(Suggestion {
                message: "Identifiers cannot start with a number".to_string(),
                replacement: None,
            })
        }
        (_, "def") | (_, "class") | (_, "if") | (_, "for") | (_, "while") => Some(Suggestion {
            message: format!("'{}' is a reserved keyword and cannot be used here", actual),
            replacement: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_location() {
        let err = ParseError::unexpected_token(Location::new(5, 10, 50), "identifier", "123");
        assert_eq!(err.location().line, 5);
        assert_eq!(err.location().column, 10);
    }

    #[test]
    fn test_error_suggestion() {
        let err = ParseError::unexpected_token(Location::new(1, 1, 0), ":", "=");
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().message.contains("type annotations"));
    }

    #[test]
    fn test_format_with_source() {
        let source = "def foo(x = int):\n    pass";
        let err = ParseError::unexpected_token(Location::new(1, 11, 10), ":", "=");
        let formatted = err.format_with_source(source);
        assert!(formatted.contains("def foo(x = int):"));
        assert!(formatted.contains("^"));
    }

    #[test]
    fn test_indentation_suggestion() {
        let err =
            ParseError::indentation_error(Location::new(2, 1, 10), "inconsistent indentation");
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().message.contains("4 spaces"));
    }
}
