//! Search tool — find anything in the codebase by text, pattern, or meaning.
//!
//! Actions: content | semantic | symbol | replace

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;

use crate::definition::*;

/// Consolidated search tool — text/regex search, semantic search, symbol find, replace.
pub struct SearchTool {
    pub workspace_root: String,
}

impl SearchTool {
    pub fn new(workspace_root: impl Into<String>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search".into(),
            description: "Find anything in the codebase: text/regex search, semantic search, symbol lookup, search-and-replace".into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Search action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec!["content".into(), "semantic".into(), "symbol".into(), "replace".into()]),
                },
                ToolParameter {
                    name: "pattern".into(),
                    description: "Search pattern (regex for content, text for semantic)".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "path".into(),
                    description: "Directory or file to search in".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "glob".into(),
                    description: "File glob filter (e.g. '*.rs')".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "replacement".into(),
                    description: "Replacement string for replace action".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "max_results".into(),
                    description: "Maximum results to return".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(json!(50)),
                    enum_values: None,
                },
            ],
            category: "io".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call
            .arguments
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action'"))?;
        let pattern = call
            .arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern'"))?;
        let search_dir = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.workspace_root);
        let max = call.arguments.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        match action {
            "content" => {
                let re = regex::Regex::new(pattern).unwrap_or_else(|_| {
                    regex::Regex::new(&regex::escape(pattern)).expect("literal escape always works")
                });
                let mut results = Vec::new();
                for entry in walkdir::WalkDir::new(search_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    if results.len() >= max {
                        break;
                    }
                    let p = entry.path();
                    // skip binary files / hidden dirs
                    if p.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
                        continue;
                    }
                    if let Ok(text) = std::fs::read_to_string(p) {
                        for (i, line) in text.lines().enumerate() {
                            if re.is_match(line) {
                                results.push(json!({
                                    "file": p.to_string_lossy(),
                                    "line": i + 1,
                                    "text": line.trim(),
                                }));
                                if results.len() >= max {
                                    break;
                                }
                            }
                        }
                    }
                }
                let summary = results
                    .iter()
                    .map(|r| {
                        format!(
                            "{}:{} {}",
                            r["file"].as_str().unwrap_or(""),
                            r["line"],
                            r["text"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, summary)
                    .with_data(json!({"matches": results, "total": results.len()})))
            }
            "semantic" => {
                // Semantic search placeholder — requires embedding model connection
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Semantic search for '{}' — requires embedding model (connect via llm tool)",
                        pattern
                    ),
                ))
            }
            "symbol" => {
                // Symbol search — scan for function/struct/class definitions
                let re = regex::Regex::new(&format!(
                    r"(fn|struct|enum|trait|class|function|def|const|type)\s+{}",
                    regex::escape(pattern)
                ))
                .unwrap_or_else(|_| regex::Regex::new(pattern).expect("pattern"));
                let mut results = Vec::new();
                for entry in walkdir::WalkDir::new(search_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    if results.len() >= max {
                        break;
                    }
                    let p = entry.path();
                    if let Ok(text) = std::fs::read_to_string(p) {
                        for (i, line) in text.lines().enumerate() {
                            if re.is_match(line) {
                                results.push(json!({"file": p.to_string_lossy(), "line": i+1, "text": line.trim()}));
                                if results.len() >= max {
                                    break;
                                }
                            }
                        }
                    }
                }
                let summary = results
                    .iter()
                    .map(|r| {
                        format!(
                            "{}:{} {}",
                            r["file"].as_str().unwrap_or(""),
                            r["line"],
                            r["text"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, summary))
            }
            "replace" => {
                let replacement = call
                    .arguments
                    .get("replacement")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Need 'replacement' for replace action"))?;
                let re = regex::Regex::new(pattern)?;
                let mut count = 0usize;
                for entry in walkdir::WalkDir::new(search_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let p = entry.path();
                    if let Ok(text) = std::fs::read_to_string(p) {
                        let new_text = re.replace_all(&text, replacement);
                        if new_text != text {
                            count += 1;
                            std::fs::write(p, new_text.as_ref())?;
                        }
                    }
                }
                Ok(ToolResult::success(call.id, format!("Replaced in {} files", count)))
            }
            other => Ok(ToolResult::error(call.id, format!("Unknown search action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_definition() {
        let tool = SearchTool::new(".");
        let def = tool.definition();
        assert_eq!(def.name, "search");
    }

    #[tokio::test]
    async fn test_content_search() {
        let tool = SearchTool::new(".");
        let call = ToolCall {
            id: "s1".into(),
            name: "search".into(),
            arguments: json!({"action": "content", "pattern": "ToolDefinition", "max_results": 5}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);
    }
}
