//! Markdown Language Handler
//!
//! This module provides formatting and linting support for Markdown files
//! using the rumdl crate for high-performance Markdown linting and formatting.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::languages::diagnostic::{Diagnostic, Severity};
use crate::languages::{FileStatus, LanguageHandler};

/// Markdown file extensions
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown"];

/// Markdown language handler using rumdl
///
/// Supports `.md` and `.markdown` file extensions.
/// Uses rumdl for high-performance Markdown linting and formatting.
pub struct MarkdownHandler;

impl MarkdownHandler {
    /// Create a new Markdown handler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if rumdl is available
    fn is_rumdl_available(&self) -> bool {
        Command::new("rumdl").arg("--version").output().is_ok()
    }

    /// Format Markdown content using rumdl
    fn format_with_rumdl(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Check if rumdl is available
        if !self.is_rumdl_available() {
            return Err(Diagnostic::error(
                &file_path_str,
                "rumdl is not installed. Install it with: cargo install rumdl",
                "format/markdown",
            ));
        }

        // Use rumdl fmt with stdin/stdout
        let output = Command::new("rumdl")
            .arg("fmt")
            .arg("-") // Read from stdin
            .arg("--quiet") // Suppress diagnostic messages
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(content.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|e| {
                Diagnostic::error(
                    &file_path_str,
                    format!("Failed to run rumdl: {e}"),
                    "format/markdown",
                )
            })?;

        if output.status.success() {
            String::from_utf8(output.stdout).map_err(|e| {
                Diagnostic::error(
                    &file_path_str,
                    format!("Invalid UTF-8 in rumdl output: {e}"),
                    "format/markdown",
                )
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(Diagnostic::error(
                &file_path_str,
                format!("rumdl formatting failed: {stderr}"),
                "format/markdown",
            ))
        }
    }

    /// Lint Markdown content using rumdl
    fn lint_with_rumdl(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Check if rumdl is available
        if !self.is_rumdl_available() {
            // Return empty diagnostics instead of error (graceful degradation)
            return Ok(Vec::new());
        }

        // Use rumdl check with stdin and JSON output
        let output = Command::new("rumdl")
            .arg("check")
            .arg("-") // Read from stdin
            .arg("--stdin-filename")
            .arg(&file_path_str)
            .arg("--output")
            .arg("json")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(content.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|_| {
                // Graceful degradation - return empty diagnostics
                return Ok(Vec::new());
            });

        let output = match output {
            Ok(o) => o,
            Err(e) => return e,
        };

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);

        // If no issues or empty output, return empty diagnostics
        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Parse rumdl JSON output, but don't fail if parsing fails
        match self.parse_rumdl_output(&file_path_str, &stdout) {
            Ok(diags) => Ok(diags),
            Err(_) => Ok(Vec::new()), // Graceful degradation
        }
    }

    /// Parse rumdl JSON output into diagnostics
    fn parse_rumdl_output(
        &self,
        file_path: &str,
        json_output: &str,
    ) -> Result<Vec<Diagnostic>, Diagnostic> {
        use serde_json::Value;

        let mut diagnostics = Vec::new();

        // Try to parse as JSON
        let json: Value = serde_json::from_str(json_output).map_err(|e| {
            Diagnostic::error(
                file_path,
                format!("Failed to parse rumdl JSON output: {e}"),
                "lint/markdown",
            )
        })?;

        // Extract issues from JSON structure
        if let Some(files) = json.get("files").and_then(|f| f.as_array()) {
            for file in files {
                if let Some(issues) = file.get("issues").and_then(|i| i.as_array()) {
                    for issue in issues {
                        let line = issue
                            .get("line")
                            .and_then(serde_json::Value::as_u64)
                            .map(|l| l as usize);
                        let column = issue
                            .get("column")
                            .and_then(serde_json::Value::as_u64)
                            .map(|c| c as usize);
                        let rule = issue
                            .get("rule")
                            .and_then(|r| r.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let message = issue
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown issue")
                            .to_string();
                        let fixable = issue
                            .get("fixable")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                        let severity_str =
                            issue.get("severity").and_then(|s| s.as_str()).unwrap_or("warning");

                        let severity = match severity_str {
                            "error" => Severity::Error,
                            "warning" => Severity::Warning,
                            _ => Severity::Info,
                        };

                        let mut diag =
                            Diagnostic::new(file_path, message, severity, "lint/markdown")
                                .with_rule(&rule);

                        if let Some(l) = line {
                            diag = diag.with_line(l);
                        }
                        if let Some(c) = column {
                            diag = diag.with_column(c);
                        }
                        // Note: fixable information is available but not stored in Diagnostic
                        // It could be added to the rule field or message if needed
                        let _ = fixable; // Suppress unused warning

                        diagnostics.push(diag);
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    /// Fallback basic formatting (when rumdl is not available)
    fn format_basic(&self, content: &str) -> String {
        // Basic formatting: normalize line endings and ensure trailing newline
        let normalized = content.replace("\r\n", "\n").replace('\r', "\n");

        let mut lines: Vec<String> =
            normalized.lines().map(|line| line.trim_end().to_string()).collect();

        // Remove trailing blank lines but ensure file ends with newline
        while lines.last().is_some_and(std::string::String::is_empty) {
            lines.pop();
        }

        let mut result = lines.join("\n");
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }

        result
    }
}

impl Default for MarkdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for MarkdownHandler {
    fn extensions(&self) -> &[&str] {
        MARKDOWN_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Try to format with rumdl, fall back to basic formatting
        let formatted = if self.is_rumdl_available() {
            match self.format_with_rumdl(path, content) {
                Ok(formatted) => formatted,
                Err(_) => {
                    // Fall back to basic formatting if rumdl fails
                    self.format_basic(content)
                }
            }
        } else {
            // Use basic formatting if rumdl is not available
            self.format_basic(content)
        };

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
                    "io/markdown",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        // Try to lint with rumdl if available
        if self.is_rumdl_available() {
            self.lint_with_rumdl(path, content)
        } else {
            // Return empty diagnostics if rumdl is not available
            // (basic linting could be added here if needed)
            Ok(Vec::new())
        }
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file (but don't fail on warnings)
        let _ = self.lint(path, content)?;

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_handler_extensions() {
        let handler = MarkdownHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"md"));
        assert!(extensions.contains(&"markdown"));
        assert_eq!(extensions.len(), 2);
    }

    #[test]
    fn test_markdown_handler_name() {
        let handler = MarkdownHandler::new();
        assert_eq!(handler.name(), "markdown");
    }

    #[test]
    fn test_basic_format_removes_trailing_whitespace() {
        let handler = MarkdownHandler::new();
        let content = "line1   \nline2\t\nline3\n";
        let formatted = handler.format_basic(content);
        assert_eq!(formatted, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_basic_format_ensures_trailing_newline() {
        let handler = MarkdownHandler::new();
        let content = "line1\nline2";
        let formatted = handler.format_basic(content);
        assert!(formatted.ends_with('\n'));
    }

    #[test]
    fn test_basic_format_normalizes_line_endings() {
        let handler = MarkdownHandler::new();
        let content = "line1\r\nline2\r\n";
        let formatted = handler.format_basic(content);
        assert!(!formatted.contains("\r\n"));
        assert!(formatted.contains('\n'));
    }

    #[test]
    fn test_format_with_basic_fallback() {
        let handler = MarkdownHandler::new();
        let content = "# Heading\n\nParagraph text.\n";

        // This should work regardless of whether rumdl is installed
        let result = handler.format(Path::new("test.md"), content, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lint_without_rumdl() {
        let handler = MarkdownHandler::new();
        let content = "# Heading\n\nParagraph text.\n";

        // Should not fail even if rumdl is not installed
        let result = handler.lint(Path::new("test.md"), content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_without_rumdl() {
        let handler = MarkdownHandler::new();
        let content = "# Heading\n\nParagraph text.\n";

        // Should not fail even if rumdl is not installed
        let result = handler.check(Path::new("test.md"), content, false);
        assert!(result.is_ok());
    }
}
