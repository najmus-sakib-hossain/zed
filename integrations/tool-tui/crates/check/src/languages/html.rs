//! HTML Language Handler
//!
//! This module provides formatting and linting support for HTML files.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::{FileStatus, LanguageHandler};

/// HTML file extensions
const HTML_EXTENSIONS: &[&str] = &["html", "htm"];

/// HTML language handler
///
/// Supports `.html` and `.htm` file extensions.
/// Provides basic formatting and linting for HTML files.
pub struct HtmlHandler;

impl HtmlHandler {
    /// Create a new HTML handler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Format HTML content
    ///
    /// This performs basic formatting:
    /// - Normalizes line endings
    /// - Ensures proper indentation
    /// - Normalizes attribute spacing
    fn format_html(&self, content: &str) -> String {
        // Detect original line ending
        let line_ending = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };

        // Normalize to LF for processing
        let normalized = content.replace("\r\n", "\n");

        // Basic HTML formatting
        let mut formatted = String::new();
        let mut indent_level: i32 = 0;
        let _in_tag = false;
        let mut in_comment = false;
        let mut in_script = false;
        let mut in_style = false;

        for line in normalized.lines() {
            let trimmed = line.trim();

            // Skip empty lines but preserve structure
            if trimmed.is_empty() {
                formatted.push_str(line_ending);
                continue;
            }

            // Detect comments
            if trimmed.contains("<!--") {
                in_comment = true;
            }
            if trimmed.contains("-->") {
                in_comment = false;
                formatted.push_str(trimmed);
                formatted.push_str(line_ending);
                continue;
            }

            // Skip formatting inside comments
            if in_comment {
                formatted.push_str(trimmed);
                formatted.push_str(line_ending);
                continue;
            }

            // Detect script and style tags
            if trimmed.contains("<script") && !trimmed.contains("</script>") {
                in_script = true;
            }
            if trimmed.contains("</script>") {
                in_script = false;
            }
            if trimmed.contains("<style") && !trimmed.contains("</style>") {
                in_style = true;
            }
            if trimmed.contains("</style>") {
                in_style = false;
            }

            // Skip formatting inside script and style tags
            if in_script || in_style {
                formatted.push_str(trimmed);
                formatted.push_str(line_ending);
                continue;
            }

            // Handle closing tags
            if trimmed.starts_with("</") {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add indentation
            for _ in 0..indent_level {
                formatted.push_str("  ");
            }

            // Add the line
            formatted.push_str(trimmed);
            formatted.push_str(line_ending);

            // Handle opening tags
            if (trimmed.starts_with('<') && !trimmed.starts_with("</") && !trimmed.ends_with("/>"))
                || (trimmed.starts_with('<') && !trimmed.contains('>') && !trimmed.contains("</"))
            {
                indent_level += 1;
            }
        }

        formatted
    }

    /// Validate HTML syntax
    fn validate_html(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Basic validation: check for unclosed tags
        let mut tag_stack = Vec::new();
        let mut in_comment = false;
        let mut in_script = false;
        let mut in_style = false;

        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1;

            // Skip comments
            if line.contains("<!--") {
                in_comment = true;
            }
            if line.contains("-->") {
                in_comment = false;
                continue;
            }

            if in_comment {
                continue;
            }

            // Detect script and style tags
            if line.contains("<script") && !line.contains("</script>") {
                in_script = true;
            }
            if line.contains("</script>") {
                in_script = false;
            }
            if line.contains("<style") && !line.contains("</style>") {
                in_style = true;
            }
            if line.contains("</style>") {
                in_style = false;
            }

            if in_script || in_style {
                continue;
            }

            // Extract tags from line
            for tag in extract_tags(line) {
                if tag.starts_with("</") {
                    // Closing tag
                    let tag_name = tag[2..].trim_end_matches('>').trim();
                    if let Some(pos) = tag_stack.iter().rposition(|t| t == tag_name) {
                        tag_stack.truncate(pos);
                    } else {
                        diagnostics.push(
                            Diagnostic::warning(
                                &file_path_str,
                                format!("Unexpected closing tag: {tag_name}"),
                                "lint/html",
                            )
                            .with_line(line_num),
                        );
                    }
                } else if !tag.ends_with("/>") && !tag.starts_with('!') {
                    // Opening tag
                    let tag_name =
                        tag[1..].trim_end_matches('>').split_whitespace().next().unwrap_or("");
                    if !tag_name.is_empty() && !is_void_element(tag_name) {
                        tag_stack.push(tag_name.to_string());
                    }
                }
            }
        }

        // Report unclosed tags
        for tag in tag_stack {
            diagnostics.push(Diagnostic::warning(
                &file_path_str,
                format!("Unclosed tag: {tag}"),
                "lint/html",
            ));
        }

        diagnostics
    }
}

/// Extract HTML tags from a line
fn extract_tags(line: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::from('<');
            while let Some(&next_c) = chars.peek() {
                if next_c == '>' {
                    tag.push(chars.next().unwrap());
                    break;
                }
                tag.push(chars.next().unwrap());
            }
            tags.push(tag);
        }
    }

    tags
}

/// Check if an HTML element is a void element (self-closing)
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

impl Default for HtmlHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for HtmlHandler {
    fn extensions(&self) -> &[&str] {
        HTML_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_html(content);

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
                    "io/html",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let diagnostics = self.validate_html(path, content);
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
        "html"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_handler_extensions() {
        let handler = HtmlHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"html"));
        assert!(extensions.contains(&"htm"));
    }

    #[test]
    fn test_html_handler_name() {
        let handler = HtmlHandler::new();
        assert_eq!(handler.name(), "html");
    }

    #[test]
    fn test_extract_tags() {
        let line = "<div><p>Hello</p></div>";
        let tags = extract_tags(line);
        assert_eq!(tags, vec!["<div>", "<p>", "</p>", "</div>"]);
    }

    #[test]
    fn test_is_void_element() {
        assert!(is_void_element("br"));
        assert!(is_void_element("img"));
        assert!(is_void_element("input"));
        assert!(!is_void_element("div"));
        assert!(!is_void_element("p"));
    }

    #[test]
    fn test_format_html_basic() {
        let handler = HtmlHandler::new();
        let input = "<html><body><p>Hello</p></body></html>";
        let formatted = handler.format_html(input);
        assert!(formatted.contains("<html>"));
        assert!(formatted.contains("<body>"));
        assert!(formatted.contains("<p>"));
    }
}
