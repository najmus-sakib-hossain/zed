//! Test tool — universal test runner, auto-detect framework.
//! Actions: run | watch | coverage | snapshot | list | create

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct TestTool;
impl Default for TestTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TestTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "test".into(),
            description: "Universal test runner: auto-detect language/framework, run/watch/coverage, snapshot testing, create test stubs".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Test action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["run".into(),"watch".into(),"coverage".into(),"snapshot".into(),"list".into(),"create".into()]) },
                ToolParameter { name: "path".into(), description: "Path to test file/directory".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "filter".into(), description: "Filter test names".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "package".into(), description: "Package/crate name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "execution".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("run");
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let filter = call.arguments.get("filter").and_then(|v| v.as_str());
        let package = call.arguments.get("package").and_then(|v| v.as_str());

        // Auto-detect test framework from project structure
        let framework = detect_framework(path);
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let cmd = match (action, &framework) {
            ("run", Framework::Cargo) => {
                let mut c = "cargo test".to_string();
                if let Some(p) = package {
                    c.push_str(&format!(" -p {p}"));
                }
                if let Some(f) = filter {
                    c.push_str(&format!(" {f}"));
                }
                c.push_str(" 2>&1");
                c
            }
            ("run", Framework::Jest) => {
                let mut c = "npx jest".to_string();
                if let Some(f) = filter {
                    c.push_str(&format!(" -t \"{f}\""));
                }
                c
            }
            ("run", Framework::Pytest) => {
                let mut c = "python -m pytest".to_string();
                if let Some(f) = filter {
                    c.push_str(&format!(" -k \"{f}\""));
                }
                c.push_str(&format!(" {path}"));
                c
            }
            ("run", Framework::Go) => {
                let mut c = format!("go test {}/...", path);
                if let Some(f) = filter {
                    c.push_str(&format!(" -run {f}"));
                }
                c
            }
            ("coverage", Framework::Cargo) => "cargo tarpaulin --out Html 2>&1".into(),
            ("coverage", Framework::Jest) => "npx jest --coverage".into(),
            ("coverage", Framework::Pytest) => {
                format!("python -m pytest --cov {} --cov-report=html", path)
            }
            ("list", Framework::Cargo) => "cargo test -- --list 2>&1".into(),
            ("watch", Framework::Cargo) => "cargo watch -x test 2>&1".into(),
            ("watch", Framework::Jest) => "npx jest --watch".into(),
            _ => format!("echo 'Test action {action} for {:?}'", framework),
        };

        let output = tokio::process::Command::new(shell)
            .arg(flag)
            .arg(&cmd)
            .current_dir(path)
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr).trim().to_string();
        let passed = output.status.success();
        Ok(ToolResult {
            tool_call_id: call.id,
            success: passed,
            output: combined,
            error: None,
            data: Some(
                json!({"framework": format!("{:?}", framework), "exit_code": output.status.code()}),
            ),
        })
    }
}

#[derive(Debug)]
enum Framework {
    Cargo,
    Jest,
    Pytest,
    Go,
    Unknown,
}

fn detect_framework(path: &str) -> Framework {
    let p = std::path::Path::new(path);
    if p.join("Cargo.toml").exists() {
        return Framework::Cargo;
    }
    if p.join("package.json").exists() {
        return Framework::Jest;
    }
    if p.join("pytest.ini").exists()
        || p.join("setup.py").exists()
        || p.join("pyproject.toml").exists()
    {
        return Framework::Pytest;
    }
    if p.join("go.mod").exists() {
        return Framework::Go;
    }
    Framework::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(TestTool.definition().name, "test");
    }
    #[test]
    fn test_detect() {
        assert!(matches!(detect_framework("."), Framework::Cargo | Framework::Unknown));
    }
}
