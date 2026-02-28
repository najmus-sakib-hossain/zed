//! Python Tool Adapters (ruff)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// Ruff adapter for Python formatting and linting
pub struct RuffAdapter {
    base: BaseAdapter,
}

impl RuffAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for RuffAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for RuffAdapter {
    fn name(&self) -> &'static str {
        "ruff"
    }

    fn extensions(&self) -> &[&'static str] {
        &["py", "pyi"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::full()
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "ruff",
            &["format", "--stdin-filename", &path.to_string_lossy(), "-"],
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
                    rule_id: "ruff/format-error".to_string(),
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

    fn lint(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "ruff",
            &[
                "check",
                "--output-format=json",
                "--stdin-filename",
                &path.to_string_lossy(),
                "-",
            ],
            Some(content),
        )?;

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
                if let Some(diag) = parse_ruff_diagnostic(item, path) {
                    result.add_diagnostic(diag);
                }
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "ruff",
            &[
                "check",
                "--fix",
                "--output-format=json",
                "--stdin-filename",
                &path.to_string_lossy(),
                "-",
            ],
            Some(content),
        )?;

        let mut result = if output.success() {
            let fixed = output.stdout.clone();
            let changed = fixed != content;
            ToolResult::with_formatted(self.name(), fixed, changed)
        } else {
            ToolResult::success(self.name())
        };

        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;
        Ok(result)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("ruff")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("ruff")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("ruff")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via pip: pip install ruff\nOr via pipx: pipx install ruff"
    }
}

/// Parse a Ruff JSON diagnostic
fn parse_ruff_diagnostic(json: &serde_json::Value, default_file: &Path) -> Option<Diagnostic> {
    let obj = json.as_object()?;

    let code = obj.get("code")?.as_str()?;
    let message = obj.get("message")?.as_str()?;

    let location = obj.get("location")?;
    let row = location.get("row")?.as_u64()? as u32;
    let _column = location.get("column")?.as_u64()? as u32;

    let end_location = obj.get("end_location")?;
    let end_row = end_location.get("row")?.as_u64()? as u32;
    let _end_column = end_location.get("column")?.as_u64()? as u32;

    let filename = obj
        .get("filename")
        .and_then(|f| f.as_str())
        .map_or_else(|| default_file.to_path_buf(), PathBuf::from);

    // Ruff doesn't provide severity in JSON, derive from code prefix
    let severity = match code.chars().next() {
        Some('E' | 'F') => DiagnosticSeverity::Error,
        Some('W') => DiagnosticSeverity::Warning,
        Some('I') => DiagnosticSeverity::Info,
        _ => DiagnosticSeverity::Warning,
    };

    // Get fix suggestion if available
    let suggestion = obj
        .get("fix")
        .and_then(|f| f.get("message"))
        .and_then(|m| m.as_str())
        .map(std::string::ToString::to_string);

    Some(Diagnostic {
        file: filename,
        span: Span::new(row, end_row),
        severity,
        rule_id: format!("ruff/{code}"),
        message: message.to_string(),
        suggestion,
        related: Vec::new(),
        fix: None,
    })
}
