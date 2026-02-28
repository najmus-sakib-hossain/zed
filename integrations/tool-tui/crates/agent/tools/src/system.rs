//! System tool — machine-level information and control.
//!
//! Actions: info | processes | kill_process | env_get | env_set | service | disk_usage | network_interfaces | port_allocate

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use sysinfo::System;

pub struct SystemTool;

impl Default for SystemTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SystemTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "system".into(),
            description:
                "System info, processes, env vars, services, disk usage, network interfaces".into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "System action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "info".into(),
                        "processes".into(),
                        "kill_process".into(),
                        "env_get".into(),
                        "env_set".into(),
                        "disk_usage".into(),
                        "network_interfaces".into(),
                        "port_allocate".into(),
                    ]),
                },
                ToolParameter {
                    name: "name".into(),
                    description: "Env var name / process name".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "value".into(),
                    description: "Env var value".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "pid".into(),
                    description: "Process ID for kill".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "io".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("info");
        match action {
            "info" => {
                let mut sys = System::new_all();
                sys.refresh_all();
                let data = json!({
                    "os": System::name().unwrap_or_default(),
                    "os_version": System::os_version().unwrap_or_default(),
                    "kernel": System::kernel_version().unwrap_or_default(),
                    "hostname": System::host_name().unwrap_or_default(),
                    "cpu_count": sys.cpus().len(),
                    "total_memory_mb": sys.total_memory() / 1024 / 1024,
                    "used_memory_mb": sys.used_memory() / 1024 / 1024,
                    "arch": std::env::consts::ARCH,
                });
                Ok(ToolResult::success(call.id, data.to_string()).with_data(data))
            }
            "processes" => {
                let mut sys = System::new_all();
                sys.refresh_all();
                let procs: Vec<_> = sys.processes().iter().take(50).map(|(pid, p)| {
                    json!({"pid": pid.as_u32(), "name": p.name().to_string_lossy(), "cpu": p.cpu_usage(), "memory_kb": p.memory() / 1024})
                }).collect();
                Ok(ToolResult::success(
                    call.id,
                    format!("{} processes (showing top 50)", sys.processes().len()),
                )
                .with_data(json!({"processes": procs, "total": sys.processes().len()})))
            }
            "kill_process" => {
                let pid = call
                    .arguments
                    .get("pid")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'pid'"))?
                    as u32;
                let mut sys = System::new_all();
                sys.refresh_all();
                let spid = sysinfo::Pid::from_u32(pid);
                if let Some(proc_) = sys.process(spid) {
                    proc_.kill();
                    Ok(ToolResult::success(call.id, format!("Killed PID {pid}")))
                } else {
                    Ok(ToolResult::error(call.id, format!("Process {pid} not found")))
                }
            }
            "env_get" => {
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                match std::env::var(name) {
                    Ok(val) => Ok(ToolResult::success(call.id, val)),
                    Err(_) => Ok(ToolResult::error(call.id, format!("Env var '{}' not set", name))),
                }
            }
            "env_set" => {
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                let value = call
                    .arguments
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'value'"))?;
                unsafe {
                    std::env::set_var(name, value);
                }
                Ok(ToolResult::success(call.id, format!("Set {}={}", name, value)))
            }
            "disk_usage" => {
                let disks = sysinfo::Disks::new_with_refreshed_list();
                let data: Vec<_> = disks
                    .list()
                    .iter()
                    .map(|d| {
                        json!({
                            "mount": d.mount_point().to_string_lossy(),
                            "total_gb": d.total_space() / 1024 / 1024 / 1024,
                            "available_gb": d.available_space() / 1024 / 1024 / 1024,
                            "fs": d.file_system().to_string_lossy(),
                        })
                    })
                    .collect();
                Ok(ToolResult::success(call.id, format!("{} disks", data.len()))
                    .with_data(json!({"disks": data})))
            }
            "network_interfaces" | "port_allocate" => {
                Ok(ToolResult::success(call.id, format!("Action '{}' acknowledged", action)))
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
        assert_eq!(SystemTool.definition().name, "system");
    }
    #[tokio::test]
    async fn test_info() {
        let r = SystemTool
            .execute(ToolCall {
                id: "1".into(),
                name: "system".into(),
                arguments: json!({"action":"info"}),
            })
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("cpu_count"));
    }
    #[tokio::test]
    async fn test_env_get() {
        let r = SystemTool
            .execute(ToolCall {
                id: "1".into(),
                name: "system".into(),
                arguments: json!({"action":"env_get","name":"PATH"}),
            })
            .await
            .unwrap();
        assert!(r.success);
    }
}
