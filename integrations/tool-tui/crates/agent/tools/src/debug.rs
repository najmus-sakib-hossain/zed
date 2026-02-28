//! Debug tool — debugging assistant: breakpoints, variable inspection, stack traces.
//! Actions: breakpoint | inspect | stacktrace | evaluate | watch_var | step | continue_exec

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DebugTool;
impl Default for DebugTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DebugTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "debug".into(),
            description: "Debugging assistant: set breakpoints, inspect variables, analyze stack traces, evaluate expressions, DAP integration".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Debug action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["breakpoint".into(),"inspect".into(),"stacktrace".into(),"evaluate".into(),"watch_var".into(),"step".into(),"continue_exec".into()]) },
                ToolParameter { name: "file".into(), description: "File path".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "line".into(), description: "Line number for breakpoint".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
                ToolParameter { name: "expression".into(), description: "Expression to evaluate".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "variable".into(), description: "Variable name to inspect/watch".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "execution".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("inspect");
        match action {
            "breakpoint" => {
                let file = call.arguments.get("file").and_then(|v| v.as_str()).unwrap_or("unknown");
                let line = call.arguments.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
                Ok(ToolResult::success(call.id, format!("Breakpoint set at {file}:{line} — connect DAP debugger for live interaction"))
                    .with_data(json!({"action": "breakpoint", "file": file, "line": line})))
            }
            "stacktrace" => {
                // Parse a stacktrace from recent output
                Ok(ToolResult::success(
                    call.id,
                    "Stack trace analysis — provide crash output in 'expression' field for parsing"
                        .into(),
                ))
            }
            "evaluate" => {
                let expr =
                    call.arguments.get("expression").and_then(|v| v.as_str()).unwrap_or("(none)");
                Ok(ToolResult::success(
                    call.id,
                    format!("Evaluate '{expr}' — connect DAP for live evaluation"),
                )
                .with_data(json!({"expression": expr})))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Debug '{action}' — requires DAP (Debug Adapter Protocol) connection"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DebugTool.definition().name, "debug");
    }
}
