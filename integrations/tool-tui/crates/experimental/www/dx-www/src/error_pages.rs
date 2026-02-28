//! # Error Pages
//!
//! This module provides support for custom error pages and error boundaries.
//!
//! Features:
//! - Custom 404 and 500 error pages
//! - Error boundary system for component trees
//! - Error recovery strategies

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{DxError, DxResult};
use crate::parser::{ComponentParser, ParsedComponent};

// =============================================================================
// Error Page Registry
// =============================================================================

/// Registry for custom error pages.
#[derive(Debug, Default)]
pub struct ErrorPageRegistry {
    /// Map of error codes to page paths
    pages: HashMap<u16, ErrorPageEntry>,
    /// Default error page (if any)
    default: Option<ErrorPageEntry>,
}

/// An error page entry.
#[derive(Debug, Clone)]
pub struct ErrorPageEntry {
    /// Path to the error page file
    pub path: PathBuf,
    /// Parsed component (lazy-loaded)
    component: Option<ParsedComponent>,
}

impl ErrorPageRegistry {
    /// Create a new error page registry.
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
            default: None,
        }
    }

    /// Scan a directory for error pages.
    ///
    /// Looks for:
    /// - `_error.pg` - Default error page
    /// - `_404.pg` - Custom 404 page
    /// - `_500.pg` - Custom 500 page
    pub fn scan(&mut self, pages_dir: &Path) -> DxResult<()> {
        // Check for _error.pg (default error page)
        let error_path = pages_dir.join("_error.pg");
        if error_path.exists() {
            self.default = Some(ErrorPageEntry {
                path: error_path,
                component: None,
            });
        }

        // Check for _404.pg
        let not_found_path = pages_dir.join("_404.pg");
        if not_found_path.exists() {
            self.pages.insert(
                404,
                ErrorPageEntry {
                    path: not_found_path,
                    component: None,
                },
            );
        }

        // Check for _500.pg
        let server_error_path = pages_dir.join("_500.pg");
        if server_error_path.exists() {
            self.pages.insert(
                500,
                ErrorPageEntry {
                    path: server_error_path,
                    component: None,
                },
            );
        }

        Ok(())
    }

    /// Register a custom error page.
    pub fn register(&mut self, status_code: u16, path: PathBuf) {
        self.pages.insert(
            status_code,
            ErrorPageEntry {
                path,
                component: None,
            },
        );
    }

    /// Get error page for a status code.
    pub fn get(&self, status_code: u16) -> Option<&ErrorPageEntry> {
        self.pages.get(&status_code).or(self.default.as_ref())
    }

    /// Check if a custom error page exists for a status code.
    pub fn has(&self, status_code: u16) -> bool {
        self.pages.contains_key(&status_code) || self.default.is_some()
    }

    /// Get all registered error pages.
    pub fn pages(&self) -> &HashMap<u16, ErrorPageEntry> {
        &self.pages
    }

    /// Parse and cache error page component.
    pub fn load(&mut self, status_code: u16) -> DxResult<Option<&ParsedComponent>> {
        if let Some(entry) = self.pages.get_mut(&status_code) {
            if entry.component.is_none() {
                let source =
                    std::fs::read_to_string(&entry.path).map_err(|e| DxError::IoError {
                        path: Some(entry.path.clone()),
                        message: e.to_string(),
                    })?;
                let parser = ComponentParser::new();
                let component = parser.parse(&entry.path, &source)?;
                entry.component = Some(component);
            }
            Ok(entry.component.as_ref())
        } else if let Some(ref mut entry) = self.default {
            if entry.component.is_none() {
                let source =
                    std::fs::read_to_string(&entry.path).map_err(|e| DxError::IoError {
                        path: Some(entry.path.clone()),
                        message: e.to_string(),
                    })?;
                let parser = ComponentParser::new();
                let component = parser.parse(&entry.path, &source)?;
                entry.component = Some(component);
            }
            Ok(entry.component.as_ref())
        } else {
            Ok(None)
        }
    }
}

// =============================================================================
// Error Info
// =============================================================================

/// Information about an error to pass to error pages.
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// HTTP status code
    pub status_code: u16,
    /// Error message
    pub message: String,
    /// Original error (if any)
    pub error: Option<String>,
    /// Request path
    pub path: Option<String>,
    /// Stack trace (development only)
    pub stack_trace: Option<String>,
    /// Additional context
    pub context: HashMap<String, String>,
}

impl ErrorInfo {
    /// Create error info for 404 Not Found.
    pub fn not_found(path: &str) -> Self {
        Self {
            status_code: 404,
            message: "Page not found".to_string(),
            error: None,
            path: Some(path.to_string()),
            stack_trace: None,
            context: HashMap::new(),
        }
    }

    /// Create error info for 500 Internal Server Error.
    pub fn internal_error(message: &str) -> Self {
        Self {
            status_code: 500,
            message: message.to_string(),
            error: None,
            path: None,
            stack_trace: None,
            context: HashMap::new(),
        }
    }

    /// Create from a DxError.
    pub fn from_error(error: &DxError) -> Self {
        Self {
            status_code: 500,
            message: error.to_string(),
            error: Some(format!("{:?}", error)),
            path: None,
            stack_trace: None,
            context: HashMap::new(),
        }
    }

    /// Add stack trace.
    pub fn with_stack_trace(mut self, trace: String) -> Self {
        self.stack_trace = Some(trace);
        self
    }

    /// Add context value.
    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }
}

// =============================================================================
// Error Boundary
// =============================================================================

/// Error boundary for component trees.
///
/// Catches rendering errors and displays fallback content.
#[derive(Debug)]
pub struct ErrorBoundary {
    /// Fallback content generator
    fallback: FallbackGenerator,
    /// Recovery strategy
    recovery: RecoveryStrategy,
    /// Caught errors
    errors: Vec<CaughtError>,
}

/// A caught error in an error boundary.
#[derive(Debug, Clone)]
pub struct CaughtError {
    /// Error message
    pub message: String,
    /// Component that threw the error
    pub component: Option<String>,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

/// Error boundary fallback generator.
#[derive(Debug, Clone)]
pub enum FallbackGenerator {
    /// Static HTML fallback
    Static(String),
    /// Component path to render
    Component(PathBuf),
    /// Default error message
    Default,
}

/// Error recovery strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Render fallback and continue
    Fallback,
    /// Retry rendering once
    Retry,
    /// Bubble error up
    Propagate,
    /// Reset component state
    Reset,
}

impl ErrorBoundary {
    /// Create a new error boundary.
    pub fn new() -> Self {
        Self {
            fallback: FallbackGenerator::Default,
            recovery: RecoveryStrategy::Fallback,
            errors: Vec::new(),
        }
    }

    /// Set static fallback content.
    pub fn with_fallback(mut self, html: &str) -> Self {
        self.fallback = FallbackGenerator::Static(html.to_string());
        self
    }

    /// Set component fallback.
    pub fn with_fallback_component(mut self, path: PathBuf) -> Self {
        self.fallback = FallbackGenerator::Component(path);
        self
    }

    /// Set recovery strategy.
    pub fn with_recovery(mut self, strategy: RecoveryStrategy) -> Self {
        self.recovery = strategy;
        self
    }

    /// Catch an error.
    pub fn catch(&mut self, error: &DxError, component: Option<&str>) {
        self.errors.push(CaughtError {
            message: error.to_string(),
            component: component.map(String::from),
            timestamp: std::time::SystemTime::now(),
        });
    }

    /// Check if any errors have been caught.
    pub fn has_error(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get caught errors.
    pub fn errors(&self) -> &[CaughtError] {
        &self.errors
    }

    /// Clear caught errors.
    pub fn clear(&mut self) {
        self.errors.clear();
    }

    /// Get recovery strategy.
    pub fn recovery(&self) -> RecoveryStrategy {
        self.recovery
    }

    /// Generate fallback content.
    pub fn render_fallback(&self) -> String {
        match &self.fallback {
            FallbackGenerator::Static(html) => html.clone(),
            FallbackGenerator::Component(_path) => {
                // Would load and render component
                self.default_fallback()
            }
            FallbackGenerator::Default => self.default_fallback(),
        }
    }

    /// Generate default fallback HTML.
    fn default_fallback(&self) -> String {
        let error_list = if self.errors.is_empty() {
            "<p>An unknown error occurred</p>".to_string()
        } else {
            self.errors
                .iter()
                .map(|e| format!("<li>{}</li>", escape_html(&e.message)))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Error</title>
    <style>
        body {{
            font-family: system-ui, sans-serif;
            padding: 2rem;
            max-width: 600px;
            margin: 0 auto;
        }}
        h1 {{ color: #c00; }}
        ul {{ padding-left: 1.5rem; }}
        li {{ margin: 0.5rem 0; }}
    </style>
</head>
<body>
    <h1>Something went wrong</h1>
    <ul>{}</ul>
    <p><a href="javascript:location.reload()">Try again</a></p>
</body>
</html>"#,
            error_list
        )
    }
}

impl Default for ErrorBoundary {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Default Error Pages
// =============================================================================

/// Generate default 404 page HTML.
pub fn default_404_page(path: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Page Not Found</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: system-ui, -apple-system, sans-serif;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        .container {{
            text-align: center;
            padding: 2rem;
        }}
        h1 {{
            font-size: 8rem;
            font-weight: 800;
            margin-bottom: 1rem;
            text-shadow: 4px 4px 0 rgba(0,0,0,0.1);
        }}
        h2 {{
            font-size: 1.5rem;
            font-weight: 400;
            margin-bottom: 2rem;
            opacity: 0.9;
        }}
        .path {{
            background: rgba(255,255,255,0.2);
            padding: 0.5rem 1rem;
            border-radius: 4px;
            font-family: monospace;
            font-size: 0.9rem;
            margin-bottom: 2rem;
            display: inline-block;
        }}
        a {{
            display: inline-block;
            background: white;
            color: #764ba2;
            padding: 0.75rem 2rem;
            border-radius: 50px;
            text-decoration: none;
            font-weight: 600;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        a:hover {{
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0,0,0,0.2);
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>404</h1>
        <h2>Page not found</h2>
        <div class="path">{}</div>
        <p><a href="/">← Back to Home</a></p>
    </div>
</body>
</html>"#,
        escape_html(path)
    )
}

/// Generate default 500 page HTML.
pub fn default_500_page(message: &str, show_details: bool) -> String {
    let details = if show_details {
        format!(
            r#"<div class="details">
                <h3>Error Details</h3>
                <pre>{}</pre>
            </div>"#,
            escape_html(message)
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>500 - Server Error</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: system-ui, -apple-system, sans-serif;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
            color: white;
        }}
        .container {{
            text-align: center;
            padding: 2rem;
            max-width: 600px;
        }}
        h1 {{
            font-size: 8rem;
            font-weight: 800;
            margin-bottom: 1rem;
            text-shadow: 4px 4px 0 rgba(0,0,0,0.1);
        }}
        h2 {{
            font-size: 1.5rem;
            font-weight: 400;
            margin-bottom: 2rem;
            opacity: 0.9;
        }}
        .details {{
            background: rgba(0,0,0,0.2);
            padding: 1rem;
            border-radius: 8px;
            margin-bottom: 2rem;
            text-align: left;
        }}
        .details h3 {{
            font-size: 0.9rem;
            margin-bottom: 0.5rem;
            opacity: 0.8;
        }}
        .details pre {{
            font-family: monospace;
            font-size: 0.85rem;
            white-space: pre-wrap;
            word-break: break-all;
        }}
        a {{
            display: inline-block;
            background: white;
            color: #f5576c;
            padding: 0.75rem 2rem;
            border-radius: 50px;
            text-decoration: none;
            font-weight: 600;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        a:hover {{
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0,0,0,0.2);
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>500</h1>
        <h2>Something went wrong</h2>
        {details}
        <p><a href="/">← Back to Home</a></p>
    </div>
</body>
</html>"#
    )
}

// =============================================================================
// Helpers
// =============================================================================

/// Escape HTML special characters.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_page_registry_new() {
        let registry = ErrorPageRegistry::new();
        assert!(registry.pages.is_empty());
        assert!(registry.default.is_none());
    }

    #[test]
    fn test_error_page_registry_register() {
        let mut registry = ErrorPageRegistry::new();
        registry.register(404, PathBuf::from("_404.pg"));
        assert!(registry.has(404));
        assert!(!registry.has(500));
    }

    #[test]
    fn test_error_info_not_found() {
        let info = ErrorInfo::not_found("/test");
        assert_eq!(info.status_code, 404);
        assert_eq!(info.path, Some("/test".to_string()));
    }

    #[test]
    fn test_error_info_internal_error() {
        let info = ErrorInfo::internal_error("test error");
        assert_eq!(info.status_code, 500);
        assert_eq!(info.message, "test error");
    }

    #[test]
    fn test_error_info_with_context() {
        let info = ErrorInfo::not_found("/test").with_context("key", "value");
        assert_eq!(info.context.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_error_boundary_new() {
        let boundary = ErrorBoundary::new();
        assert!(!boundary.has_error());
        assert!(boundary.errors().is_empty());
    }

    #[test]
    fn test_error_boundary_catch() {
        let mut boundary = ErrorBoundary::new();
        let error = DxError::ConfigValidationError {
            message: "test".to_string(),
            field: None,
        };
        boundary.catch(&error, Some("TestComponent"));
        assert!(boundary.has_error());
        assert_eq!(boundary.errors().len(), 1);
        assert_eq!(boundary.errors()[0].component, Some("TestComponent".to_string()));
    }

    #[test]
    fn test_error_boundary_clear() {
        let mut boundary = ErrorBoundary::new();
        let error = DxError::ConfigValidationError {
            message: "test".to_string(),
            field: None,
        };
        boundary.catch(&error, None);
        boundary.clear();
        assert!(!boundary.has_error());
    }

    #[test]
    fn test_error_boundary_recovery() {
        let boundary = ErrorBoundary::new().with_recovery(RecoveryStrategy::Retry);
        assert_eq!(boundary.recovery(), RecoveryStrategy::Retry);
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"test\""), "&quot;test&quot;");
    }

    #[test]
    fn test_default_404_page() {
        let html = default_404_page("/test");
        assert!(html.contains("404"));
        assert!(html.contains("/test"));
    }

    #[test]
    fn test_default_500_page() {
        let html = default_500_page("error", false);
        assert!(html.contains("500"));
        assert!(!html.contains("Error Details"));

        let html_detailed = default_500_page("error", true);
        assert!(html_detailed.contains("Error Details"));
    }
}
