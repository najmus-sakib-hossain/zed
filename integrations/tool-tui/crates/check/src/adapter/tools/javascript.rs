//! JavaScript/TypeScript Tool Adapters (prettier, eslint)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolErrorKind, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// Prettier adapter for JS/TS/CSS/HTML formatting
pub struct PrettierAdapter {
    base: BaseAdapter,
}

impl PrettierAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for PrettierAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for PrettierAdapter {
    fn name(&self) -> &'static str {
        "prettier"
    }

    fn extensions(&self) -> &[&'static str] {
        &[
            "js", "jsx", "ts", "tsx", "mjs", "cjs", "json", "css", "scss", "less", "html", "vue",
            "svelte", "md", "yaml", "yml",
        ]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: true,
            can_lint: false,
            can_fix: false,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: false,
            supports_caching: true,
        }
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "prettier",
            &["--stdin-filepath", &path.to_string_lossy()],
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
                    rule_id: "prettier/format-error".to_string(),
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

    fn lint(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "prettier does not support linting, use eslint instead",
        ))
    }

    fn is_available(&self) -> bool {
        self.base.is_available("prettier")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("prettier")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("prettier")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via npm: npm install -g prettier\nOr locally: npm install --save-dev prettier"
    }
}

/// `ESLint` adapter for JS/TS linting
pub struct ESLintAdapter {
    base: BaseAdapter,
}

impl ESLintAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for ESLintAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for ESLintAdapter {
    fn name(&self) -> &'static str {
        "eslint"
    }

    fn extensions(&self) -> &[&'static str] {
        &["js", "jsx", "ts", "tsx", "mjs", "cjs"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: false,
            can_lint: true,
            can_fix: true,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: true,
            supports_caching: true,
        }
    }

    fn format(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "eslint does not support formatting, use prettier instead",
        ))
    }

    fn lint(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "eslint",
            &[
                "--format=json",
                "--stdin",
                "--stdin-filename",
                &path.to_string_lossy(),
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
            for file_result in array {
                if let Some(messages) = file_result.get("messages").and_then(|m| m.as_array()) {
                    let filename = file_result
                        .get("filePath")
                        .and_then(|f| f.as_str())
                        .map_or_else(|| path.to_path_buf(), PathBuf::from);

                    for msg in messages {
                        if let Some(diag) = parse_eslint_message(msg, &filename) {
                            result.add_diagnostic(diag);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        // ESLint --fix doesn't work well with stdin, so we need to write to a temp file
        // For now, just run lint and report what would be fixed
        let output = self.base.run_tool(
            "eslint",
            &[
                "--fix-dry-run",
                "--format=json",
                "--stdin",
                "--stdin-filename",
                &path.to_string_lossy(),
            ],
            Some(content),
        )?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;

        // Check if there would be fixes
        let stdout = output.stdout_str();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
            && let Some(array) = json.as_array()
        {
            for file_result in array {
                if let Some(output_content) = file_result.get("output").and_then(|o| o.as_str()) {
                    result.formatted_content = Some(output_content.as_bytes().to_vec());
                    result.changed = output_content.as_bytes() != content;
                }
            }
        }

        Ok(result)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("eslint")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("eslint")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("eslint")
    }

    fn install_instructions(&self) -> &'static str {
        "Install via npm: npm install -g eslint\nOr locally: npm install --save-dev eslint"
    }
}

/// Parse `ESLint` message from JSON
fn parse_eslint_message(json: &serde_json::Value, file: &Path) -> Option<Diagnostic> {
    let obj = json.as_object()?;

    let message = obj.get("message")?.as_str()?;
    let rule_id = obj.get("ruleId").and_then(|r| r.as_str()).unwrap_or("unknown");

    let line = obj.get("line")?.as_u64()? as u32;
    let column = obj.get("column").and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
    let end_line = obj
        .get("endLine")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::from(line)) as u32;
    let _end_column = obj
        .get("endColumn")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::from(column)) as u32;

    let severity = obj.get("severity").and_then(serde_json::Value::as_u64).map_or(
        DiagnosticSeverity::Warning,
        |s| match s {
            2 => DiagnosticSeverity::Error,
            1 => DiagnosticSeverity::Warning,
            _ => DiagnosticSeverity::Info,
        },
    );

    let suggestion = obj
        .get("fix")
        .and_then(|f| f.get("text"))
        .and_then(|t| t.as_str())
        .map(std::string::ToString::to_string);

    Some(Diagnostic {
        file: file.to_path_buf(),
        span: Span::new(line, end_line),
        severity,
        rule_id: format!("eslint/{rule_id}"),
        message: message.to_string(),
        suggestion,
        related: Vec::new(),
        fix: None,
    })
}
