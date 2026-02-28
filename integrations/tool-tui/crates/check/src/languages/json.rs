//! JSON Language Handler
//!
//! This module provides formatting and linting support for JSON files
//! using `serde_json` for parsing and validation.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::{FileStatus, LanguageHandler};

/// JSON file extensions
const JSON_EXTENSIONS: &[&str] = &["json", "jsonc"];

/// JSON language handler
///
/// Supports `.json` and `.jsonc` file extensions.
/// Uses `serde_json` for parsing and validation.
pub struct JsonHandler;

impl JsonHandler {
    /// Create a new JSON handler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Format JSON content
    ///
    /// This performs formatting:
    /// - Parses and re-serializes JSON
    /// - Ensures consistent indentation
    /// - Normalizes whitespace
    fn format_json(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Remove JSONC comments if present
        let content_without_comments = remove_jsonc_comments(content);

        // Parse JSON content
        let value: serde_json::Value =
            serde_json::from_str(&content_without_comments).map_err(|e| {
                let (line, col) = Self::extract_location_from_error(&e);
                let mut diag = Diagnostic::error(
                    &file_path_str,
                    format!("JSON parse error: {e}"),
                    "format/json",
                );
                if let Some(l) = line {
                    diag = diag.with_line(l);
                }
                if let Some(c) = col {
                    diag = diag.with_column(c);
                }
                diag
            })?;

        // Re-serialize with pretty formatting
        let formatted = serde_json::to_string_pretty(&value).map_err(|e| {
            Diagnostic::error(
                &file_path_str,
                format!("JSON serialization error: {e}"),
                "format/json",
            )
        })?;

        Ok(formatted)
    }

    /// Validate JSON syntax and semantics
    fn validate_json(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Remove JSONC comments if present
        let content_without_comments = remove_jsonc_comments(content);

        // Try to parse JSON content
        match serde_json::from_str::<serde_json::Value>(&content_without_comments) {
            Ok(_) => {
                // Syntax is valid, no errors
            }
            Err(e) => {
                let (line, col) = Self::extract_location_from_error(&e);
                let mut diag = Diagnostic::error(
                    &file_path_str,
                    format!("JSON syntax error: {e}"),
                    "lint/json",
                );
                if let Some(l) = line {
                    diag = diag.with_line(l);
                }
                if let Some(c) = col {
                    diag = diag.with_column(c);
                }
                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    /// Extract line and column from a JSON parse error
    fn extract_location_from_error(error: &serde_json::Error) -> (Option<usize>, Option<usize>) {
        // serde_json errors often contain line/column information
        let error_str = error.to_string();

        // Try to parse line number from error message
        if let Some(line_start) = error_str.find("line ") {
            let line_part = &error_str[line_start + 5..];
            if let Some(line_end) = line_part.find(' ')
                && let Ok(line_num) = line_part[..line_end].parse::<usize>()
            {
                // Try to parse column number
                if let Some(col_start) = error_str.find("column ") {
                    let col_part = &error_str[col_start + 7..];
                    if let Some(col_end) = col_part.find(' ')
                        && let Ok(col_num) = col_part[..col_end].parse::<usize>()
                    {
                        return (Some(line_num), Some(col_num));
                    }
                }
                return (Some(line_num), None);
            }
        }

        (None, None)
    }
}

/// Remove JSONC (JSON with comments) comments from content
fn remove_jsonc_comments(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut in_single_line_comment = false;
    let mut in_multi_line_comment = false;

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_single_line_comment && !in_multi_line_comment => {
                // Check if it's an escaped quote
                if let Some(prev) = result.chars().last()
                    && prev == '\\'
                {
                    result.push(c);
                    continue;
                }
                in_string = !in_string;
                result.push(c);
            }
            '/' if !in_string && !in_single_line_comment && !in_multi_line_comment => {
                if let Some(&next) = chars.peek() {
                    match next {
                        '/' => {
                            in_single_line_comment = true;
                            chars.next(); // consume the second '/'
                        }
                        '*' => {
                            in_multi_line_comment = true;
                            chars.next(); // consume the '*'
                        }
                        _ => {
                            result.push(c);
                        }
                    }
                }
            }
            '\n' if in_single_line_comment => {
                in_single_line_comment = false;
                result.push(c);
            }
            '*' if in_multi_line_comment => {
                if let Some(&next) = chars.peek()
                    && next == '/'
                {
                    in_multi_line_comment = false;
                    chars.next(); // consume the '/'
                }
            }
            _ if !in_single_line_comment && !in_multi_line_comment => {
                result.push(c);
            }
            _ => {
                // Skip characters in comments
            }
        }
    }

    result
}

impl Default for JsonHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for JsonHandler {
    fn extensions(&self) -> &[&str] {
        JSON_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_json(path, content)?;

        // Check if content changed
        if formatted == content {
            return Ok(FileStatus::Unchanged);
        }

        // Write if requested
        if write {
            fs::write(path, &formatted).map_err(|e| {
                Diagnostic::error(
                    &file_path_str,
                    format!("Failed to write formatted content: {e}"),
                    "io/json",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let diagnostics = self.validate_json(path, content);
        Ok(diagnostics)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are errors, report them
        if !lint_diagnostics.is_empty() {
            return Err(lint_diagnostics.into_iter().next().unwrap());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_handler_extensions() {
        let handler = JsonHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"json"));
        assert!(extensions.contains(&"jsonc"));
    }

    #[test]
    fn test_json_handler_name() {
        let handler = JsonHandler::new();
        assert_eq!(handler.name(), "json");
    }

    #[test]
    fn test_format_json_valid() {
        let handler = JsonHandler::new();
        let input = r#"{"name":"test","value":123}"#;
        let formatted = handler.format_json(Path::new("test.json"), input).unwrap();
        assert!(formatted.contains("name"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn test_validate_json_valid() {
        let handler = JsonHandler::new();
        let valid_json = r#"{"name": "test", "value": 123}"#;
        let diagnostics = handler.validate_json(Path::new("test.json"), valid_json);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_json_invalid() {
        let handler = JsonHandler::new();
        let invalid_json = r#"{"name": "test", "value": }"#;
        let diagnostics = handler.validate_json(Path::new("test.json"), invalid_json);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_remove_jsonc_comments() {
        let input = r#"{"name": "test", // comment
        "value": 123 /* multi-line
        comment */}"#;
        let result = remove_jsonc_comments(input);
        assert!(!result.contains("//"));
        assert!(!result.contains("/*"));
    }
}
