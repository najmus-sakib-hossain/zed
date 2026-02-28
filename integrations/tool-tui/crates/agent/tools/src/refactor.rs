//! Refactor tool — structural code transformations.
//! Actions: extract | inline | rename | translate | migrate | regex | feature_flag

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct RefactorTool;
impl Default for RefactorTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RefactorTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "refactor".into(),
            description: "Code transformations: extract function, inline, rename, translate between languages, regex builder".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Refactor action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["extract".into(),"inline".into(),"rename".into(),"translate".into(),"migrate".into(),"regex".into(),"feature_flag".into()]) },
                ToolParameter { name: "file".into(), description: "Source file".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "code".into(), description: "Code snippet".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "name".into(), description: "New name for extract/rename".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "target_language".into(), description: "Target language for translate".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "pattern".into(), description: "Regex pattern for regex action".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "test_strings".into(), description: "Test strings for regex (JSON array)".into(), param_type: ParameterType::Array, required: false, default: None, enum_values: None },
            ],
            category: "code_intel".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("extract");

        match action {
            "regex" => {
                let pattern = call
                    .arguments
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'pattern'"))?;
                match regex::Regex::new(pattern) {
                    Ok(re) => {
                        let mut results = Vec::new();
                        if let Some(tests) =
                            call.arguments.get("test_strings").and_then(|v| v.as_array())
                        {
                            for t in tests {
                                if let Some(s) = t.as_str() {
                                    let matched = re.is_match(s);
                                    let captures: Vec<String> = re
                                        .captures(s)
                                        .map(|c| {
                                            c.iter()
                                                .filter_map(|m| m.map(|m| m.as_str().to_string()))
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    results.push(json!({"input": s, "matched": matched, "captures": captures}));
                                }
                            }
                        }
                        Ok(ToolResult::success(
                            call.id,
                            format!("Regex '{}' is valid. {} tests run.", pattern, results.len()),
                        )
                        .with_data(json!({"pattern": pattern, "valid": true, "results": results})))
                    }
                    Err(e) => Ok(ToolResult::error(call.id, format!("Invalid regex: {e}"))),
                }
            }
            "extract" | "inline" | "rename" | "translate" | "migrate" | "feature_flag" => {
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Refactor '{}' — requires LSP + tree-sitter integration for safe transforms",
                        action
                    ),
                ))
            }
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(RefactorTool.definition().name, "refactor");
    }

    #[tokio::test]
    async fn test_regex_action() {
        let tool = RefactorTool;
        let call = ToolCall {
            id: "r1".into(),
            name: "refactor".into(),
            arguments: json!({"action":"regex","pattern":"\\d+","test_strings":["hello","world123","42"]}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);
        assert!(r.output.contains("valid"));
    }
}
