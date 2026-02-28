//! Format tool — universal code formatting, any language, one tool.
//! Actions: format | check | config

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct FormatTool;
impl Default for FormatTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FormatTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "format".into(),
            description: "Universal code formatting: detect language, dispatch to correct formatter (rustfmt, prettier, black, gofmt, etc.)".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Format action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["format".into(),"check".into(),"config".into()]) },
                ToolParameter { name: "file".into(), description: "File to format".into(), param_type: ParameterType::String, required: true, default: None, enum_values: None },
                ToolParameter { name: "language".into(), description: "Force language".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "code_intel".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("format");
        let file = call
            .arguments
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file'"))?;
        let ext = std::path::Path::new(file).extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = call.arguments.get("language").and_then(|v| v.as_str()).unwrap_or(ext);

        let (formatter, args) = match lang {
            "rs" | "rust" => ("rustfmt", vec![file.to_string()]),
            "py" | "python" => ("black", vec![file.to_string()]),
            "go" => ("gofmt", vec!["-w".into(), file.to_string()]),
            "js" | "jsx" | "ts" | "tsx" | "json" | "css" | "html" | "md" => {
                ("prettier", vec!["--write".into(), file.to_string()])
            }
            "c" | "cpp" | "h" | "hpp" => ("clang-format", vec!["-i".into(), file.to_string()]),
            _ => ("prettier", vec!["--write".into(), file.to_string()]),
        };

        let check_flag = if action == "check" { "--check" } else { "" };
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let cmd = if check_flag.is_empty() {
            format!("{} {}", formatter, args.join(" "))
        } else {
            format!("{} {} {}", formatter, check_flag, args.join(" "))
        };

        let output = tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(ToolResult::success(call.id, format!("Formatted {} with {}", file, formatter)))
        } else {
            Ok(ToolResult::error(call.id, format!("{}\n{}", stdout, stderr)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(FormatTool.definition().name, "format");
    }
}
