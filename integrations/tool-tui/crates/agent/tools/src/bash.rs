//! Bash/shell command execution tool.

use anyhow::Result;
use async_trait::async_trait;
use tokio::process::Command;
use tracing::warn;

use crate::definition::*;

/// Shell command execution tool
pub struct BashTool {
    /// Working directory
    pub cwd: Option<String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Blocked commands
    pub blocked_commands: Vec<String>,
}

impl Default for BashTool {
    fn default() -> Self {
        Self {
            cwd: None,
            timeout_secs: 30,
            blocked_commands: vec![
                "rm -rf /".into(),
                "mkfs".into(),
                "dd if=".into(),
                ":(){:|:&};:".into(), // Fork bomb
            ],
        }
    }
}

impl BashTool {
    pub fn new(cwd: Option<String>, timeout_secs: u64) -> Self {
        Self {
            cwd,
            timeout_secs,
            ..Default::default()
        }
    }

    fn is_blocked(&self, command: &str) -> bool {
        self.blocked_commands.iter().any(|blocked| command.contains(blocked))
    }
}

#[async_trait]
impl Tool for BashTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".into(),
            description: "Execute a shell command and return stdout/stderr".into(),
            parameters: vec![
                ToolParameter {
                    name: "command".into(),
                    description: "The shell command to execute".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "cwd".into(),
                    description: "Working directory (optional)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "timeout".into(),
                    description: "Timeout in seconds (default: 30)".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(serde_json::json!(30)),
                    enum_values: None,
                },
            ],
            category: "shell".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let command = call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        // Safety check
        if self.is_blocked(command) {
            return Ok(ToolResult::error(call.id, "Command blocked for safety reasons".into()));
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

        // Determine shell
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut cmd = Command::new(shell);
        cmd.arg(flag).arg(command);

        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }

        let output = tokio::time::timeout(std::time::Duration::from_secs(timeout), cmd.output())
            .await
            .map_err(|_| anyhow::anyhow!("Command timed out after {}s", timeout))?
            .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let exit_code = output.status.code().unwrap_or(-1);

        if output.status.success() {
            let mut result = stdout;
            if !stderr.is_empty() {
                result.push_str("\n[stderr]:\n");
                result.push_str(&stderr);
            }
            Ok(ToolResult::success(call.id, result))
        } else {
            let error_msg = if stderr.is_empty() {
                format!("Command exited with code {}", exit_code)
            } else {
                format!("Exit code {}: {}", exit_code, stderr)
            };
            warn!("Command failed: {}", error_msg);
            Ok(ToolResult::error(call.id, error_msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_commands() {
        let tool = BashTool::default();
        assert!(tool.is_blocked("rm -rf /"));
        assert!(tool.is_blocked("mkfs.ext4 /dev/sda"));
        assert!(!tool.is_blocked("echo hello"));
        assert!(!tool.is_blocked("ls -la"));
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool::default();
        let call = ToolCall {
            id: "1".into(),
            name: "bash".into(),
            arguments: if cfg!(windows) {
                serde_json::json!({"command": "echo hello"})
            } else {
                serde_json::json!({"command": "echo hello"})
            },
        };

        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }
}
