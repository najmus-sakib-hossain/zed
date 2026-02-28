//! Rust Tool Adapters (rustfmt, clippy)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolErrorKind, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// Rustfmt adapter for Rust formatting
pub struct RustfmtAdapter {
    base: BaseAdapter,
}

impl RustfmtAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for RustfmtAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for RustfmtAdapter {
    fn name(&self) -> &'static str {
        "rustfmt"
    }

    fn extensions(&self) -> &[&'static str] {
        &["rs"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: true,
            can_lint: false,
            can_fix: false,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: false,
            supports_caching: false,
        }
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool("rustfmt", &["--edition", "2021"], Some(content))?;

        let mut result = if output.success() {
            let formatted = output.stdout.clone();
            let changed = formatted != content;
            ToolResult::with_formatted(self.name(), formatted, changed)
        } else {
            // Parse error output
            let stderr = output.stderr_str();
            let mut result = ToolResult::success(self.name());

            // Parse rustfmt error output
            for line in stderr.lines() {
                if line.contains("error") || line.contains("Error") {
                    result.add_diagnostic(Diagnostic {
                        file: path.to_path_buf(),
                        span: Span::new(1, 1),
                        severity: DiagnosticSeverity::Error,
                        rule_id: "rustfmt/format-error".to_string(),
                        message: line.to_string(),
                        suggestion: None,
                        related: Vec::new(),
                        fix: None,
                    });
                }
            }
            result.exit_code = output.exit_code;
            result
        };

        result.set_duration(output.duration_ms);
        Ok(result)
    }

    fn lint(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        // rustfmt doesn't lint, only formats
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "rustfmt does not support linting, use clippy instead",
        ))
    }

    fn is_available(&self) -> bool {
        self.base.is_available("rustfmt")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("rustfmt")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("rustfmt")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via rustup: rustup component add rustfmt"
    }
}

/// Clippy adapter for Rust linting
pub struct ClippyAdapter {
    base: BaseAdapter,
}

impl ClippyAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for ClippyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for ClippyAdapter {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn extensions(&self) -> &[&'static str] {
        &["rs"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: false,
            can_lint: true,
            can_fix: true,
            supports_stdin: false,
            supports_config: true,
            supports_json_output: true,
            supports_caching: true,
        }
    }

    fn format(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "clippy does not support formatting, use rustfmt instead",
        ))
    }

    fn lint(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        // Clippy runs on the whole crate, not individual files
        // For now, we'll run it with JSON output
        let output =
            self.base.run_tool("cargo", &["clippy", "--message-format=json", "-q"], None)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;

        // Parse JSON output (cargo outputs one JSON per line)
        let stdout = output.stdout_str();
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(diag) = parse_clippy_diagnostic(&json, path)
            {
                result.add_diagnostic(diag);
            }
        }

        Ok(result)
    }

    fn fix(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "cargo",
            &["clippy", "--fix", "--allow-dirty", "--allow-staged", "-q"],
            None,
        )?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;
        result.changed = output.success();

        Ok(result)
    }

    fn is_available(&self) -> bool {
        // Check for cargo clippy
        self.base.is_available("cargo")
    }

    fn version(&self) -> Option<String> {
        self.base
            .run_tool("cargo", &["clippy", "--version"], None)
            .ok()
            .map(|o| o.stdout_str().trim().to_string())
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("cargo")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via rustup: rustup component add clippy"
    }
}

/// Parse a Clippy JSON diagnostic
fn parse_clippy_diagnostic(json: &serde_json::Value, _default_file: &Path) -> Option<Diagnostic> {
    let reason = json.get("reason")?.as_str()?;
    if reason != "compiler-message" {
        return None;
    }

    let message = json.get("message")?;
    let msg_text = message.get("message")?.as_str()?;

    let level = message.get("level")?.as_str()?;
    let severity = match level {
        "error" => DiagnosticSeverity::Error,
        "warning" => DiagnosticSeverity::Warning,
        "note" | "help" => DiagnosticSeverity::Info,
        _ => DiagnosticSeverity::Hint,
    };

    // Get code/rule
    let code = message
        .get("code")
        .and_then(|c| c.get("code"))
        .and_then(|c| c.as_str())
        .unwrap_or("unknown");

    // Get span info
    let spans = message.get("spans")?.as_array()?;
    let primary_span = spans
        .iter()
        .find(|s| s.get("is_primary").and_then(serde_json::Value::as_bool).unwrap_or(false))
        .or_else(|| spans.first())?;

    let file_name = primary_span.get("file_name")?.as_str()?;
    let line_start = primary_span.get("line_start")?.as_u64()? as u32;
    let line_end = primary_span.get("line_end")?.as_u64()? as u32;

    // Get suggestion if available
    let suggestion = spans
        .iter()
        .find_map(|s| s.get("suggested_replacement").and_then(|r| r.as_str()))
        .map(std::string::ToString::to_string);

    Some(Diagnostic {
        file: PathBuf::from(file_name),
        span: Span::new(line_start, line_end),
        severity,
        rule_id: format!("clippy/{code}"),
        message: msg_text.to_string(),
        suggestion,
        related: Vec::new(),
        fix: None,
    })
}
