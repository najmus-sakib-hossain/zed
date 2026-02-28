//! Package tool — universal package manager (cargo, npm, pip, go, etc.).
//! Actions: install | remove | update | list | search | audit | publish | outdated | lock

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct PackageTool;
impl Default for PackageTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for PackageTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "package".into(),
            description: "Universal package manager: install/remove/update/audit dependencies across cargo/npm/pip/go".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Package action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["install".into(),"remove".into(),"update".into(),"list".into(),"search".into(),"audit".into(),"publish".into(),"outdated".into(),"lock".into()]) },
                ToolParameter { name: "name".into(), description: "Package name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "version".into(), description: "Package version".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "manager".into(), description: "Package manager (auto, cargo, npm, pip, go)".into(), param_type: ParameterType::String, required: false, default: Some(json!("auto")), enum_values: None },
                ToolParameter { name: "dev".into(), description: "Install as dev dependency".into(), param_type: ParameterType::Boolean, required: false, default: Some(json!(false)), enum_values: None },
            ],
            category: "infra".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        let manager = call.arguments.get("manager").and_then(|v| v.as_str()).unwrap_or("auto");
        let name = call.arguments.get("name").and_then(|v| v.as_str());

        // Auto-detect package manager
        let pm = if manager != "auto" {
            manager.to_string()
        } else {
            detect_pm()
        };
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let cmd = match (action, pm.as_str()) {
            ("install", "cargo") => {
                let n = name.ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                let v = call.arguments.get("version").and_then(|v| v.as_str());
                match v {
                    Some(ver) => format!("cargo add {}@{}", n, ver),
                    None => format!("cargo add {}", n),
                }
            }
            ("install", "npm") => {
                let n = name.ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                let dev = call.arguments.get("dev").and_then(|v| v.as_bool()).unwrap_or(false);
                if dev {
                    format!("npm install -D {n}")
                } else {
                    format!("npm install {n}")
                }
            }
            ("install", "pip") => {
                let n = name.ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                format!("pip install {n}")
            }
            ("remove", "cargo") => format!("cargo remove {}", name.unwrap_or("?")),
            ("remove", "npm") => format!("npm uninstall {}", name.unwrap_or("?")),
            ("remove", "pip") => format!("pip uninstall -y {}", name.unwrap_or("?")),
            ("update", "cargo") => "cargo update".into(),
            ("update", "npm") => "npm update".into(),
            ("update", "pip") => "pip list --outdated --format=json".into(),
            ("list", "cargo") => "cargo tree --depth 1".into(),
            ("list", "npm") => "npm list --depth=0".into(),
            ("list", "pip") => "pip list --format=columns".into(),
            ("search", "cargo") => format!("cargo search {}", name.unwrap_or("dx")),
            ("search", "npm") => format!("npm search {}", name.unwrap_or("dx")),
            ("audit", "cargo") => "cargo audit 2>&1".into(),
            ("audit", "npm") => "npm audit".into(),
            ("outdated", "cargo") => "cargo outdated 2>&1".into(),
            ("outdated", "npm") => "npm outdated".into(),
            ("publish", "cargo") => "cargo publish --dry-run".into(),
            ("publish", "npm") => "npm publish --dry-run".into(),
            ("lock", "cargo") => "cargo generate-lockfile".into(),
            ("lock", "npm") => "npm ci".into(),
            _ => format!("echo 'Package action {action} for {pm}'"),
        };

        let output = tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n{stderr}")
        };
        Ok(ToolResult {
            tool_call_id: call.id,
            success: output.status.success(),
            output: combined,
            error: None,
            data: Some(json!({"manager": pm, "action": action})),
        })
    }
}

fn detect_pm() -> String {
    if std::path::Path::new("Cargo.toml").exists() {
        return "cargo".into();
    }
    if std::path::Path::new("package.json").exists() {
        return "npm".into();
    }
    if std::path::Path::new("requirements.txt").exists()
        || std::path::Path::new("pyproject.toml").exists()
    {
        return "pip".into();
    }
    if std::path::Path::new("go.mod").exists() {
        return "go".into();
    }
    "cargo".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(PackageTool.definition().name, "package");
    }
    #[test]
    fn test_detect_pm() {
        assert_eq!(detect_pm(), "cargo");
    }
}
