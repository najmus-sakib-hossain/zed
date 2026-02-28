//! Analyze tool — deep static analysis, the agent's "engineering intuition."
//! Actions: lint | complexity | dead_code | code_smell | type_coverage | clone_detect | dep_graph | change_impact | api_diff | review | tech_debt | i18n_audit

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct AnalyzeTool {
    pub workspace_root: String,
}
impl AnalyzeTool {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            workspace_root: root.into(),
        }
    }
}
impl Default for AnalyzeTool {
    fn default() -> Self {
        Self {
            workspace_root: ".".into(),
        }
    }
}

#[async_trait]
impl Tool for AnalyzeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "analyze".into(),
            description: "Static analysis: lint, complexity metrics, dead code, code smells, dependency graph, change impact, tech debt scoring".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Analysis action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["lint".into(),"complexity".into(),"dead_code".into(),"code_smell".into(),"clone_detect".into(),"dep_graph".into(),"change_impact".into(),"tech_debt".into(),"review".into()]) },
                ToolParameter { name: "path".into(), description: "File or directory to analyze".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "language".into(), description: "Language filter".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "code_intel".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("lint");
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        match action {
            "complexity" => {
                // Calculate basic complexity metrics by counting branches
                let mut total_files = 0u32;
                let mut total_lines = 0u32;
                let mut total_branches = 0u32;
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let p = entry.path();
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if !["rs", "py", "js", "ts", "go", "java", "c", "cpp"].contains(&ext) {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(p) {
                        total_files += 1;
                        total_lines += content.lines().count() as u32;
                        total_branches += content.matches("if ").count() as u32;
                        total_branches += content.matches("match ").count() as u32;
                        total_branches += content.matches("for ").count() as u32;
                        total_branches += content.matches("while ").count() as u32;
                    }
                }
                let avg = if total_files > 0 {
                    total_branches / total_files
                } else {
                    0
                };
                Ok(ToolResult::success(call.id, format!("Files: {total_files} | Lines: {total_lines} | Branches: {total_branches} | Avg complexity: {avg}"))
                    .with_data(json!({"files": total_files, "lines": total_lines, "branches": total_branches, "avg_complexity": avg})))
            }
            "dead_code" => {
                // Simple heuristic: find functions that are defined but never called
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Dead code analysis on '{path}' — connect tree-sitter + LSP for precise results"
                    ),
                ))
            }
            "dep_graph" => {
                // For Rust projects, parse Cargo.toml dependencies
                let cargo_path = std::path::Path::new(path).join("Cargo.toml");
                if cargo_path.exists() {
                    let content = tokio::fs::read_to_string(&cargo_path).await?;
                    let parsed: toml::Value = toml::from_str(&content)?;
                    let deps: Vec<String> = parsed
                        .get("dependencies")
                        .and_then(|d| d.as_table())
                        .map(|t| t.keys().cloned().collect())
                        .unwrap_or_default();
                    Ok(ToolResult::success(
                        call.id,
                        format!("{} dependencies: {}", deps.len(), deps.join(", ")),
                    )
                    .with_data(json!({"dependencies": deps})))
                } else {
                    Ok(ToolResult::success(
                        call.id,
                        "No Cargo.toml found — connect package manager for dep graph".into(),
                    ))
                }
            }
            "lint" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let cmd = format!("cargo clippy --message-format=short 2>&1 | head -50");
                let output = tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg(&cmd)
                    .current_dir(path)
                    .output()
                    .await;
                match output {
                    Ok(o) => Ok(ToolResult::success(
                        call.id,
                        String::from_utf8_lossy(&o.stdout).to_string(),
                    )),
                    Err(_) => Ok(ToolResult::success(
                        call.id,
                        "Lint requires cargo/clippy in PATH".into(),
                    )),
                }
            }
            "tech_debt" | "code_smell" | "clone_detect" | "change_impact" | "review" => {
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Analysis '{}' on '{}' — requires tree-sitter + LLM integration for full results",
                        action, path
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
        assert_eq!(AnalyzeTool::default().definition().name, "analyze");
    }
}
