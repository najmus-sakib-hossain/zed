//! Memory tool — persistent agent memory (vector + key-value store).
//! Actions: store | recall | search | forget | list | summarize | export | import

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct MemoryTool {
    memories: Mutex<HashMap<String, MemoryEntry>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct MemoryEntry {
    key: String,
    content: String,
    tags: Vec<String>,
    timestamp: String,
    importance: f32,
}

impl Default for MemoryTool {
    fn default() -> Self {
        Self {
            memories: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "memory".into(),
            description: "Persistent agent memory: store/recall facts, semantic search, auto-summarize, export/import knowledge base".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Memory action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["store".into(),"recall".into(),"search".into(),"forget".into(),"list".into(),"summarize".into(),"export".into(),"import".into()]) },
                ToolParameter { name: "key".into(), description: "Memory key".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "content".into(), description: "Content to store".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "query".into(), description: "Search query".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "tags".into(), description: "Tags for categorization (JSON array)".into(), param_type: ParameterType::Array, required: false, default: None, enum_values: None },
                ToolParameter { name: "importance".into(), description: "Importance score 0.0-1.0".into(), param_type: ParameterType::Number, required: false, default: Some(json!(0.5)), enum_values: None },
                ToolParameter { name: "file".into(), description: "File path for export/import".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "ai".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "store" => {
                let key = call
                    .arguments
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key'"))?;
                let content = call
                    .arguments
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'content'"))?;
                let tags: Vec<String> = call
                    .arguments
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let importance =
                    call.arguments.get("importance").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
                let entry = MemoryEntry {
                    key: key.to_string(),
                    content: content.to_string(),
                    tags,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    importance,
                };
                self.memories.lock().unwrap().insert(key.to_string(), entry);
                Ok(ToolResult::success(call.id, format!("Stored memory '{key}'")))
            }
            "recall" => {
                let key = call
                    .arguments
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key'"))?;
                let mems = self.memories.lock().unwrap();
                match mems.get(key) {
                    Some(entry) => {
                        Ok(ToolResult::success(call.id, entry.content.clone())
                            .with_data(json!(entry)))
                    }
                    None => Ok(ToolResult::error(call.id, format!("Memory '{key}' not found"))),
                }
            }
            "search" => {
                let query = call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query'"))?;
                let query_lower = query.to_lowercase();
                let mems = self.memories.lock().unwrap();
                let results: Vec<&MemoryEntry> = mems
                    .values()
                    .filter(|e| {
                        e.content.to_lowercase().contains(&query_lower)
                            || e.key.to_lowercase().contains(&query_lower)
                            || e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                    })
                    .collect();
                let output = results
                    .iter()
                    .take(20)
                    .map(|e| {
                        format!(
                            "[{}] {} (importance: {:.1})",
                            e.key,
                            &e.content[..e.content.len().min(100)],
                            e.importance
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("{} results:\n{}", results.len(), output)))
            }
            "forget" => {
                let key = call
                    .arguments
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key'"))?;
                self.memories.lock().unwrap().remove(key);
                Ok(ToolResult::success(call.id, format!("Forgot memory '{key}'")))
            }
            "list" => {
                let mems = self.memories.lock().unwrap();
                let keys: Vec<String> = mems.keys().cloned().collect();
                Ok(ToolResult::success(
                    call.id,
                    format!("{} memories: {}", keys.len(), keys.join(", ")),
                ))
            }
            "export" => {
                let file =
                    call.arguments.get("file").and_then(|v| v.as_str()).unwrap_or("memories.json");
                let json = {
                    let mems = self.memories.lock().unwrap();
                    let entries: Vec<&MemoryEntry> = mems.values().collect();
                    serde_json::to_string_pretty(&entries)?
                };
                tokio::fs::write(file, &json).await?;
                let count = serde_json::from_str::<Vec<serde_json::Value>>(&json)
                    .map(|v| v.len())
                    .unwrap_or(0);
                Ok(ToolResult::success(call.id, format!("Exported {count} memories to {file}")))
            }
            "import" => {
                let file =
                    call.arguments.get("file").and_then(|v| v.as_str()).unwrap_or("memories.json");
                let content = tokio::fs::read_to_string(file).await?;
                let entries: Vec<MemoryEntry> = serde_json::from_str(&content)?;
                let count = entries.len();
                let mut mems = self.memories.lock().unwrap();
                for entry in entries {
                    mems.insert(entry.key.clone(), entry);
                }
                Ok(ToolResult::success(call.id, format!("Imported {count} memories from {file}")))
            }
            "summarize" => {
                let mems = self.memories.lock().unwrap();
                let total = mems.len();
                let avg_importance = if total > 0 {
                    mems.values().map(|e| e.importance as f64).sum::<f64>() / total as f64
                } else {
                    0.0
                };
                let mut tag_counts: HashMap<String, usize> = HashMap::new();
                for entry in mems.values() {
                    for tag in &entry.tags {
                        *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                    }
                }
                Ok(ToolResult::success(call.id, format!("{total} memories, avg importance: {avg_importance:.2}, tags: {:?}", tag_counts))
                    .with_data(json!({"total": total, "avg_importance": avg_importance, "tag_counts": tag_counts})))
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
        assert_eq!(MemoryTool::default().definition().name, "memory");
    }

    #[tokio::test]
    async fn test_store_recall() {
        let tool = MemoryTool::default();
        let store = ToolCall {
            id: "s1".into(),
            name: "memory".into(),
            arguments: json!({"action":"store","key":"test","content":"hello world","tags":["greeting"]}),
        };
        assert!(tool.execute(store).await.unwrap().success);
        let recall = ToolCall {
            id: "r1".into(),
            name: "memory".into(),
            arguments: json!({"action":"recall","key":"test"}),
        };
        let r = tool.execute(recall).await.unwrap();
        assert!(r.output.contains("hello world"));
    }
}
