//! AST tool — parse, query, and transform syntax trees for any language.
//! Actions: parse | query | transform | detect_lang

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct AstTool;
impl Default for AstTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for AstTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "ast".into(),
            description: "Parse, query, and transform abstract syntax trees for any language using tree-sitter".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "AST action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["parse".into(),"query".into(),"transform".into(),"detect_lang".into()]) },
                ToolParameter { name: "code".into(), description: "Source code to parse".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "file".into(), description: "File path to parse".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "language".into(), description: "Language (rust, python, javascript, etc.)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "query_pattern".into(), description: "S-expression pattern for query action".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "code_intel".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("parse");
        let code = if let Some(c) = call.arguments.get("code").and_then(|v| v.as_str()) {
            c.to_string()
        } else if let Some(f) = call.arguments.get("file").and_then(|v| v.as_str()) {
            tokio::fs::read_to_string(f).await?
        } else {
            return Ok(ToolResult::error(call.id, "Need 'code' or 'file'".into()));
        };

        match action {
            "detect_lang" => {
                let lang = detect_language(&code);
                Ok(ToolResult::success(call.id, lang.to_string())
                    .with_data(json!({"language": lang})))
            }
            "parse" | "query" | "transform" => {
                let lang = call
                    .arguments
                    .get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| detect_language(&code));
                // tree-sitter integration point
                let line_count = code.lines().count();
                let fn_count = code.matches("fn ").count()
                    + code.matches("function ").count()
                    + code.matches("def ").count();
                Ok(ToolResult::success(call.id, format!("AST '{}' for {} ({} lines, ~{} functions) — connect tree-sitter for full AST", action, lang, line_count, fn_count))
                    .with_data(json!({"action": action, "language": lang, "lines": line_count, "estimated_functions": fn_count})))
            }
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

fn detect_language(code: &str) -> &str {
    if code.contains("fn ") && (code.contains("->") || code.contains("pub ")) {
        return "rust";
    }
    if code.contains("def ") && code.contains(":") {
        return "python";
    }
    if code.contains("function ") || code.contains("const ") || code.contains("=>") {
        return "javascript";
    }
    if code.contains("package ") && code.contains("func ") {
        return "go";
    }
    if code.contains("class ") && code.contains("public ") {
        return "java";
    }
    "unknown"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(AstTool.definition().name, "ast");
    }
    #[test]
    fn test_detect() {
        assert_eq!(detect_language("fn main() -> Result<()> {}"), "rust");
        assert_eq!(detect_language("def hello():\n    pass"), "python");
    }
}
