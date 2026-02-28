//! Compiler error types
//!
//! This module provides detailed error messages with:
//! - Precise line/column information
//! - Source context showing the problematic line
//! - A caret (^) pointing to the error location
//! - Clear error messages following Python's error format

use std::path::PathBuf;

/// Compilation error
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// Syntax error in source
    #[error("SyntaxError: {message} at {file}:{line}:{column}")]
    SyntaxError {
        message: String,
        file: PathBuf,
        line: u32,
        column: u32,
        source_line: String,
    },

    /// Name resolution error
    #[error("NameError: {message} at {file}:{line}")]
    NameError {
        message: String,
        name: String,
        file: PathBuf,
        line: u32,
    },

    /// Invalid bytecode generation
    #[error("CodeGenError: {message}")]
    CodeGenError { message: String },

    /// Symbol table error
    #[error("SymbolError: {0}")]
    SymbolError(#[from] crate::symbol_table::SymbolError),

    /// Parse error
    #[error("ParseError: {0}")]
    ParseError(#[from] dx_py_parser::ParseError),

    /// IO error
    #[error("IOError: {0}")]
    IoError(#[from] std::io::Error),
}

impl CompileError {
    /// Create a syntax error
    pub fn syntax_error(
        message: impl Into<String>,
        file: impl Into<PathBuf>,
        line: u32,
        column: u32,
        source_line: impl Into<String>,
    ) -> Self {
        Self::SyntaxError {
            message: message.into(),
            file: file.into(),
            line,
            column,
            source_line: source_line.into(),
        }
    }

    /// Create a name error
    pub fn name_error(
        message: impl Into<String>,
        name: impl Into<String>,
        file: impl Into<PathBuf>,
        line: u32,
    ) -> Self {
        Self::NameError {
            message: message.into(),
            name: name.into(),
            file: file.into(),
            line,
        }
    }

    /// Create a code generation error
    pub fn codegen_error(message: impl Into<String>) -> Self {
        Self::CodeGenError {
            message: message.into(),
        }
    }

    /// Format the error for display (without source context)
    pub fn format(&self) -> String {
        match self {
            CompileError::SyntaxError {
                message,
                file,
                line,
                column,
                source_line,
            } => {
                let pointer = " ".repeat(*column as usize) + "^";
                format!(
                    "  File \"{}\", line {}\n    {}\n    {}\nSyntaxError: {}",
                    file.display(),
                    line,
                    source_line,
                    pointer,
                    message
                )
            }
            CompileError::NameError {
                message,
                name,
                file,
                line,
            } => {
                format!(
                    "  File \"{}\", line {}\nNameError: {} (name '{}')",
                    file.display(),
                    line,
                    message,
                    name
                )
            }
            CompileError::CodeGenError { message } => {
                format!("CodeGenError: {}", message)
            }
            CompileError::SymbolError(e) => format!("SymbolError: {}", e),
            CompileError::ParseError(e) => format!("ParseError: {}", e),
            CompileError::IoError(e) => format!("IOError: {}", e),
        }
    }

    /// Format the error with source context for better error reporting
    /// 
    /// This produces Python-style error output:
    /// ```text
    ///   File "<string>", line 1
    ///     x = 1 +
    ///           ^
    /// SyntaxError: unexpected end of expression
    /// ```
    pub fn format_with_source(&self, source: &str, filename: &str) -> String {
        match self {
            CompileError::SyntaxError {
                message,
                line,
                column,
                source_line,
                ..
            } => {
                // Use the stored source_line if available, otherwise extract from source
                let line_content = if !source_line.is_empty() {
                    source_line.clone()
                } else {
                    source.lines().nth((*line as usize).saturating_sub(1))
                        .unwrap_or("")
                        .to_string()
                };
                
                // Calculate caret position (column is 1-indexed)
                let caret_pos = (*column as usize).saturating_sub(1);
                let caret = " ".repeat(caret_pos) + "^";
                
                format!(
                    "  File \"{}\", line {}\n    {}\n    {}\nSyntaxError: {}",
                    filename,
                    line,
                    line_content,
                    caret,
                    message
                )
            }
            CompileError::NameError {
                message,
                name,
                line,
                ..
            } => {
                // Get the source line for context
                let line_content = source.lines().nth((*line as usize).saturating_sub(1))
                    .unwrap_or("");
                
                format!(
                    "  File \"{}\", line {}\n    {}\nNameError: {} (name '{}')",
                    filename,
                    line,
                    line_content,
                    message,
                    name
                )
            }
            CompileError::ParseError(parse_err) => {
                // Use the parser's format_with_source for parse errors
                let loc = parse_err.location();
                let lines: Vec<&str> = source.lines().collect();
                
                let mut output = format!("  File \"{}\", line {}\n", filename, loc.line);
                
                // Show the problematic line
                if loc.line > 0 && loc.line <= lines.len() {
                    let line_content = lines[loc.line - 1];
                    output.push_str(&format!("    {}\n", line_content));
                    
                    // Show caret pointing to the error (column is 1-indexed)
                    let caret_pos = loc.column.saturating_sub(1);
                    let caret = " ".repeat(caret_pos) + "^";
                    output.push_str(&format!("    {}\n", caret));
                }
                
                // Format the error message based on the parse error type
                let error_msg = match parse_err {
                    dx_py_parser::ParseError::UnexpectedToken { expected, actual, .. } => {
                        format!("SyntaxError: expected {}, got {}", expected, actual)
                    }
                    dx_py_parser::ParseError::UnexpectedEof { message, .. } => {
                        format!("SyntaxError: unexpected end of input: {}", message)
                    }
                    dx_py_parser::ParseError::InvalidSyntax { message, .. } => {
                        format!("SyntaxError: {}", message)
                    }
                    dx_py_parser::ParseError::IndentationError { message, .. } => {
                        format!("IndentationError: {}", message)
                    }
                    dx_py_parser::ParseError::InvalidString { message, .. } => {
                        format!("SyntaxError: invalid string literal: {}", message)
                    }
                    dx_py_parser::ParseError::InvalidNumber { message, .. } => {
                        format!("SyntaxError: invalid number literal: {}", message)
                    }
                    dx_py_parser::ParseError::InvalidIdentifier { message, .. } => {
                        format!("SyntaxError: invalid identifier: {}", message)
                    }
                };
                
                output.push_str(&error_msg);
                
                // Add suggestion if available
                if let Some(suggestion) = parse_err.suggestion() {
                    output.push_str(&format!("\nHint: {}", suggestion.message));
                }
                
                output
            }
            CompileError::CodeGenError { message } => {
                format!("CodeGenError: {}", message)
            }
            CompileError::SymbolError(e) => {
                format!("SymbolError: {}", e)
            }
            CompileError::IoError(e) => {
                format!("IOError: {}", e)
            }
        }
    }

    /// Get the line number if available
    pub fn line(&self) -> Option<u32> {
        match self {
            CompileError::SyntaxError { line, .. } => Some(*line),
            CompileError::NameError { line, .. } => Some(*line),
            CompileError::ParseError(e) => Some(e.location().line as u32),
            _ => None,
        }
    }
    
    /// Get the column number if available
    pub fn column(&self) -> Option<u32> {
        match self {
            CompileError::SyntaxError { column, .. } => Some(*column),
            CompileError::ParseError(e) => Some(e.location().column as u32),
            _ => None,
        }
    }
}

/// Result type for compilation operations
pub type CompileResult<T> = Result<T, CompileError>;


#[cfg(test)]
mod tests {
    use super::*;
    use dx_py_parser::{ParseError, error::Location};

    #[test]
    fn test_syntax_error_format_with_source() {
        let err = CompileError::syntax_error(
            "unexpected end of expression",
            "<string>",
            1,
            8,
            "x = 1 +",
        );
        
        let formatted = err.format_with_source("x = 1 +", "<string>");
        
        // Should contain the file and line info
        assert!(formatted.contains("File \"<string>\", line 1"));
        // Should contain the source line
        assert!(formatted.contains("x = 1 +"));
        // Should contain the caret pointing to the error
        assert!(formatted.contains("^"));
        // Should contain the error message
        assert!(formatted.contains("SyntaxError: unexpected end of expression"));
    }

    #[test]
    fn test_parse_error_format_with_source() {
        let parse_err = ParseError::unexpected_token(
            Location::new(1, 7, 6),
            "expression",
            "end of input",
        );
        let err = CompileError::ParseError(parse_err);
        
        let source = "x = 1 +";
        let formatted = err.format_with_source(source, "<string>");
        
        // Should contain the file and line info
        assert!(formatted.contains("File \"<string>\", line 1"));
        // Should contain the source line
        assert!(formatted.contains("x = 1 +"));
        // Should contain the caret
        assert!(formatted.contains("^"));
        // Should contain SyntaxError
        assert!(formatted.contains("SyntaxError"));
    }

    #[test]
    fn test_parse_error_invalid_syntax() {
        let parse_err = ParseError::invalid_syntax(
            Location::new(1, 5, 4),
            "invalid syntax",
        );
        let err = CompileError::ParseError(parse_err);
        
        let source = "def (x):";
        let formatted = err.format_with_source(source, "<string>");
        
        assert!(formatted.contains("File \"<string>\", line 1"));
        assert!(formatted.contains("def (x):"));
        assert!(formatted.contains("SyntaxError: invalid syntax"));
    }

    #[test]
    fn test_parse_error_indentation() {
        let parse_err = ParseError::indentation_error(
            Location::new(2, 1, 10),
            "unexpected indent",
        );
        let err = CompileError::ParseError(parse_err);
        
        let source = "def foo():\n  pass";
        let formatted = err.format_with_source(source, "<string>");
        
        assert!(formatted.contains("File \"<string>\", line 2"));
        assert!(formatted.contains("IndentationError: unexpected indent"));
    }

    #[test]
    fn test_name_error_format_with_source() {
        let err = CompileError::name_error(
            "name 'undefined_var' is not defined",
            "undefined_var",
            "<string>",
            3,
        );
        
        let source = "x = 1\ny = 2\nprint(undefined_var)";
        let formatted = err.format_with_source(source, "<string>");
        
        assert!(formatted.contains("File \"<string>\", line 3"));
        assert!(formatted.contains("print(undefined_var)"));
        assert!(formatted.contains("NameError"));
    }

    #[test]
    fn test_error_line_number() {
        let syntax_err = CompileError::syntax_error(
            "test",
            "<string>",
            5,
            1,
            "test line",
        );
        assert_eq!(syntax_err.line(), Some(5));
        
        let name_err = CompileError::name_error(
            "test",
            "x",
            "<string>",
            10,
        );
        assert_eq!(name_err.line(), Some(10));
        
        let parse_err = ParseError::invalid_syntax(
            Location::new(7, 3, 20),
            "test",
        );
        let compile_err = CompileError::ParseError(parse_err);
        assert_eq!(compile_err.line(), Some(7));
    }

    #[test]
    fn test_error_column_number() {
        let syntax_err = CompileError::syntax_error(
            "test",
            "<string>",
            1,
            15,
            "test line",
        );
        assert_eq!(syntax_err.column(), Some(15));
        
        let parse_err = ParseError::invalid_syntax(
            Location::new(1, 8, 7),
            "test",
        );
        let compile_err = CompileError::ParseError(parse_err);
        assert_eq!(compile_err.column(), Some(8));
    }

    #[test]
    fn test_caret_position() {
        let err = CompileError::syntax_error(
            "unexpected token",
            "<string>",
            1,
            5,  // Column 5 (1-indexed)
            "x = 1 + +",
        );
        
        let formatted = err.format_with_source("x = 1 + +", "<string>");
        
        // The format is:
        //   File "<string>", line 1
        //     x = 1 + +
        //     ^
        // The caret line has 4 spaces of indentation plus (column-1) spaces
        let lines: Vec<&str> = formatted.lines().collect();
        let caret_line = lines.iter().find(|l| l.contains('^')).unwrap();
        
        // Count spaces before ^
        let spaces_before_caret = caret_line.chars().take_while(|c| *c == ' ').count();
        // Should be 4 (indentation) + 4 (column 5 - 1) = 8 spaces
        assert_eq!(spaces_before_caret, 8);
    }

    #[test]
    fn test_multiline_source_error() {
        let err = CompileError::syntax_error(
            "unexpected token",
            "<string>",
            3,
            10,
            "    return x +",
        );
        
        let source = "def foo():\n    x = 1\n    return x +";
        let formatted = err.format_with_source(source, "<string>");
        
        assert!(formatted.contains("File \"<string>\", line 3"));
        assert!(formatted.contains("return x +"));
    }
}
