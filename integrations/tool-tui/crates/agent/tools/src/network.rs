//! Network tool — diagnostics, mocking, connectivity.
//! Actions: dns | port_scan | mock_server | health_check | ping

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct NetworkTool;
impl Default for NetworkTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for NetworkTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "network".into(),
            description: "Network diagnostics: DNS, port scan, health check, ping, mock server"
                .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Network action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "dns".into(),
                        "port_scan".into(),
                        "health_check".into(),
                        "ping".into(),
                        "mock_server".into(),
                    ]),
                },
                ToolParameter {
                    name: "host".into(),
                    description: "Target hostname or IP".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "port".into(),
                    description: "Port number".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "timeout".into(),
                    description: "Timeout in seconds".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(json!(5)),
                    enum_values: None,
                },
            ],
            category: "network".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("ping");
        let host = call
            .arguments
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'host'"))?;

        match action {
            "dns" => {
                use tokio::net::lookup_host;
                let addr = format!("{}:80", host);
                let results: Vec<String> =
                    lookup_host(&addr).await?.map(|a| a.ip().to_string()).collect();
                Ok(ToolResult::success(call.id, results.join("\n"))
                    .with_data(json!({"addresses": results})))
            }
            "port_scan" => {
                let port = call.arguments.get("port").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                let timeout_s = call.arguments.get("timeout").and_then(|v| v.as_u64()).unwrap_or(5);
                let addr = format!("{}:{}", host, port);
                match tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_s),
                    tokio::net::TcpStream::connect(&addr),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        Ok(ToolResult::success(call.id, format!("{}:{} is OPEN", host, port)))
                    }
                    _ => Ok(ToolResult::success(
                        call.id,
                        format!("{}:{} is CLOSED/FILTERED", host, port),
                    )),
                }
            }
            "health_check" => {
                let url = if host.starts_with("http") {
                    host.to_string()
                } else {
                    format!("http://{}", host)
                };
                let start = std::time::Instant::now();
                let resp = reqwest::get(&url).await;
                let elapsed = start.elapsed().as_millis();
                match resp {
                    Ok(r) => Ok(ToolResult::success(
                        call.id,
                        format!("Status: {} ({}ms)", r.status(), elapsed),
                    )),
                    Err(e) => Ok(ToolResult::error(
                        call.id,
                        format!("Health check failed: {} ({}ms)", e, elapsed),
                    )),
                }
            }
            "ping" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let ping_cmd = if cfg!(windows) {
                    format!("ping -n 1 {}", host)
                } else {
                    format!("ping -c 1 {}", host)
                };
                let output =
                    tokio::process::Command::new(shell).arg(flag).arg(&ping_cmd).output().await?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(ToolResult::success(call.id, stdout))
            }
            "mock_server" => Ok(ToolResult::success(
                call.id,
                "Mock server requires runtime orchestration".into(),
            )),
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(NetworkTool.definition().name, "network");
    }
}
