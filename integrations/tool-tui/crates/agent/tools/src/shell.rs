//! Shell tool — execute terminal commands with safety.
//!
//! Actions: exec | stream | background | kill

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;
use tracing::warn;

use crate::definition::*;

/// Consolidated shell tool with exec, stream, background, kill actions.
pub struct ShellTool {
    pub cwd: Option<String>,
    pub timeout_secs: u64,
    pub blocked_patterns: Vec<String>,
}

impl Default for ShellTool {
    fn default() -> Self {
        Self {
            cwd: None,
            timeout_secs: 30,
            blocked_patterns: vec![
                "rm -rf /".into(),
                "mkfs".into(),
                "dd if=".into(),
                ":(){:|:&};:".into(),
                "format c:".into(),
            ],
        }
    }
}

impl ShellTool {
    pub fn new(cwd: Option<String>) -> Self {
        Self {
            cwd,
            ..Default::default()
        }
    }

    fn is_blocked(&self, cmd: &str) -> bool {
        self.blocked_patterns.iter().any(|b| cmd.contains(b))
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell".into(),
            description: "Execute terminal commands: exec, stream, background, kill".into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Shell action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "exec".into(),
                        "stream".into(),
                        "background".into(),
                        "kill".into(),
                    ]),
                },
                ToolParameter {
                    name: "command".into(),
                    description: "Shell command to execute".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "cwd".into(),
                    description: "Working directory".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "timeout".into(),
                    description: "Timeout in seconds".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(json!(30)),
                    enum_values: None,
                },
                ToolParameter {
                    name: "env".into(),
                    description: "Environment variables as JSON object".into(),
                    param_type: ParameterType::Object,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "io".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("exec");
        let cmd_str = call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command'"))?;

        if self.is_blocked(cmd_str) {
            return Ok(ToolResult::error(call.id, "Command blocked for safety".into()));
        }

        let cwd = call
            .arguments
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| self.cwd.clone());
        let timeout = call
            .arguments
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.timeout_secs);

        match action {
            "exec" | "stream" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let mut cmd = Command::new(shell);
                cmd.arg(flag).arg(cmd_str);
                if let Some(ref d) = cwd {
                    cmd.current_dir(d);
                }
                if let Some(env_obj) = call.arguments.get("env").and_then(|v| v.as_object()) {
                    for (k, v) in env_obj {
                        if let Some(val) = v.as_str() {
                            cmd.env(k, val);
                        }
                    }
                }

                let output =
                    tokio::time::timeout(std::time::Duration::from_secs(timeout), cmd.output())
                        .await
                        .map_err(|_| anyhow::anyhow!("Command timed out after {}s", timeout))?
                        .map_err(|e| anyhow::anyhow!("Failed to execute: {}", e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                if output.status.success() {
                    let mut result = stdout;
                    if !stderr.is_empty() {
                        result.push_str("\n[stderr]: ");
                        result.push_str(&stderr);
                    }
                    Ok(ToolResult::success(call.id, result)
                        .with_data(json!({"exit_code": exit_code})))
                } else {
                    warn!("Shell command failed (exit {}): {}", exit_code, stderr);
                    Ok(ToolResult::error(
                        call.id,
                        format!(
                            "Exit {}: {}",
                            exit_code,
                            if stderr.is_empty() { &stdout } else { &stderr }
                        ),
                    ))
                }
            }
            "background" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let mut cmd = Command::new(shell);
                cmd.arg(flag).arg(cmd_str);
                if let Some(ref d) = cwd {
                    cmd.current_dir(d);
                }
                let child = cmd.spawn()?;
                let pid = child.id().unwrap_or(0);
                Ok(ToolResult::success(
                    call.id,
                    format!("Background process started (PID: {})", pid),
                )
                .with_data(json!({"pid": pid})))
            }
            "kill" => {
                // Kill a process by PID from the command field
                let pid: u32 = cmd_str.parse().map_err(|_| {
                    anyhow::anyhow!("'command' should be a PID number for kill action")
                })?;
                #[cfg(unix)]
                {
                    use std::process::Command as StdCommand;
                    StdCommand::new("kill").arg("-9").arg(pid.to_string()).output()?;
                }
                #[cfg(windows)]
                {
                    use std::process::Command as StdCommand;
                    StdCommand::new("taskkill")
                        .arg("/PID")
                        .arg(pid.to_string())
                        .arg("/F")
                        .output()?;
                }
                Ok(ToolResult::success(call.id, format!("Killed PID {}", pid)))
            }
            other => Ok(ToolResult::error(call.id, format!("Unknown shell action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_definition() {
        let tool = ShellTool::default();
        assert_eq!(tool.definition().name, "shell");
        assert!(tool.definition().requires_confirmation);
    }

    #[test]
    fn test_blocked() {
        let tool = ShellTool::default();
        assert!(tool.is_blocked("rm -rf /"));
        assert!(!tool.is_blocked("echo hello"));
    }

    #[tokio::test]
    async fn test_shell_exec() {
        let tool = ShellTool::default();
        let call = ToolCall {
            id: "s1".into(),
            name: "shell".into(),
            arguments: json!({"action": "exec", "command": if cfg!(windows) { "echo hello" } else { "echo hello" }}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);
        assert!(r.output.contains("hello"));
    }
}
