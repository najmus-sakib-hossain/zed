//! OXC parser integration

use crate::error::{DxError, DxResult};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Parsed AST wrapper
#[derive(Debug)]
pub struct ParsedAST {
    /// Source code (owned copy)
    pub source: String,
    /// Filename
    pub filename: String,
    /// Whether this is TypeScript
    pub is_typescript: bool,
    /// Number of statements parsed
    pub statement_count: usize,
}

/// Parse JavaScript/TypeScript source code using OXC
pub fn parse(source: &str, filename: &str) -> DxResult<ParsedAST> {
    // Determine source type from filename
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let is_typescript = filename.ends_with(".ts") || filename.ends_with(".tsx");

    // Create allocator for this parse session
    let allocator = Allocator::default();

    // Parse with OXC
    let parser_result = Parser::new(&allocator, source, source_type).parse();

    // Check for parse errors
    if !parser_result.errors.is_empty() {
        // Try to extract position information from the first error
        let first_error = &parser_result.errors[0];
        let error_str = first_error.to_string();

        // Try to extract line and column from the error
        // OXC errors typically include position information
        if let Some((line, column, message)) = extract_error_position(&error_str, source) {
            return Err(DxError::ParseErrorWithLocation {
                file: filename.to_string(),
                line,
                column,
                message,
            });
        }

        // Fallback to simple error
        let error_messages: Vec<String> =
            parser_result.errors.iter().map(|e| e.to_string()).collect();
        return Err(DxError::ParseError(error_messages.join("\n")));
    }

    // Get statement count for basic info
    let statement_count = parser_result.program.body.len();

    Ok(ParsedAST {
        source: source.to_string(),
        filename: filename.to_string(),
        is_typescript,
        statement_count,
    })
}

/// Extract line and column from an OXC error message
fn extract_error_position(error_str: &str, source: &str) -> Option<(usize, usize, String)> {
    // OXC errors often contain byte offset information
    // Try to find patterns like "at position X" or extract from the error format

    // First, try to find a byte offset in the error message
    // OXC errors typically show the problematic code with context

    // Look for common patterns in error messages
    // Pattern: "Unexpected token" or similar at a specific position

    // For now, try to find the position by looking at the error structure
    // OXC errors contain span information that we can use

    // Simple heuristic: find the first non-whitespace position after the last valid token
    // This is a simplified approach - a full implementation would use OXC's span info

    // Try to extract position from error message format
    // Many parsers include line:column in their error messages

    // Check for patterns like "line X, column Y" or "X:Y"
    let line_col_pattern =
        regex_lite::Regex::new(r"(?:line\s+)?(\d+)(?:,\s*column\s+|\s*:\s*)(\d+)").ok()?;
    if let Some(caps) = line_col_pattern.captures(error_str) {
        let line: usize = caps.get(1)?.as_str().parse().ok()?;
        let column: usize = caps.get(2)?.as_str().parse().ok()?;

        // Extract the actual error message (remove position info)
        let message = error_str
            .split(&format!("{}:{}", line, column))
            .next()
            .unwrap_or(error_str)
            .trim()
            .to_string();

        return Some((
            line,
            column,
            if message.is_empty() {
                error_str.to_string()
            } else {
                message
            },
        ));
    }

    // Fallback: try to find the error position by scanning the source
    // Look for common syntax error indicators
    if let Some(pos) = find_syntax_error_position(source) {
        let (line, column) = offset_to_line_column(source, pos);
        return Some((line, column, error_str.to_string()));
    }

    None
}

/// Find the position of a likely syntax error in the source
fn find_syntax_error_position(source: &str) -> Option<usize> {
    // Look for unclosed brackets, braces, or parentheses
    let mut stack: Vec<(char, usize)> = Vec::new();

    for (i, c) in source.char_indices() {
        match c {
            '(' | '[' | '{' => stack.push((c, i)),
            ')' => {
                if stack.last().map(|(ch, _)| *ch) != Some('(') {
                    return Some(i);
                }
                stack.pop();
            }
            ']' => {
                if stack.last().map(|(ch, _)| *ch) != Some('[') {
                    return Some(i);
                }
                stack.pop();
            }
            '}' => {
                if stack.last().map(|(ch, _)| *ch) != Some('{') {
                    return Some(i);
                }
                stack.pop();
            }
            _ => {}
        }
    }

    // If there are unclosed brackets, return the position of the last one
    stack.last().map(|(_, pos)| *pos)
}

/// Convert a byte offset to line and column numbers (1-indexed)
fn offset_to_line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;

    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

/// Convert line and column to byte offset
pub fn line_column_to_offset(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut current_line = 1;
    let mut current_column = 1;

    for (i, c) in source.char_indices() {
        if current_line == line && current_column == column {
            return Some(i);
        }
        if c == '\n' {
            if current_line == line {
                // Column is past end of line
                return Some(i);
            }
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    // Handle case where position is at end of file
    if current_line == line {
        Some(source.len())
    } else {
        None
    }
}

/// Get basic information about parsed code (for debugging)
pub fn get_ast_info(ast: &ParsedAST) -> AstInfo {
    AstInfo {
        filename: ast.filename.clone(),
        is_typescript: ast.is_typescript,
        source_len: ast.source.len(),
        statement_count: ast.statement_count,
    }
}

#[derive(Debug)]
pub struct AstInfo {
    pub filename: String,
    pub is_typescript: bool,
    pub source_len: usize,
    pub statement_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_js() {
        let source = "const x = 1 + 2;";
        let ast = parse(source, "test.js").unwrap();
        assert_eq!(ast.statement_count, 1);
        assert!(!ast.is_typescript);
    }

    #[test]
    fn test_parse_typescript() {
        let source = "const x: number = 42;";
        let ast = parse(source, "test.ts").unwrap();
        assert!(ast.is_typescript);
    }

    #[test]
    fn test_parse_error() {
        let source = "const x = {";
        let result = parse(source, "test.js");
        assert!(result.is_err());
    }
}
