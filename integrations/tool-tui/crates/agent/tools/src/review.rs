//! Review tool — automated code review.
//! Actions: full_review | pr_review | suggest_fixes

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct ReviewTool;
impl Default for ReviewTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReviewTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "review".into(),
            description: "Automated code review: full review of changes, PR review with line comments, fix suggestions".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Review action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["full_review".into(),"pr_review".into(),"suggest_fixes".into()]) },
                ToolParameter { name: "diff".into(), description: "Git diff to review".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "file".into(), description: "File to review".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "pr_number".into(), description: "PR number for pr_review".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
                ToolParameter { name: "repo".into(), description: "Repository (owner/name)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "code_intel".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("full_review");
        match action {
            "full_review" => {
                if let Some(file) = call.arguments.get("file").and_then(|v| v.as_str()) {
                    let content = tokio::fs::read_to_string(file).await?;
                    let lines = content.lines().count();
                    let todos = content.matches("TODO").count();
                    let unwraps = content.matches(".unwrap()").count();
                    Ok(ToolResult::success(call.id, format!("Review of {file}: {lines} lines, {todos} TODOs, {unwraps} unwraps — connect LLM for deep review"))
                        .with_data(json!({"file": file, "lines": lines, "todos": todos, "unwraps": unwraps})))
                } else {
                    Ok(ToolResult::success(call.id, "Provide 'file' or 'diff' for review".into()))
                }
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Review '{}' — connect LLM for intelligent review", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(ReviewTool.definition().name, "review");
    }
}
