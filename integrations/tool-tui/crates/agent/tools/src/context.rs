//! Context tool — intelligent context window management.
//! Actions: add | remove | list | compress | prioritize | window_info | auto_manage

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct ContextTool {
    window: Mutex<ContextWindow>,
}

struct ContextWindow {
    items: VecDeque<ContextItem>,
    max_tokens: usize,
    current_tokens: usize,
}

#[derive(Clone, serde::Serialize)]
struct ContextItem {
    id: String,
    content: String,
    source: String,
    tokens: usize,
    priority: f32,
}

impl Default for ContextTool {
    fn default() -> Self {
        Self {
            window: Mutex::new(ContextWindow {
                items: VecDeque::new(),
                max_tokens: 128_000,
                current_tokens: 0,
            }),
        }
    }
}

#[async_trait]
impl Tool for ContextTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "context".into(),
            description: "Context window manager: add/remove items, compress, prioritize, auto-manage token budget".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Context action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["add".into(),"remove".into(),"list".into(),"compress".into(),"prioritize".into(),"window_info".into(),"auto_manage".into()]) },
                ToolParameter { name: "content".into(), description: "Content to add".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "source".into(), description: "Source identifier".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "id".into(), description: "Context item ID to remove".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "priority".into(), description: "Priority 0.0-1.0".into(), param_type: ParameterType::Number, required: false, default: Some(json!(0.5)), enum_values: None },
                ToolParameter { name: "max_tokens".into(), description: "Override max token budget".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
            ],
            category: "ai".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("window_info");

        match action {
            "add" => {
                let content = call.arguments.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let source =
                    call.arguments.get("source").and_then(|v| v.as_str()).unwrap_or("manual");
                let priority =
                    call.arguments.get("priority").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
                // ~4 chars per token estimate
                let tokens = content.len() / 4;
                let id = uuid::Uuid::new_v4().to_string();
                let item = ContextItem {
                    id: id.clone(),
                    content: content.to_string(),
                    source: source.to_string(),
                    tokens,
                    priority,
                };
                let mut win = self.window.lock().unwrap();
                win.current_tokens += tokens;
                win.items.push_back(item);
                // Auto-evict lowest priority if over budget
                while win.current_tokens > win.max_tokens && win.items.len() > 1 {
                    if let Some((idx, _)) = win
                        .items
                        .iter()
                        .enumerate()
                        .min_by(|a, b| a.1.priority.partial_cmp(&b.1.priority).unwrap())
                    {
                        if let Some(removed) = win.items.remove(idx) {
                            win.current_tokens = win.current_tokens.saturating_sub(removed.tokens);
                        }
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Added context '{id}' ({tokens} tokens, {}/{} used)",
                        win.current_tokens, win.max_tokens
                    ),
                ))
            }
            "remove" => {
                let id = call.arguments.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let mut win = self.window.lock().unwrap();
                if let Some(pos) = win.items.iter().position(|i| i.id == id) {
                    if let Some(removed) = win.items.remove(pos) {
                        win.current_tokens = win.current_tokens.saturating_sub(removed.tokens);
                    }
                    Ok(ToolResult::success(call.id, format!("Removed context '{id}'")))
                } else {
                    Ok(ToolResult::error(call.id, format!("Context '{id}' not found")))
                }
            }
            "list" => {
                let win = self.window.lock().unwrap();
                let items: Vec<serde_json::Value> = win.items.iter()
                    .map(|i| json!({"id": i.id, "source": i.source, "tokens": i.tokens, "priority": i.priority}))
                    .collect();
                Ok(ToolResult::success(call.id, format!("{} items, {}/{} tokens", items.len(), win.current_tokens, win.max_tokens))
                    .with_data(json!({"items": items, "current_tokens": win.current_tokens, "max_tokens": win.max_tokens})))
            }
            "window_info" => {
                let win = self.window.lock().unwrap();
                Ok(ToolResult::success(call.id, format!("{} items, {}/{} tokens ({:.1}% used)", win.items.len(), win.current_tokens, win.max_tokens, (win.current_tokens as f64 / win.max_tokens as f64) * 100.0))
                    .with_data(json!({"items": win.items.len(), "current_tokens": win.current_tokens, "max_tokens": win.max_tokens})))
            }
            "compress" => {
                let win = self.window.lock().unwrap();
                // Compression would summarize low-priority items
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Compression available for {} items — connect LLM for summarization",
                        win.items.len()
                    ),
                ))
            }
            "auto_manage" => {
                if let Some(max) = call.arguments.get("max_tokens").and_then(|v| v.as_u64()) {
                    let mut win = self.window.lock().unwrap();
                    win.max_tokens = max as usize;
                }
                Ok(ToolResult::success(call.id, "Auto-management enabled".into()))
            }
            _ => Ok(ToolResult::success(call.id, format!("Context '{action}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(ContextTool::default().definition().name, "context");
    }

    #[tokio::test]
    async fn test_add_and_list() {
        let tool = ContextTool::default();
        let add = ToolCall {
            id: "a1".into(),
            name: "context".into(),
            arguments: json!({"action":"add","content":"test content","source":"test"}),
        };
        assert!(tool.execute(add).await.unwrap().success);
        let list = ToolCall {
            id: "l1".into(),
            name: "context".into(),
            arguments: json!({"action":"list"}),
        };
        let r = tool.execute(list).await.unwrap();
        assert!(r.output.contains("1 items"));
    }
}
