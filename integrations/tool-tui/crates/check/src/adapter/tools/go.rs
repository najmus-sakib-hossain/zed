//! Go Tool Adapters (gofmt, golangci-lint)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolErrorKind, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// Gofmt adapter for Go formatting
pub struct GofmtAdapter {
    base: BaseAdapter,
}

impl GofmtAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for GofmtAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for GofmtAdapter {
    fn name(&self) -> &'static str {
        "gofmt"
    }

    fn extensions(&self) -> &[&'static str] {
        &["go"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: true,
            can_lint: false,
            can_fix: false,
            supports_stdin: true,
            supports_config: false,
            supports_json_output: false,
            supports_caching: false,
        }
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool("gofmt", &[], Some(content))?;

        let mut result = if output.success() {
            let formatted = output.stdout.clone();
            let changed = formatted != content;
            ToolResult::with_formatted(self.name(), formatted, changed)
        } else {
            let mut result = ToolResult::success(self.name());
            let stderr = output.stderr_str();

            // Parse gofmt errors
            for line in stderr.lines() {
                if let Some(diag) = parse_gofmt_error(line, path) {
                    result.add_diagnostic(diag);
                }
            }
            result.exit_code = output.exit_code;
            result
        };

        result.set_duration(output.duration_ms);
        Ok(result)
    }

    fn lint(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "gofmt does not support linting, use golangci-lint instead",
        ))
    }

    fn is_available(&self) -> bool {
        self.base.is_available("gofmt")
    }

    fn version(&self) -> Option<String> {
        // gofmt doesn't have --version, use go version instead
        self.base
            .run_tool("go", &["version"], None)
            .ok()
            .map(|o| o.stdout_str().trim().to_string())
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("gofmt")
    }

    fn install_instructions(&self) -> &'static str {
        "Install Go from https://golang.org/dl/"
    }
}

/// Golangci-lint adapter for Go linting
pub struct GolangciLintAdapter {
    base: BaseAdapter,
}

impl GolangciLintAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for GolangciLintAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for GolangciLintAdapter {
    fn name(&self) -> &'static str {
        "golangci-lint"
    }

    fn extensions(&self) -> &[&'static str] {
        &["go"]
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
            "golangci-lint does not support formatting, use gofmt instead",
        ))
    }

    fn lint(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_on_file("golangci-lint", &["run", "--out-format=json"], path)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;

        // Parse JSON output
        let stdout = output.stdout_str();
        if !stdout.trim().is_empty()
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
            && let Some(issues) = json.get("Issues").and_then(|i| i.as_array())
        {
            for issue in issues {
                if let Some(diag) = parse_golangci_lint_issue(issue) {
                    result.add_diagnostic(diag);
                }
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_on_file("golangci-lint", &["run", "--fix"], path)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;
        result.changed = output.success();

        Ok(result)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("golangci-lint")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("golangci-lint")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("golangci-lint")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest\nOr: brew install golangci-lint"
    }
}

/// Parse gofmt error line
fn parse_gofmt_error(line: &str, default_file: &Path) -> Option<Diagnostic> {
    // Format: <stdin>:line:column: message
    // Or: file.go:line:column: message
    let parts: Vec<&str> = line.splitn(4, ':').collect();
    if parts.len() < 4 {
        return None;
    }

    let file = if parts[0] == "<stdin>" {
        default_file.to_path_buf()
    } else {
        PathBuf::from(parts[0])
    };

    let line_num = parts[1].parse::<u32>().ok()?;
    let _col = parts[2].parse::<u32>().ok()?;
    let message = parts[3].trim();

    Some(Diagnostic {
        file,
        span: Span::new(line_num, line_num),
        severity: DiagnosticSeverity::Error,
        rule_id: "gofmt/syntax".to_string(),
        message: message.to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    })
}

/// Parse golangci-lint issue from JSON
fn parse_golangci_lint_issue(json: &serde_json::Value) -> Option<Diagnostic> {
    let obj = json.as_object()?;

    let text = obj.get("Text")?.as_str()?;
    let from_linter = obj.get("FromLinter")?.as_str()?;

    let pos = obj.get("Pos")?;
    let filename = pos.get("Filename")?.as_str()?;
    let line = pos.get("Line")?.as_u64()? as u32;
    let _column = pos.get("Column").and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;

    let severity = obj.get("Severity").and_then(|s| s.as_str()).map_or(
        DiagnosticSeverity::Warning,
        |s| match s {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            _ => DiagnosticSeverity::Info,
        },
    );

    Some(Diagnostic {
        file: PathBuf::from(filename),
        span: Span::new(line, line),
        severity,
        rule_id: format!("golangci-lint/{from_linter}"),
        message: text.to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    })
}
