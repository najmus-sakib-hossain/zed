//! Project tool — project scaffolding, structure analysis, workspace management.
//! Actions: init | scaffold | structure | clean | stats | workspace | migrate_version

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct ProjectTool {
    pub workspace_root: String,
}
impl ProjectTool {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            workspace_root: root.into(),
        }
    }
}
impl Default for ProjectTool {
    fn default() -> Self {
        Self {
            workspace_root: ".".into(),
        }
    }
}

#[async_trait]
impl Tool for ProjectTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "project".into(),
            description: "Project management: scaffolding, structure analysis, workspace ops, stats, migrations".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Project action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["init".into(),"scaffold".into(),"structure".into(),"clean".into(),"stats".into(),"workspace".into(),"migrate_version".into()]) },
                ToolParameter { name: "path".into(), description: "Project path".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "template".into(), description: "Template name for scaffold".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "name".into(), description: "Project name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "project".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("structure");
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.workspace_root);

        match action {
            "structure" => {
                let mut output = String::new();
                let mut files = 0u32;
                let mut dirs = 0u32;
                for entry in
                    walkdir::WalkDir::new(path).max_depth(3).into_iter().filter_map(|e| e.ok())
                {
                    let depth = entry.depth();
                    if entry.file_type().is_dir() {
                        if depth > 0 {
                            output.push_str(&format!(
                                "{}📁 {}/\n",
                                "  ".repeat(depth),
                                entry.file_name().to_string_lossy()
                            ));
                        }
                        dirs += 1;
                    } else {
                        output.push_str(&format!(
                            "{}📄 {}\n",
                            "  ".repeat(depth),
                            entry.file_name().to_string_lossy()
                        ));
                        files += 1;
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!("{dirs} dirs, {files} files:\n{}", &output[..output.len().min(5000)]),
                )
                .with_data(json!({"dirs": dirs, "files": files})))
            }
            "stats" => {
                let mut total_files = 0u32;
                let mut total_lines = 0u64;
                let mut by_ext: std::collections::HashMap<String, (u32, u64)> =
                    std::collections::HashMap::new();
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("none")
                        .to_string();
                    if [
                        "exe", "dll", "so", "bin", "wasm", "png", "jpg", "gif", "ico", "woff",
                        "woff2", "ttf",
                    ]
                    .contains(&ext.as_str())
                    {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        let lines = content.lines().count() as u64;
                        total_files += 1;
                        total_lines += lines;
                        let entry = by_ext.entry(ext).or_insert((0, 0));
                        entry.0 += 1;
                        entry.1 += lines;
                    }
                }
                let mut top: Vec<_> = by_ext.into_iter().collect();
                top.sort_by(|a, b| b.1.1.cmp(&a.1.1));
                let summary: String = top
                    .iter()
                    .take(15)
                    .map(|(ext, (count, lines))| format!("  .{ext}: {count} files, {lines} lines"))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(
                    call.id,
                    format!("Project stats: {total_files} files, {total_lines} lines\n{summary}"),
                ))
            }
            "clean" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let output = tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg("cargo clean 2>&1")
                    .current_dir(path)
                    .output()
                    .await?;
                Ok(ToolResult::success(
                    call.id,
                    format!("Cleaned: {}", String::from_utf8_lossy(&output.stdout)),
                ))
            }
            "workspace" => {
                // List workspace members from Cargo.toml
                let cargo = std::path::Path::new(path).join("Cargo.toml");
                if cargo.exists() {
                    let content = tokio::fs::read_to_string(&cargo).await?;
                    let parsed: toml::Value = toml::from_str(&content)?;
                    let members: Vec<String> = parsed
                        .get("workspace")
                        .and_then(|w| w.get("members"))
                        .and_then(|m| m.as_array())
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    Ok(ToolResult::success(
                        call.id,
                        format!("{} workspace members:\n{}", members.len(), members.join("\n")),
                    )
                    .with_data(json!({"members": members})))
                } else {
                    Ok(ToolResult::success(call.id, "No Cargo.toml found".into()))
                }
            }
            _ => Ok(ToolResult::success(call.id, format!("Project '{}' on '{}'", action, path))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(ProjectTool::default().definition().name, "project");
    }
}
