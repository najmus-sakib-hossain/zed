//! Docs tool — documentation generation and management.
//! Actions: generate | docstring | readme | changelog | api_docs | rustdoc | mdbook

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DocsTool;
impl Default for DocsTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DocsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "docs".into(),
            description:
                "Documentation: generate docstrings, READMEs, changelogs, API docs, rustdoc, mdbook"
                    .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Docs action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "generate".into(),
                        "docstring".into(),
                        "readme".into(),
                        "changelog".into(),
                        "api_docs".into(),
                        "rustdoc".into(),
                        "mdbook".into(),
                    ]),
                },
                ToolParameter {
                    name: "file".into(),
                    description: "Source file".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "path".into(),
                    description: "Project path".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(json!(".")),
                    enum_values: None,
                },
                ToolParameter {
                    name: "format".into(),
                    description: "Output format".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(json!("markdown")),
                    enum_values: None,
                },
            ],
            category: "project".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("generate");
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        match action {
            "rustdoc" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let output = tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg("cargo doc --no-deps 2>&1")
                    .current_dir(path)
                    .output()
                    .await?;
                if output.status.success() {
                    Ok(ToolResult::success(call.id, "Rustdoc generated in target/doc/".into()))
                } else {
                    Ok(ToolResult::error(
                        call.id,
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ))
                }
            }
            "readme" => {
                if let Some(file) = call.arguments.get("file").and_then(|v| v.as_str()) {
                    let content = tokio::fs::read_to_string(file).await?;
                    let fns: Vec<&str> = content
                        .lines()
                        .filter(|l| l.contains("pub fn ") || l.contains("pub async fn "))
                        .collect();
                    let structs: Vec<&str> = content
                        .lines()
                        .filter(|l| l.contains("pub struct ") || l.contains("pub enum "))
                        .collect();
                    let mut readme =
                        format!("# Module Documentation\n\n## Public API\n\n### Functions\n\n");
                    for f in &fns {
                        readme.push_str(&format!("- `{}`\n", f.trim()));
                    }
                    readme.push_str("\n### Types\n\n");
                    for s in &structs {
                        readme.push_str(&format!("- `{}`\n", s.trim()));
                    }
                    Ok(ToolResult::success(call.id, readme))
                } else {
                    Ok(ToolResult::success(call.id, "Provide 'file' to generate README for".into()))
                }
            }
            "changelog" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let output = tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg("git log --oneline --no-merges -20 2>&1")
                    .current_dir(path)
                    .output()
                    .await?;
                let commits = String::from_utf8_lossy(&output.stdout);
                let changelog = format!(
                    "# Changelog\n\n## Recent Changes\n\n{}",
                    commits.lines().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n")
                );
                Ok(ToolResult::success(call.id, changelog))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Docs '{}' — connect LLM for intelligent documentation generation", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DocsTool.definition().name, "docs");
    }
}
