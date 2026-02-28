//! PHP Tool Adapters (`PHP_CodeSniffer`)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// `PHP_CodeSniffer` adapter for PHP formatting and linting
pub struct PhpCsAdapter {
    base: BaseAdapter,
}

impl PhpCsAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for PhpCsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for PhpCsAdapter {
    fn name(&self) -> &'static str {
        "phpcs"
    }

    fn extensions(&self) -> &[&'static str] {
        &["php", "phtml", "php3", "php4", "php5", "phps"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: true,
            can_lint: true,
            can_fix: true,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: true,
            supports_caching: true,
        }
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        // Use phpcbf for formatting
        let output = self.base.run_tool(
            "phpcbf",
            &["--stdin-path", &path.to_string_lossy(), "-"],
            Some(content),
        )?;

        let mut result = if output.exit_code == 0 || output.exit_code == 1 {
            // exit code 1 means fixes were applied
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
                    rule_id: "phpcs/format-error".to_string(),
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
            "phpcs",
            &[
                "--report=json",
                "--stdin-path",
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
            && let Some(files) = json.get("files").and_then(|f| f.as_object())
        {
            for (_, file_data) in files {
                if let Some(messages) = file_data.get("messages").and_then(|m| m.as_array()) {
                    for msg in messages {
                        if let Some(diag) = parse_phpcs_message(msg, path) {
                            result.add_diagnostic(diag);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        // phpcbf is the fixer for phpcs
        self.format(path, content)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("phpcs")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("phpcs")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("phpcs")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via Composer: composer global require squizlabs/php_codesniffer\nOr: brew install php-code-sniffer (macOS)"
    }
}

/// Parse PHPCS message from JSON
fn parse_phpcs_message(json: &serde_json::Value, file: &Path) -> Option<Diagnostic> {
    let obj = json.as_object()?;

    let message = obj.get("message")?.as_str()?;
    let line = obj.get("line")?.as_u64()? as u32;
    let _column = obj.get("column").and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
    let source = obj.get("source")?.as_str()?;
    let msg_type = obj.get("type")?.as_str()?;

    let severity = match msg_type.to_uppercase().as_str() {
        "ERROR" => DiagnosticSeverity::Error,
        "WARNING" => DiagnosticSeverity::Warning,
        _ => DiagnosticSeverity::Info,
    };

    Some(Diagnostic {
        file: file.to_path_buf(),
        span: Span::new(line, line),
        severity,
        rule_id: format!("phpcs/{source}"),
        message: message.to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    })
}
