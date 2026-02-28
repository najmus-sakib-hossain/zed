//! File operation tools (read, write, edit, search).

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::definition::*;

/// Read file tool
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".into(),
            description: "Read the contents of a file".into(),
            parameters: vec![
                ToolParameter {
                    name: "path".into(),
                    description: "File path to read".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "start_line".into(),
                    description: "Start line (1-based, optional)".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "end_line".into(),
                    description: "End line (1-based, inclusive, optional)".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "file".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path, e))?;

        let start_line =
            call.arguments.get("start_line").and_then(|v| v.as_u64()).map(|l| l as usize);
        let end_line = call.arguments.get("end_line").and_then(|v| v.as_u64()).map(|l| l as usize);

        let output = if start_line.is_some() || end_line.is_some() {
            let lines: Vec<&str> = content.lines().collect();
            let start = start_line.unwrap_or(1).saturating_sub(1);
            let end = end_line.unwrap_or(lines.len()).min(lines.len());
            lines[start..end].join("\n")
        } else {
            content
        };

        Ok(ToolResult::success(call.id, output))
    }
}

/// Write file tool
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".into(),
            description: "Write content to a file (creates or overwrites)".into(),
            parameters: vec![
                ToolParameter {
                    name: "path".into(),
                    description: "File path to write".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "content".into(),
                    description: "Content to write".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "file".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let content = call
            .arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content'"))?;

        // Create parent dirs
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, content).await?;
        info!("Wrote {} bytes to {}", content.len(), path);

        Ok(ToolResult::success(
            call.id,
            format!("Wrote {} bytes to {}", content.len(), path),
        ))
    }
}

/// Edit file tool (search-and-replace)
pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".into(),
            description: "Edit a file by replacing old_string with new_string".into(),
            parameters: vec![
                ToolParameter {
                    name: "path".into(),
                    description: "File path to edit".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "old_string".into(),
                    description: "Exact string to find and replace".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "new_string".into(),
                    description: "Replacement string".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "file".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let old_string = call
            .arguments
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string'"))?;
        let new_string = call
            .arguments
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string'"))?;

        let content = tokio::fs::read_to_string(path).await?;

        let count = content.matches(old_string).count();
        if count == 0 {
            return Ok(ToolResult::error(call.id, format!("String not found in {}", path)));
        }

        let new_content = content.replacen(old_string, new_string, 1);
        tokio::fs::write(path, &new_content).await?;

        Ok(ToolResult::success(call.id, format!("Replaced 1 occurrence in {}", path)))
    }
}

/// List directory tool  
pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".into(),
            description: "List files and directories in a path".into(),
            parameters: vec![ToolParameter {
                name: "path".into(),
                description: "Directory path to list".into(),
                param_type: ParameterType::String,
                required: true,
                default: None,
                enum_values: None,
            }],
            category: "file".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;

        let mut entries = tokio::fs::read_dir(path).await?;
        let mut items = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await?.is_dir();
            if is_dir {
                items.push(format!("{}/", name));
            } else {
                items.push(name);
            }
        }

        items.sort();
        Ok(ToolResult::success(call.id, items.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file() {
        let tool = ReadFileTool;
        let def = tool.definition();
        assert_eq!(def.name, "read_file");
        assert!(!def.requires_confirmation);
    }

    #[tokio::test]
    async fn test_list_dir() {
        let tool = ListDirTool;
        let call = ToolCall {
            id: "1".into(),
            name: "list_dir".into(),
            arguments: serde_json::json!({"path": "."}),
        };
        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
    }
}
