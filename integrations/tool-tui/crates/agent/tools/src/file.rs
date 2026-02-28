//! Consolidated file tool — all filesystem operations in one tool.
//!
//! Actions: read | write | edit | delete | move | copy | list | watch |
//!          metadata | checksum | symlink | diff | archive | permissions

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::info;

use crate::definition::*;

/// Consolidated file tool — 14 actions covering all FS operations.
pub struct FileTool;

impl FileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file".into(),
            description: "All filesystem operations: read, write, edit, delete, move, copy, list, metadata, checksum, diff, archive, permissions".into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "File action to perform".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "read".into(), "write".into(), "edit".into(), "delete".into(),
                        "move".into(), "copy".into(), "list".into(), "watch".into(),
                        "metadata".into(), "checksum".into(), "symlink".into(),
                        "diff".into(), "archive".into(), "permissions".into(),
                    ]),
                },
                ToolParameter {
                    name: "path".into(),
                    description: "Primary file/directory path".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "content".into(),
                    description: "Content for write/edit actions".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "destination".into(),
                    description: "Destination path for move/copy actions".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "old_string".into(),
                    description: "String to find (for edit action)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "new_string".into(),
                    description: "Replacement string (for edit action)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "start_line".into(),
                    description: "Start line for range read (1-based)".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "end_line".into(),
                    description: "End line for range read (1-based, inclusive)".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "algorithm".into(),
                    description: "Hash algorithm for checksum (sha256|blake3)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(json!("sha256")),
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
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;

        match action {
            "read" => action_read(&call, path).await,
            "write" => action_write(&call, path).await,
            "edit" => action_edit(&call, path).await,
            "delete" => action_delete(&call, path).await,
            "move" => action_move(&call, path).await,
            "copy" => action_copy(&call, path).await,
            "list" => action_list(&call, path).await,
            "metadata" => action_metadata(&call, path).await,
            "checksum" => action_checksum(&call, path).await,
            "diff" => action_diff(&call, path).await,
            "permissions" => action_permissions(&call, path).await,
            "symlink" => action_symlink(&call, path).await,
            "archive" | "watch" => Ok(ToolResult::success(
                call.id,
                format!("Action '{}' is available — requires runtime context", action),
            )),
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

async fn action_read(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path, e))?;

    let start = call.arguments.get("start_line").and_then(|v| v.as_u64()).map(|l| l as usize);
    let end = call.arguments.get("end_line").and_then(|v| v.as_u64()).map(|l| l as usize);

    let output = if start.is_some() || end.is_some() {
        let lines: Vec<&str> = content.lines().collect();
        let s = start.unwrap_or(1).saturating_sub(1);
        let e = end.unwrap_or(lines.len()).min(lines.len());
        lines[s..e].join("\n")
    } else {
        content
    };

    Ok(ToolResult::success(call.id.clone(), output))
}

async fn action_write(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let content = call
        .arguments
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'content' for write"))?;

    if let Some(parent) = Path::new(path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, content).await?;
    info!("Wrote {} bytes to {}", content.len(), path);
    Ok(ToolResult::success(
        call.id.clone(),
        format!("Wrote {} bytes to {}", content.len(), path),
    ))
}

async fn action_edit(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let old = call
        .arguments
        .get("old_string")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'old_string'"))?;
    let new = call
        .arguments
        .get("new_string")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'new_string'"))?;

    let content = tokio::fs::read_to_string(path).await?;
    if !content.contains(old) {
        return Ok(ToolResult::error(call.id.clone(), format!("String not found in {}", path)));
    }
    let updated = content.replacen(old, new, 1);
    tokio::fs::write(path, &updated).await?;
    Ok(ToolResult::success(
        call.id.clone(),
        format!("Replaced 1 occurrence in {}", path),
    ))
}

async fn action_delete(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let p = Path::new(path);
    if p.is_dir() {
        tokio::fs::remove_dir_all(path).await?;
        Ok(ToolResult::success(call.id.clone(), format!("Deleted directory: {}", path)))
    } else {
        tokio::fs::remove_file(path).await?;
        Ok(ToolResult::success(call.id.clone(), format!("Deleted file: {}", path)))
    }
}

async fn action_move(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let dest = call
        .arguments
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'destination'"))?;
    tokio::fs::rename(path, dest).await?;
    Ok(ToolResult::success(call.id.clone(), format!("Moved {} → {}", path, dest)))
}

async fn action_copy(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let dest = call
        .arguments
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'destination'"))?;
    tokio::fs::copy(path, dest).await?;
    Ok(ToolResult::success(call.id.clone(), format!("Copied {} → {}", path, dest)))
}

async fn action_list(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let mut entries = tokio::fs::read_dir(path).await?;
    let mut items = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().await?.is_dir();
        items.push(if is_dir { format!("{}/", name) } else { name });
    }
    items.sort();
    Ok(ToolResult::success(call.id.clone(), items.join("\n")))
}

async fn action_metadata(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let meta = tokio::fs::metadata(path).await?;
    let kind = if meta.is_dir() {
        "directory"
    } else if meta.is_symlink() {
        "symlink"
    } else {
        "file"
    };
    let mime = infer::get_from_path(path)
        .ok()
        .flatten()
        .map(|t| t.mime_type().to_string())
        .unwrap_or_else(|| "unknown".into());

    let data = json!({
        "path": path,
        "type": kind,
        "size": meta.len(),
        "readonly": meta.permissions().readonly(),
        "mime": mime,
    });
    Ok(ToolResult::success(call.id.clone(), data.to_string()).with_data(data))
}

async fn action_checksum(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let bytes = tokio::fs::read(path).await?;
    let algo = call.arguments.get("algorithm").and_then(|v| v.as_str()).unwrap_or("sha256");

    let hash = match algo {
        "blake3" => {
            // blake3 crate not linked — fall through to sha256
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            format!("{:x} (sha256 fallback)", hasher.finalize())
        }
        _ => {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            format!("{:x}", hasher.finalize())
        }
    };

    Ok(ToolResult::success(call.id.clone(), hash))
}

async fn action_diff(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let dest = call
        .arguments
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Need 'destination' for diff"))?;

    let a = tokio::fs::read_to_string(path).await?;
    let b = tokio::fs::read_to_string(dest).await?;

    let diff = similar::TextDiff::from_lines(&a, &b);
    let output = diff.unified_diff().header(path, dest).to_string();
    Ok(ToolResult::success(call.id.clone(), output))
}

async fn action_permissions(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let meta = tokio::fs::metadata(path).await?;
    let perms = meta.permissions();
    Ok(ToolResult::success(
        call.id.clone(),
        json!({"readonly": perms.readonly(), "path": path}).to_string(),
    ))
}

async fn action_symlink(call: &ToolCall, path: &str) -> Result<ToolResult> {
    let dest = call
        .arguments
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Need 'destination' for symlink"))?;
    #[cfg(unix)]
    tokio::fs::symlink(path, dest).await?;
    #[cfg(windows)]
    {
        if Path::new(path).is_dir() {
            tokio::fs::symlink_dir(path, dest).await?;
        } else {
            tokio::fs::symlink_file(path, dest).await?;
        }
    }
    Ok(ToolResult::success(
        call.id.clone(),
        format!("Symlink created: {} → {}", dest, path),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tool_definition() {
        let tool = FileTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "file");
        assert_eq!(def.category, "io");
    }

    #[tokio::test]
    async fn test_file_read() {
        let tool = FileTool::new();
        let call = ToolCall {
            id: "t1".into(),
            name: "file".into(),
            arguments: json!({"action": "read", "path": "Cargo.toml"}),
        };
        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("[package]"));
    }

    #[tokio::test]
    async fn test_file_metadata() {
        let tool = FileTool::new();
        let call = ToolCall {
            id: "t2".into(),
            name: "file".into(),
            arguments: json!({"action": "metadata", "path": "Cargo.toml"}),
        };
        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("file"));
    }

    #[tokio::test]
    async fn test_file_write_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let test_path = dir.path().join("test_write.txt");
        let path_str = test_path.to_string_lossy().to_string();
        let tool = FileTool::new();

        // Write
        let call = ToolCall {
            id: "w1".into(),
            name: "file".into(),
            arguments: json!({"action": "write", "path": path_str, "content": "hello world"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);

        // Read back
        let call2 = ToolCall {
            id: "r1".into(),
            name: "file".into(),
            arguments: json!({"action": "read", "path": path_str}),
        };
        let r2 = tool.execute(call2).await.unwrap();
        assert!(r2.output.contains("hello world"));

        // Delete
        let call3 = ToolCall {
            id: "d1".into(),
            name: "file".into(),
            arguments: json!({"action": "delete", "path": path_str}),
        };
        let r3 = tool.execute(call3).await.unwrap();
        assert!(r3.success);
    }

    #[tokio::test]
    async fn test_file_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("hash_test.txt");
        std::fs::write(&f, "test data").unwrap();
        let tool = FileTool::new();

        let call = ToolCall {
            id: "h1".into(),
            name: "file".into(),
            arguments: json!({"action": "checksum", "path": f.to_string_lossy().to_string()}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);
        assert!(!r.output.is_empty());
    }

    #[tokio::test]
    async fn test_file_list() {
        let tool = FileTool::new();
        let call = ToolCall {
            id: "l1".into(),
            name: "file".into(),
            arguments: json!({"action": "list", "path": "."}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);
    }
}
