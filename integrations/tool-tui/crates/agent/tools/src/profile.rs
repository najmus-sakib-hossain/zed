//! Profile tool — performance profiling and benchmarking.
//! Actions: benchmark | flamegraph | allocations | time | compare

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct ProfileTool;
impl Default for ProfileTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ProfileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "profile".into(),
            description: "Performance profiling: benchmarks, flamegraphs, allocation tracking, timing, comparison".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Profile action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["benchmark".into(),"flamegraph".into(),"allocations".into(),"time".into(),"compare".into()]) },
                ToolParameter { name: "command".into(), description: "Command to profile".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "iterations".into(), description: "Number of iterations for benchmarks".into(), param_type: ParameterType::Integer, required: false, default: Some(json!(100)), enum_values: None },
            ],
            category: "execution".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("time");
        let command = call.arguments.get("command").and_then(|v| v.as_str());

        match action {
            "time" => {
                if let Some(cmd) = command {
                    let start = std::time::Instant::now();
                    let (shell, flag) = if cfg!(windows) {
                        ("cmd", "/C")
                    } else {
                        ("sh", "-c")
                    };
                    let output =
                        tokio::process::Command::new(shell).arg(flag).arg(cmd).output().await?;
                    let elapsed = start.elapsed();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(ToolResult::success(call.id, format!("Completed in {:.3}s (exit: {})\n{}", elapsed.as_secs_f64(), output.status.code().unwrap_or(-1), stdout))
                        .with_data(json!({"elapsed_ms": elapsed.as_millis(), "exit_code": output.status.code()})))
                } else {
                    Ok(ToolResult::error(call.id, "Need 'command' to time".into()))
                }
            }
            "benchmark" => {
                if let Some(cmd) = command {
                    let iterations =
                        call.arguments.get("iterations").and_then(|v| v.as_u64()).unwrap_or(10);
                    let mut times = Vec::new();
                    let (shell, flag) = if cfg!(windows) {
                        ("cmd", "/C")
                    } else {
                        ("sh", "-c")
                    };
                    for _ in 0..iterations.min(100) {
                        let start = std::time::Instant::now();
                        let _ =
                            tokio::process::Command::new(shell).arg(flag).arg(cmd).output().await;
                        times.push(start.elapsed().as_secs_f64());
                    }
                    let avg = times.iter().sum::<f64>() / times.len() as f64;
                    let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    Ok(ToolResult::success(
                        call.id,
                        format!(
                            "{iterations} iterations: avg={avg:.4}s min={min:.4}s max={max:.4}s"
                        ),
                    )
                    .with_data(
                        json!({"iterations": iterations, "avg_s": avg, "min_s": min, "max_s": max}),
                    ))
                } else {
                    Ok(ToolResult::error(call.id, "Need 'command' to benchmark".into()))
                }
            }
            "flamegraph" => Ok(ToolResult::success(
                call.id,
                "Flamegraph generation — install cargo-flamegraph and run: `cargo flamegraph`"
                    .into(),
            )),
            _ => Ok(ToolResult::success(
                call.id,
                format!("Profile '{action}' — specialized profiling tools required"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(ProfileTool.definition().name, "profile");
    }
}
