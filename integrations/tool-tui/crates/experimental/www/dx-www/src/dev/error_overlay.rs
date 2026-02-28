//! # Error Overlay
//!
//! Displays compilation and runtime errors in the browser during development.

#![allow(dead_code)]

use crate::error::DxError;

// =============================================================================
// Error Overlay
// =============================================================================

/// Error overlay for displaying errors in the browser.
#[derive(Debug, Default)]
pub struct ErrorOverlay {
    /// Currently displayed error
    current_error: Option<ErrorInfo>,
    /// Error history
    history: Vec<ErrorInfo>,
}

impl ErrorOverlay {
    /// Create a new error overlay.
    pub fn new() -> Self {
        Self {
            current_error: None,
            history: Vec::new(),
        }
    }

    /// Show an error.
    pub fn show(&mut self, error: &DxError) {
        let info = ErrorInfo::from_error(error);
        self.history.push(info.clone());
        self.current_error = Some(info);
    }

    /// Clear the current error.
    pub fn clear(&mut self) {
        self.current_error = None;
    }

    /// Get the current error.
    pub fn current(&self) -> Option<&ErrorInfo> {
        self.current_error.as_ref()
    }

    /// Get error history.
    pub fn history(&self) -> &[ErrorInfo] {
        &self.history
    }

    /// Clear history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Generate HTML for the error overlay.
    pub fn to_html(&self) -> String {
        match &self.current_error {
            Some(error) => generate_error_html(error),
            None => String::new(),
        }
    }

    /// Generate JavaScript for the error overlay.
    pub fn to_script(&self) -> String {
        OVERLAY_SCRIPT.to_string()
    }
}

// =============================================================================
// Error Info
// =============================================================================

/// Information about an error for display.
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Error title
    pub title: String,
    /// Error message
    pub message: String,
    /// Source file path
    pub file: Option<String>,
    /// Line number
    pub line: Option<usize>,
    /// Column number
    pub column: Option<usize>,
    /// Code snippet with context
    pub code_snippet: Option<String>,
    /// Stack trace
    pub stack_trace: Option<String>,
    /// Error type
    pub error_type: ErrorType,
}

impl ErrorInfo {
    /// Create error info from a DxError.
    pub fn from_error(error: &DxError) -> Self {
        match error {
            DxError::ParseError {
                message,
                file,
                line,
                column,
                src,
                ..
            } => Self {
                title: "Parse Error".to_string(),
                message: message.clone(),
                file: Some(file.to_string_lossy().to_string()),
                line: *line,
                column: *column,
                code_snippet: src.clone(),
                stack_trace: None,
                error_type: ErrorType::Parse,
            },
            DxError::CompilationError {
                message, file, src, ..
            } => Self {
                title: "Compilation Error".to_string(),
                message: message.clone(),
                file: Some(file.to_string_lossy().to_string()),
                line: None,
                column: None,
                code_snippet: src.clone(),
                stack_trace: None,
                error_type: ErrorType::Compilation,
            },
            _ => Self {
                title: "Error".to_string(),
                message: error.to_string(),
                file: None,
                line: None,
                column: None,
                code_snippet: None,
                stack_trace: None,
                error_type: ErrorType::Unknown,
            },
        }
    }
}

/// Type of error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Parse error
    Parse,
    /// Compilation error
    Compilation,
    /// Template error
    Template,
    /// Runtime error
    Runtime,
    /// Network error
    Network,
    /// Unknown error
    Unknown,
}

// =============================================================================
// HTML Generation
// =============================================================================

/// Generate HTML for an error.
fn generate_error_html(error: &ErrorInfo) -> String {
    let file_info = match (&error.file, error.line, error.column) {
        (Some(file), Some(line), Some(col)) => format!("{} ({}:{})", file, line, col),
        (Some(file), Some(line), None) => format!("{} (line {})", file, line),
        (Some(file), None, None) => file.clone(),
        _ => String::new(),
    };

    let code_block = error
        .code_snippet
        .as_ref()
        .map(|code| format!(r#"<pre class="dx-error-code">{}</pre>"#, html_escape(code)))
        .unwrap_or_default();

    let stack_block = error
        .stack_trace
        .as_ref()
        .map(|stack| format!(r#"<pre class="dx-error-stack">{}</pre>"#, html_escape(stack)))
        .unwrap_or_default();

    format!(
        r#"
<div id="dx-error-overlay" class="dx-error-overlay">
    <div class="dx-error-container">
        <div class="dx-error-header">
            <span class="dx-error-icon">⚠️</span>
            <h1 class="dx-error-title">{}</h1>
            <button class="dx-error-close" onclick="window.__DX_HIDE_ERROR__()">×</button>
        </div>
        <div class="dx-error-body">
            <p class="dx-error-message">{}</p>
            {}
            {}
            {}
        </div>
        <div class="dx-error-footer">
            <span class="dx-error-hint">Fix the error and save to reload</span>
        </div>
    </div>
</div>
"#,
        html_escape(&error.title),
        html_escape(&error.message),
        if !file_info.is_empty() {
            format!(r#"<p class="dx-error-file">{}</p>"#, html_escape(&file_info))
        } else {
            String::new()
        },
        code_block,
        stack_block
    )
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// =============================================================================
// Overlay Script
// =============================================================================

/// JavaScript for the error overlay.
const OVERLAY_SCRIPT: &str = r#"
(function() {
    window.__DX_SHOW_ERROR__ = function(message) {
        let overlay = document.getElementById('dx-error-overlay');
        if (!overlay) {
            overlay = document.createElement('div');
            overlay.id = 'dx-error-overlay';
            overlay.className = 'dx-error-overlay';
            document.body.appendChild(overlay);
        }
        overlay.innerHTML = `
            <div class="dx-error-container">
                <div class="dx-error-header">
                    <span class="dx-error-icon">⚠️</span>
                    <h1 class="dx-error-title">Error</h1>
                    <button class="dx-error-close" onclick="window.__DX_HIDE_ERROR__()">×</button>
                </div>
                <div class="dx-error-body">
                    <p class="dx-error-message">${message}</p>
                </div>
            </div>
        `;
        overlay.style.display = 'flex';
    };
    
    window.__DX_HIDE_ERROR__ = function() {
        const overlay = document.getElementById('dx-error-overlay');
        if (overlay) {
            overlay.style.display = 'none';
        }
    };
})();
"#;

/// CSS for the error overlay.
pub const OVERLAY_STYLES: &str = r#"
.dx-error-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 999999;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.dx-error-container {
    background: #1a1a1a;
    border-radius: 8px;
    max-width: 800px;
    width: 90%;
    max-height: 90vh;
    overflow: auto;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
}

.dx-error-header {
    display: flex;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #333;
    background: #ff5555;
    border-radius: 8px 8px 0 0;
}

.dx-error-icon {
    font-size: 24px;
    margin-right: 12px;
}

.dx-error-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: white;
    flex: 1;
}

.dx-error-close {
    background: none;
    border: none;
    color: white;
    font-size: 24px;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    opacity: 0.8;
}

.dx-error-close:hover {
    opacity: 1;
}

.dx-error-body {
    padding: 20px;
}

.dx-error-message {
    margin: 0 0 16px 0;
    color: #ff8888;
    font-size: 16px;
    line-height: 1.5;
}

.dx-error-file {
    margin: 0 0 16px 0;
    color: #888;
    font-size: 14px;
}

.dx-error-code {
    background: #0d0d0d;
    padding: 16px;
    border-radius: 4px;
    overflow-x: auto;
    font-family: 'Fira Code', 'Monaco', 'Consolas', monospace;
    font-size: 14px;
    line-height: 1.5;
    color: #e0e0e0;
    margin: 0 0 16px 0;
}

.dx-error-stack {
    background: #0d0d0d;
    padding: 16px;
    border-radius: 4px;
    overflow-x: auto;
    font-family: 'Fira Code', 'Monaco', 'Consolas', monospace;
    font-size: 12px;
    line-height: 1.5;
    color: #888;
    margin: 0;
}

.dx-error-footer {
    padding: 12px 20px;
    border-top: 1px solid #333;
    text-align: center;
}

.dx-error-hint {
    color: #666;
    font-size: 14px;
}
"#;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_overlay_new() {
        let overlay = ErrorOverlay::new();
        assert!(overlay.current().is_none());
        assert!(overlay.history().is_empty());
    }

    #[test]
    fn test_error_overlay_show_clear() {
        let mut overlay = ErrorOverlay::new();

        let error = DxError::ConfigValidationError {
            message: "test error".to_string(),
            field: None,
        };
        overlay.show(&error);

        assert!(overlay.current().is_some());
        assert_eq!(overlay.history().len(), 1);

        overlay.clear();
        assert!(overlay.current().is_none());
        assert_eq!(overlay.history().len(), 1); // History preserved
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"test\""), "&quot;test&quot;");
    }

    #[test]
    fn test_error_info_from_config_error() {
        let error = DxError::ConfigValidationError {
            message: "invalid config".to_string(),
            field: None,
        };
        let info = ErrorInfo::from_error(&error);
        assert_eq!(info.title, "Error");
        assert!(info.message.contains("invalid config"));
    }
}
