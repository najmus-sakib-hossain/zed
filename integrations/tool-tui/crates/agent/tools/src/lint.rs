//! Lint tool — universal code linting, any language.
//! Actions: lint | fix

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct LintTool;
impl Default for LintTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LintTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "lint".into(),
            description: "Universal linting: auto-detect language, run clippy/eslint/ruff/golangci-lint/phpstan, auto-fix where possible".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Lint action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["lint".into(),"fix".into()]) },
                ToolParameter { name: "path".into(), description: "File or directory to lint".into(), param_type: ParameterType::String, required: true, default: None, enum_values: None },
                ToolParameter { name: "language".into(), description: "Force language".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "code_intel".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("lint");
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = call.arguments.get("language").and_then(|v| v.as_str()).unwrap_or(ext);
        let fix = action == "fix";

        let cmd = match lang {
            "rs" | "rust" => {
                if fix {
                    "cargo clippy --fix --allow-dirty 2>&1".into()
                } else {
                    "cargo clippy 2>&1".to_string()
                }
            }
            "py" | "python" => {
                if fix {
                    format!("ruff check --fix {}", path)
                } else {
                    format!("ruff check {}", path)
                }
            }
            "js" | "jsx" | "ts" | "tsx" | "javascript" | "typescript" => {
                if fix {
                    format!("eslint --fix {}", path)
                } else {
                    format!("eslint {}", path)
                }
            }
            "go" => format!("golangci-lint run {}", path),
            _ => format!("echo 'No linter configured for {}'", lang),
        };

        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let output = tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{}\n{}", stdout, stderr)
        };
        Ok(ToolResult::success(call.id, combined))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(LintTool.definition().name, "lint");
    }
}
