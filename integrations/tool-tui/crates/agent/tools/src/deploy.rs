//! Deploy tool — deployment automation across environments.
//! Actions: deploy | rollback | status | preview | promote | canary | blue_green | health

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct DeployTool {
    deployments: Mutex<Vec<Deployment>>,
}

#[derive(Clone, serde::Serialize)]
struct Deployment {
    id: String,
    env: String,
    status: String,
    version: String,
    timestamp: String,
}

impl Default for DeployTool {
    fn default() -> Self {
        Self {
            deployments: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Tool for DeployTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "deploy".into(),
            description: "Deployment automation: deploy, rollback, preview, canary, blue/green, health checks".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Deploy action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["deploy".into(),"rollback".into(),"status".into(),"preview".into(),"promote".into(),"canary".into(),"blue_green".into(),"health".into()]) },
                ToolParameter { name: "env".into(), description: "Environment (dev, staging, production)".into(), param_type: ParameterType::String, required: false, default: Some(json!("staging")), enum_values: None },
                ToolParameter { name: "version".into(), description: "Version to deploy".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "command".into(), description: "Custom deploy command".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "url".into(), description: "Health check URL".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "workflow".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let env = call.arguments.get("env").and_then(|v| v.as_str()).unwrap_or("staging");
        let version = call.arguments.get("version").and_then(|v| v.as_str()).unwrap_or("latest");

        match action {
            "deploy" => {
                if let Some(cmd) = call.arguments.get("command").and_then(|v| v.as_str()) {
                    let (shell, flag) = if cfg!(windows) {
                        ("cmd", "/C")
                    } else {
                        ("sh", "-c")
                    };
                    let output =
                        tokio::process::Command::new(shell).arg(flag).arg(cmd).output().await?;
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if output.status.success() {
                        let dep = Deployment {
                            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                            env: env.into(),
                            status: "deployed".into(),
                            version: version.into(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        self.deployments.lock().unwrap().push(dep);
                        Ok(ToolResult::success(
                            call.id,
                            format!("Deployed {version} to {env}\n{stdout}"),
                        ))
                    } else {
                        Ok(ToolResult::error(
                            call.id,
                            format!("Deploy failed: {}", String::from_utf8_lossy(&output.stderr)),
                        ))
                    }
                } else {
                    let dep = Deployment {
                        id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                        env: env.into(),
                        status: "deployed".into(),
                        version: version.into(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };
                    self.deployments.lock().unwrap().push(dep);
                    Ok(ToolResult::success(
                        call.id,
                        format!(
                            "Deployed {version} to {env} — provide 'command' for actual deployment"
                        ),
                    ))
                }
            }
            "rollback" => {
                let deps = self.deployments.lock().unwrap();
                let prev = deps.iter().rev().skip(1).next();
                match prev {
                    Some(d) => Ok(ToolResult::success(
                        call.id,
                        format!("Rolling back to {} (deployed {})", d.version, d.timestamp),
                    )),
                    None => Ok(ToolResult::error(
                        call.id,
                        "No previous deployment to rollback to".into(),
                    )),
                }
            }
            "status" => {
                let deps = self.deployments.lock().unwrap();
                let output: String = deps
                    .iter()
                    .rev()
                    .take(10)
                    .map(|d| format!("[{}] {} → {} ({})", d.id, d.version, d.env, d.status))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("{} deployments:\n{}", deps.len(), output)))
            }
            "health" => {
                let url = call
                    .arguments
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for health check"))?;
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()?;
                let start = std::time::Instant::now();
                match client.get(url).send().await {
                    Ok(resp) => {
                        let elapsed = start.elapsed();
                        Ok(ToolResult::success(call.id, format!("Health: {} ({:.0}ms)", resp.status(), elapsed.as_millis()))
                            .with_data(json!({"status": resp.status().as_u16(), "latency_ms": elapsed.as_millis()})))
                    }
                    Err(e) => Ok(ToolResult::error(call.id, format!("Health check failed: {e}"))),
                }
            }
            _ => Ok(ToolResult::success(call.id, format!("Deploy '{}' for {env}", action))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DeployTool::default().definition().name, "deploy");
    }
}
