//! Agent tool — agent orchestration, self-management, and introspection.
//! Actions: status | configure | capabilities | history | delegate | pause | resume | abort

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Mutex;

pub struct AgentTool {
    state: Mutex<AgentState>,
}

struct AgentState {
    name: String,
    status: String,
    tasks_completed: u32,
    tasks_failed: u32,
    history: Vec<String>,
}

impl Default for AgentTool {
    fn default() -> Self {
        Self {
            state: Mutex::new(AgentState {
                name: "dx-agent".into(),
                status: "running".into(),
                tasks_completed: 0,
                tasks_failed: 0,
                history: Vec::new(),
            }),
        }
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "agent".into(),
            description: "Agent self-management: status, capabilities, task history, delegation, pause/resume/abort".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Agent action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["status".into(),"configure".into(),"capabilities".into(),"history".into(),"delegate".into(),"pause".into(),"resume".into(),"abort".into()]) },
                ToolParameter { name: "setting".into(), description: "Config setting name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "value".into(), description: "Config setting value".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "task".into(), description: "Task description for delegation".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "ai".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let mut state = self.state.lock().unwrap();

        match action {
            "status" => {
                Ok(ToolResult::success(call.id, format!("Agent '{}': {} | completed: {} | failed: {} | history: {} entries",
                    state.name, state.status, state.tasks_completed, state.tasks_failed, state.history.len()))
                    .with_data(json!({"name": state.name, "status": state.status, "completed": state.tasks_completed, "failed": state.tasks_failed})))
            }
            "capabilities" => {
                Ok(ToolResult::success(call.id, "50 tools across 10 categories: I/O, VCS, Code Intel, Execution, Data, AI/Memory, Infra, Project, Comms, Monitoring".into())
                    .with_data(json!({"tool_count": 50, "categories": ["io","vcs","code_intel","execution","data","ai","infra","project","comms","monitoring"]})))
            }
            "history" => {
                let recent: Vec<&String> = state.history.iter().rev().take(20).collect();
                Ok(ToolResult::success(call.id, format!("{} history entries (showing last 20):\n{}", state.history.len(), recent.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"))))
            }
            "configure" => {
                let setting = call.arguments.get("setting").and_then(|v| v.as_str()).unwrap_or("");
                let value = call.arguments.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if setting == "name" { state.name = value.to_string(); }
                state.history.push(format!("configured {setting}={value}"));
                Ok(ToolResult::success(call.id, format!("Set {setting}={value}")))
            }
            "pause" => { state.status = "paused".into(); Ok(ToolResult::success(call.id, "Agent paused".into())) }
            "resume" => { state.status = "running".into(); Ok(ToolResult::success(call.id, "Agent resumed".into())) }
            "abort" => { state.status = "aborted".into(); Ok(ToolResult::success(call.id, "Agent aborted".into())) }
            "delegate" => {
                let task = call.arguments.get("task").and_then(|v| v.as_str()).unwrap_or("(no task)");
                state.history.push(format!("delegated: {task}"));
                Ok(ToolResult::success(call.id, format!("Task delegated: {task} — spawn sub-agent for execution")))
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
        assert_eq!(AgentTool::default().definition().name, "agent");
    }

    #[tokio::test]
    async fn test_agent_lifecycle() {
        let tool = AgentTool::default();
        let status = ToolCall {
            id: "s1".into(),
            name: "agent".into(),
            arguments: json!({"action":"status"}),
        };
        let r = tool.execute(status).await.unwrap();
        assert!(r.output.contains("running"));
        let pause = ToolCall {
            id: "p1".into(),
            name: "agent".into(),
            arguments: json!({"action":"pause"}),
        };
        assert!(tool.execute(pause).await.unwrap().output.contains("paused"));
    }
}
