//! Docker tool — container management via Docker CLI/API.
//! Actions: build | run | stop | logs | ps | images | compose | exec | inspect | prune

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DockerTool;
impl Default for DockerTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DockerTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "docker".into(),
            description:
                "Docker container management: build, run, stop, logs, compose, exec, inspect, prune"
                    .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Docker action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "build".into(),
                        "run".into(),
                        "stop".into(),
                        "logs".into(),
                        "ps".into(),
                        "images".into(),
                        "compose".into(),
                        "exec".into(),
                        "inspect".into(),
                        "prune".into(),
                    ]),
                },
                ToolParameter {
                    name: "image".into(),
                    description: "Image name".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "container".into(),
                    description: "Container ID/name".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "command".into(),
                    description: "Command to exec in container".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "file".into(),
                    description: "Dockerfile or compose file path".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "ports".into(),
                    description: "Port mappings (e.g. '8080:80')".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "env".into(),
                    description: "Environment variables (JSON object)".into(),
                    param_type: ParameterType::Object,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "infra".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("ps");
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let cmd = match action {
            "ps" => "docker ps --format \"table {{.ID}}\\t{{.Image}}\\t{{.Status}}\\t{{.Names}}\""
                .into(),
            "images" => {
                "docker images --format \"table {{.Repository}}\\t{{.Tag}}\\t{{.Size}}\"".into()
            }
            "build" => {
                let file =
                    call.arguments.get("file").and_then(|v| v.as_str()).unwrap_or("Dockerfile");
                let image =
                    call.arguments.get("image").and_then(|v| v.as_str()).unwrap_or("dx-app");
                format!("docker build -f {} -t {} .", file, image)
            }
            "run" => {
                let image = call
                    .arguments
                    .get("image")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'image'"))?;
                let ports = call.arguments.get("ports").and_then(|v| v.as_str());
                let mut c = format!("docker run -d");
                if let Some(p) = ports {
                    c.push_str(&format!(" -p {p}"));
                }
                c.push_str(&format!(" {image}"));
                c
            }
            "stop" => {
                let container = call
                    .arguments
                    .get("container")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'container'"))?;
                format!("docker stop {container}")
            }
            "logs" => {
                let container = call
                    .arguments
                    .get("container")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'container'"))?;
                format!("docker logs --tail 100 {container}")
            }
            "exec" => {
                let container = call
                    .arguments
                    .get("container")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'container'"))?;
                let command =
                    call.arguments.get("command").and_then(|v| v.as_str()).unwrap_or("sh");
                format!("docker exec {container} {command}")
            }
            "compose" => {
                let file = call
                    .arguments
                    .get("file")
                    .and_then(|v| v.as_str())
                    .unwrap_or("docker-compose.yml");
                let command =
                    call.arguments.get("command").and_then(|v| v.as_str()).unwrap_or("up -d");
                format!("docker compose -f {file} {command}")
            }
            "inspect" => {
                let container = call
                    .arguments
                    .get("container")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'container'"))?;
                format!("docker inspect {container}")
            }
            "prune" => "docker system prune -f".into(),
            other => return Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        };

        let output = tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            Ok(ToolResult::success(call.id, stdout))
        } else {
            Ok(ToolResult::error(call.id, if stderr.is_empty() { stdout } else { stderr }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DockerTool.definition().name, "docker");
    }
}
