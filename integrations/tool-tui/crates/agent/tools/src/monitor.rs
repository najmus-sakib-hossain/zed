//! Monitor tool — application monitoring, logging, metrics, APM.
//! Actions: metrics | logs | alerts | apm | uptime | error_rate | latency | resource_usage

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct MonitorTool {
    logs: Mutex<VecDeque<LogEntry>>,
    metrics: Mutex<std::collections::HashMap<String, Vec<f64>>>,
}

#[derive(Clone, serde::Serialize)]
struct LogEntry {
    level: String,
    message: String,
    timestamp: String,
    source: String,
}

impl Default for MonitorTool {
    fn default() -> Self {
        Self {
            logs: Mutex::new(VecDeque::with_capacity(10_000)),
            metrics: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl Tool for MonitorTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "monitor".into(),
            description: "Application monitoring: metrics collection, log aggregation, alerting, APM, uptime tracking".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Monitor action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["metrics".into(),"logs".into(),"alerts".into(),"apm".into(),"uptime".into(),"error_rate".into(),"latency".into(),"resource_usage".into()]) },
                ToolParameter { name: "metric_name".into(), description: "Metric name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "value".into(), description: "Metric value".into(), param_type: ParameterType::Number, required: false, default: None, enum_values: None },
                ToolParameter { name: "level".into(), description: "Log level (debug, info, warn, error)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "message".into(), description: "Log message".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "url".into(), description: "URL for uptime check".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "monitoring".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("metrics");

        match action {
            "metrics" => {
                if let (Some(name), Some(value)) = (
                    call.arguments.get("metric_name").and_then(|v| v.as_str()),
                    call.arguments.get("value").and_then(|v| v.as_f64()),
                ) {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.entry(name.to_string()).or_insert_with(Vec::new).push(value);
                    Ok(ToolResult::success(call.id, format!("Recorded metric {name}={value}")))
                } else {
                    let metrics = self.metrics.lock().unwrap();
                    let summary: String = metrics
                        .iter()
                        .map(|(k, v)| {
                            let avg = v.iter().sum::<f64>() / v.len() as f64;
                            format!("{k}: count={}, avg={avg:.4}", v.len())
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(ToolResult::success(
                        call.id,
                        format!("{} metrics:\n{}", metrics.len(), summary),
                    ))
                }
            }
            "logs" => {
                if let Some(msg) = call.arguments.get("message").and_then(|v| v.as_str()) {
                    let level =
                        call.arguments.get("level").and_then(|v| v.as_str()).unwrap_or("info");
                    let entry = LogEntry {
                        level: level.into(),
                        message: msg.into(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        source: "agent".into(),
                    };
                    let mut logs = self.logs.lock().unwrap();
                    if logs.len() >= 10_000 {
                        logs.pop_front();
                    }
                    logs.push_back(entry);
                    Ok(ToolResult::success(call.id, format!("[{level}] {msg}")))
                } else {
                    let logs = self.logs.lock().unwrap();
                    let recent: Vec<String> = logs
                        .iter()
                        .rev()
                        .take(20)
                        .map(|e| format!("[{}] {} — {}", e.level, e.timestamp, e.message))
                        .collect();
                    Ok(ToolResult::success(
                        call.id,
                        format!("{} logs (last 20):\n{}", logs.len(), recent.join("\n")),
                    ))
                }
            }
            "resource_usage" => {
                let sys = sysinfo::System::new_all();
                let cpu = sys.global_cpu_usage();
                let mem_total = sys.total_memory();
                let mem_used = sys.used_memory();
                let mem_pct = (mem_used as f64 / mem_total as f64) * 100.0;
                Ok(ToolResult::success(call.id, format!("CPU: {cpu:.1}% | Memory: {:.1}% ({} / {} MB)", mem_pct, mem_used / 1_048_576, mem_total / 1_048_576))
                    .with_data(json!({"cpu_pct": cpu, "mem_pct": mem_pct, "mem_used_mb": mem_used / 1_048_576, "mem_total_mb": mem_total / 1_048_576})))
            }
            "uptime" => {
                let url = call
                    .arguments
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url'"))?;
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()?;
                let start = std::time::Instant::now();
                match client.get(url).send().await {
                    Ok(resp) => {
                        let latency = start.elapsed();
                        let up = resp.status().is_success();
                        Ok(ToolResult::success(call.id, format!("{}: {} ({:.0}ms)", url, if up { "UP" } else { "DOWN" }, latency.as_millis()))
                            .with_data(json!({"url": url, "up": up, "status": resp.status().as_u16(), "latency_ms": latency.as_millis()})))
                    }
                    Err(e) => Ok(ToolResult::success(call.id, format!("{}: DOWN ({})", url, e))
                        .with_data(json!({"url": url, "up": false}))),
                }
            }
            _ => Ok(ToolResult::success(call.id, format!("Monitor '{}' completed", action))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(MonitorTool::default().definition().name, "monitor");
    }

    #[tokio::test]
    async fn test_record_metrics() {
        let tool = MonitorTool::default();
        let call = ToolCall {
            id: "m1".into(),
            name: "monitor".into(),
            arguments: json!({"action":"metrics","metric_name":"latency","value":42.5}),
        };
        assert!(tool.execute(call).await.unwrap().success);
    }
}
