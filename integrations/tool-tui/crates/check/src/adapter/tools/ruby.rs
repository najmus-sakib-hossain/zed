//! Ruby Tool Adapters (`RuboCop`)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// `RuboCop` adapter for Ruby formatting and linting
pub struct RubocopAdapter {
    base: BaseAdapter,
}

impl RubocopAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for RubocopAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for RubocopAdapter {
    fn name(&self) -> &'static str {
        "rubocop"
    }

    fn extensions(&self) -> &[&'static str] {
        &["rb", "rake", "gemspec"]
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
        let output = self.base.run_tool(
            "rubocop",
            &[
                "--auto-correct",
                "--stdin",
                &path.to_string_lossy(),
                "--format",
                "quiet",
            ],
            Some(content),
        )?;

        let mut result = if output.exit_code == 0 || output.exit_code == 1 {
            // Extract corrected code from stdout
            // RuboCop outputs the corrected code to stdout when using --stdin
            let stdout = output.stdout_str();

            // Find the separator line and get content after it
            let formatted = if let Some(idx) = stdout.find("====================") {
                let after_separator = &stdout[idx..];
                if let Some(newline_idx) = after_separator.find('\n') {
                    after_separator[newline_idx + 1..].as_bytes().to_vec()
                } else {
                    content.to_vec()
                }
            } else {
                output.stdout.clone()
            };

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
                    rule_id: "rubocop/format-error".to_string(),
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
            "rubocop",
            &["--format", "json", "--stdin", &path.to_string_lossy()],
            Some(content),
        )?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;

        // Parse JSON output
        let stdout = output.stdout_str();
        if !stdout.trim().is_empty()
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
            && let Some(files) = json.get("files").and_then(|f| f.as_array())
        {
            for file_data in files {
                let filename = file_data
                    .get("path")
                    .and_then(|p| p.as_str())
                    .map_or_else(|| path.to_path_buf(), PathBuf::from);

                if let Some(offenses) = file_data.get("offenses").and_then(|o| o.as_array()) {
                    for offense in offenses {
                        if let Some(diag) = parse_rubocop_offense(offense, &filename) {
                            result.add_diagnostic(diag);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        self.format(path, content)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("rubocop")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("rubocop")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("rubocop")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via gem: gem install rubocop\nOr add to Gemfile: gem 'rubocop'"
    }
}

/// Parse `RuboCop` offense from JSON
fn parse_rubocop_offense(json: &serde_json::Value, file: &Path) -> Option<Diagnostic> {
    let obj = json.as_object()?;

    let message = obj.get("message")?.as_str()?;
    let cop_name = obj.get("cop_name")?.as_str()?;
    let severity_str = obj.get("severity")?.as_str()?;

    let location = obj.get("location")?;
    let start_line = location.get("start_line")?.as_u64()? as u32;
    let end_line = location
        .get("last_line")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::from(start_line)) as u32;

    let severity = match severity_str {
        "error" | "fatal" => DiagnosticSeverity::Error,
        "warning" => DiagnosticSeverity::Warning,
        "convention" | "refactor" => DiagnosticSeverity::Info,
        _ => DiagnosticSeverity::Hint,
    };

    // Get corrector suggestion if available
    let suggestion = obj
        .get("corrector")
        .and_then(|c| c.get("replacement"))
        .and_then(|r| r.as_str())
        .map(std::string::ToString::to_string);

    Some(Diagnostic {
        file: file.to_path_buf(),
        span: Span::new(start_line, end_line),
        severity,
        rule_id: format!("rubocop/{cop_name}"),
        message: message.to_string(),
        suggestion,
        related: Vec::new(),
        fix: None,
    })
}
