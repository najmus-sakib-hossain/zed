//! Kotlin Tool Adapters (ktlint)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// Ktlint adapter for Kotlin formatting and linting
pub struct KtlintAdapter {
    base: BaseAdapter,
}

impl KtlintAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for KtlintAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for KtlintAdapter {
    fn name(&self) -> &'static str {
        "ktlint"
    }

    fn extensions(&self) -> &[&'static str] {
        &["kt", "kts"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: true,
            can_lint: true,
            can_fix: true,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: true,
            supports_caching: false,
        }
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "ktlint",
            &["--format", "--stdin", "--log-level=none"],
            Some(content),
        )?;

        let mut result = if output.success() {
            let formatted = output.stdout.clone();
            let changed = formatted != content;
            ToolResult::with_formatted(self.name(), formatted, changed)
        } else {
            let mut result = ToolResult::success(self.name());
            let stderr = output.stderr_str();
            if !stderr.is_empty() {
                result.add_diagnostic(Diagnostic {
                    file: path.to_path_buf(),
                    span: Span::new(1, 1),
                    severity: DiagnosticSeverity::Error,
                    rule_id: "ktlint/format-error".to_string(),
                    message: stderr,
                    suggestion: None,
                    related: Vec::new(),
                    fix: None,
                });
            }
            result.exit_code = output.exit_code;
            result
        };

        result.set_duration(output.duration_ms);
        Ok(result)
    }

    fn lint(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_on_file("ktlint", &["--reporter=json"], path)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;

        // Parse JSON output
        let stdout = output.stdout_str();
        if !stdout.trim().is_empty()
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
            && let Some(array) = json.as_array()
        {
            for item in array {
                if let Some(errors) = item.get("errors").and_then(|e| e.as_array()) {
                    let filename = item
                        .get("file")
                        .and_then(|f| f.as_str())
                        .map_or_else(|| path.to_path_buf(), PathBuf::from);

                    for error in errors {
                        if let Some(diag) = parse_ktlint_error(error, &filename) {
                            result.add_diagnostic(diag);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_on_file("ktlint", &["--format"], path)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;
        result.changed = output.success();

        Ok(result)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("ktlint")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("ktlint")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("ktlint")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via: brew install ktlint (macOS)\nOr download from https://github.com/pinterest/ktlint/releases"
    }
}

/// Parse ktlint error from JSON
fn parse_ktlint_error(json: &serde_json::Value, file: &Path) -> Option<Diagnostic> {
    let obj = json.as_object()?;

    let line = obj.get("line")?.as_u64()? as u32;
    let _column = obj.get("column").and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
    let message = obj.get("message")?.as_str()?;
    let rule = obj.get("rule")?.as_str()?;

    Some(Diagnostic {
        file: file.to_path_buf(),
        span: Span::new(line, line),
        severity: DiagnosticSeverity::Warning,
        rule_id: format!("ktlint/{rule}"),
        message: message.to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    })
}
