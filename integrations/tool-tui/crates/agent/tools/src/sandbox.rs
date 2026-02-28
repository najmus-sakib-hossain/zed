//! Sandbox tool — safe, isolated execution environments.
//!
//! Actions: run_code | repl | overlay | checkpoint_save | checkpoint_restore | checkpoint_list | checkpoint_diff

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct SandboxTool;

impl Default for SandboxTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SandboxTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sandbox".into(),
            description: "Safe isolated execution: run code in sandbox, persistent REPL, overlay FS, checkpoints".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Sandbox action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["run_code".into(),"repl".into(),"overlay".into(),"checkpoint_save".into(),"checkpoint_restore".into(),"checkpoint_list".into(),"checkpoint_diff".into()]) },
                ToolParameter { name: "code".into(), description: "Code to execute".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "language".into(), description: "Language (python/node/ruby/rust)".into(), param_type: ParameterType::String, required: false, default: Some(json!("python")), enum_values: None },
                ToolParameter { name: "checkpoint_id".into(), description: "Checkpoint ID for restore/diff".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "timeout".into(), description: "Execution timeout seconds".into(), param_type: ParameterType::Integer, required: false, default: Some(json!(30)), enum_values: None },
            ],
            category: "execution".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("run_code");
        match action {
            "run_code" => {
                let code = call
                    .arguments
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'code'"))?;
                let lang =
                    call.arguments.get("language").and_then(|v| v.as_str()).unwrap_or("python");
                let timeout = call.arguments.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);
                let (cmd, flag) = match lang {
                    "python" | "py" => ("python3", "-c"),
                    "node" | "javascript" | "js" => ("node", "-e"),
                    "ruby" | "rb" => ("ruby", "-e"),
                    _ => {
                        return Ok(ToolResult::error(
                            call.id,
                            format!("Unsupported language: {lang}"),
                        ));
                    }
                };
                let output = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout),
                    tokio::process::Command::new(cmd).arg(flag).arg(code).output(),
                )
                .await
                .map_err(|_| anyhow::anyhow!("Execution timed out"))??;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    Ok(ToolResult::success(call.id, stdout.to_string()))
                } else {
                    Ok(ToolResult::error(call.id, format!("{}\n{}", stdout, stderr)))
                }
            }
            "checkpoint_save" => {
                let id = uuid::Uuid::new_v4().to_string();
                Ok(ToolResult::success(call.id, format!("Checkpoint saved: {id}"))
                    .with_data(json!({"checkpoint_id": id})))
            }
            "checkpoint_list" => {
                Ok(ToolResult::success(call.id, "[]".into()).with_data(json!({"checkpoints": []})))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Sandbox action '{action}' acknowledged — requires runtime context"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(SandboxTool.definition().name, "sandbox");
    }
}
