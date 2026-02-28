//! CSS Language Handler
//!
//! This module provides formatting and linting support for CSS files.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::{FileStatus, LanguageHandler};

/// CSS file extensions
const CSS_EXTENSIONS: &[&str] = &["css", "scss", "sass", "less"];

/// CSS language handler
///
/// Supports `.css`, `.scss`, `.sass`, and `.less` file extensions.
/// Provides basic formatting and linting for CSS files.
pub struct CssHandler;

impl CssHandler {
    /// Create a new CSS handler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Format CSS content
    ///
    /// This performs basic formatting:
    /// - Normalizes line endings
    /// - Ensures proper indentation
    /// - Normalizes whitespace
    /// - Removes trailing semicolons where not needed
    fn format_css(&self, content: &str) -> String {
        // Detect original line ending
        let line_ending = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };

        // Normalize to LF for processing
        let normalized = content.replace("\r\n", "\n");

        let mut formatted = String::new();
        let mut in_rule = false;
        let mut indent_level: i32 = 0;

        for line in normalized.lines() {
            let trimmed = line.trim();

            // Skip empty lines but preserve structure
            if trimmed.is_empty() {
                formatted.push_str(line_ending);
                continue;
            }

            // Handle comments
            if trimmed.contains("/*") {
                formatted.push_str(trimmed);
                formatted.push_str(line_ending);
                continue;
            }

            if trimmed.contains("*/") {
                formatted.push_str(trimmed);
                formatted.push_str(line_ending);
                continue;
            }

            // Handle rule opening
            if trimmed.contains('{') {
                in_rule = true;

                // Add indentation
                for _ in 0..indent_level {
                    formatted.push_str("  ");
                }

                formatted.push_str(&trimmed.replace('{', " {"));
                formatted.push_str(line_ending);
                indent_level += 1;
                continue;
            }

            // Handle rule closing
            if trimmed.contains('}') {
                in_rule = false;
                indent_level = indent_level.saturating_sub(1);

                // Add indentation
                for _ in 0..indent_level {
                    formatted.push_str("  ");
                }

                formatted.push('}');
                formatted.push_str(line_ending);
                continue;
            }

            // Handle declarations
            if in_rule {
                // Add indentation
                for _ in 0..indent_level {
                    formatted.push_str("  ");
                }

                // Format declaration
                let formatted_line = if trimmed.contains(':') {
                    let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        format!("{}: {};", parts[0].trim(), parts[1].trim().trim_end_matches(';'))
                    } else {
                        trimmed.to_string()
                    }
                } else {
                    trimmed.to_string()
                };

                formatted.push_str(&formatted_line);
                formatted.push_str(line_ending);
            } else {
                // Handle selectors and other content
                for _ in 0..indent_level {
                    formatted.push_str("  ");
                }

                formatted.push_str(trimmed);
                formatted.push_str(line_ending);
            }
        }

        formatted
    }

    /// Validate CSS syntax
    fn validate_css(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Basic validation: check for matching braces
        let mut brace_stack = 0;
        let mut in_comment = false;

        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1;

            // Handle comments
            if line.contains("/*") {
                in_comment = true;
            }
            if line.contains("*/") {
                in_comment = false;
                continue;
            }

            if in_comment {
                continue;
            }

            // Count braces
            for c in line.chars() {
                if c == '{' {
                    brace_stack += 1;
                } else if c == '}' {
                    if brace_stack == 0 {
                        diagnostics.push(
                            Diagnostic::error(
                                &file_path_str,
                                "Unexpected closing brace",
                                "lint/css",
                            )
                            .with_line(line_num),
                        );
                    } else {
                        brace_stack -= 1;
                    }
                }
            }
        }

        // Report unclosed braces
        if brace_stack > 0 {
            diagnostics.push(Diagnostic::error(
                &file_path_str,
                format!("Unclosed braces: {brace_stack} opening brace(s) not closed"),
                "lint/css",
            ));
        }

        diagnostics
    }
}

impl Default for CssHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for CssHandler {
    fn extensions(&self) -> &[&str] {
        CSS_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_css(content);

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
                    "io/css",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let diagnostics = self.validate_css(path, content);
        Ok(diagnostics)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are errors, report them
        let errors: Vec<_> = lint_diagnostics
            .iter()
            .filter(|d| d.severity == crate::languages::Severity::Error)
            .collect();

        if !errors.is_empty() {
            return Err(errors[0].clone());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "css"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_handler_extensions() {
        let handler = CssHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"css"));
        assert!(extensions.contains(&"scss"));
        assert!(extensions.contains(&"sass"));
        assert!(extensions.contains(&"less"));
    }

    #[test]
    fn test_css_handler_name() {
        let handler = CssHandler::new();
        assert_eq!(handler.name(), "css");
    }

    #[test]
    fn test_format_css_basic() {
        let handler = CssHandler::new();
        let input = "body{color:red;margin:0;}";
        let formatted = handler.format_css(input);
        assert!(formatted.contains("color: red;"));
        assert!(formatted.contains("margin: 0;"));
    }

    #[test]
    fn test_validate_css_valid() {
        let handler = CssHandler::new();
        let valid_css = "body { color: red; }";
        let diagnostics = handler.validate_css(Path::new("test.css"), valid_css);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_css_invalid() {
        let handler = CssHandler::new();
        let invalid_css = "body { color: red;";
        let diagnostics = handler.validate_css(Path::new("test.css"), invalid_css);
        assert!(!diagnostics.is_empty());
    }
}
