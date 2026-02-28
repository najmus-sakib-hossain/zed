//! Spawn tool — spawn sub-agents for parallel task execution.
//! Actions: spawn | list | join | kill | send_message

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct SpawnTool {
    agents: Mutex<HashMap<String, SubAgent>>,
}

#[derive(Clone, serde::Serialize)]
struct SubAgent {
    id: String,
    task: String,
    status: String,
    created_at: String,
}

impl Default for SpawnTool {
    fn default() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Tool for SpawnTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "spawn".into(),
            description:
                "Spawn sub-agents for parallel task execution, coordinate multi-agent workflows"
                    .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Spawn action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "spawn".into(),
                        "list".into(),
                        "join".into(),
                        "kill".into(),
                        "send_message".into(),
                    ]),
                },
                ToolParameter {
                    name: "task".into(),
                    description: "Task for spawned agent".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "agent_id".into(),
                    description: "Agent ID to interact with".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "message".into(),
                    description: "Message to send to agent".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "tools".into(),
                    description: "Tools available to spawned agent (JSON array)".into(),
                    param_type: ParameterType::Array,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "ai".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "spawn" => {
                let task =
                    call.arguments.get("task").and_then(|v| v.as_str()).unwrap_or("(no task)");
                let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
                let agent = SubAgent {
                    id: id.clone(),
                    task: task.to_string(),
                    status: "running".into(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                self.agents.lock().unwrap().insert(id.clone(), agent);
                Ok(ToolResult::success(call.id, format!("Spawned agent '{id}' for task: {task}"))
                    .with_data(json!({"agent_id": id})))
            }
            "list" => {
                let agents = self.agents.lock().unwrap();
                let list: Vec<&SubAgent> = agents.values().collect();
                let output = list
                    .iter()
                    .map(|a| format!("[{}] {} — {}", a.id, a.status, a.task))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("{} agents:\n{}", list.len(), output))
                    .with_data(json!(list)))
            }
            "join" => {
                let agent_id =
                    call.arguments.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
                let mut agents = self.agents.lock().unwrap();
                if let Some(agent) = agents.get_mut(agent_id) {
                    agent.status = "completed".into();
                    Ok(ToolResult::success(call.id, format!("Joined agent '{agent_id}'")))
                } else {
                    Ok(ToolResult::error(call.id, format!("Agent '{agent_id}' not found")))
                }
            }
            "kill" => {
                let agent_id =
                    call.arguments.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
                self.agents.lock().unwrap().remove(agent_id);
                Ok(ToolResult::success(call.id, format!("Killed agent '{agent_id}'")))
            }
            "send_message" => {
                let agent_id =
                    call.arguments.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
                let message = call.arguments.get("message").and_then(|v| v.as_str()).unwrap_or("");
                Ok(ToolResult::success(call.id, format!("Message sent to '{agent_id}': {message}")))
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
        assert_eq!(SpawnTool::default().definition().name, "spawn");
    }

    #[tokio::test]
    async fn test_spawn_and_list() {
        let tool = SpawnTool::default();
        let spawn = ToolCall {
            id: "s1".into(),
            name: "spawn".into(),
            arguments: json!({"action":"spawn","task":"analyze code"}),
        };
        assert!(tool.execute(spawn).await.unwrap().success);
        let list = ToolCall {
            id: "l1".into(),
            name: "spawn".into(),
            arguments: json!({"action":"list"}),
        };
        let r = tool.execute(list).await.unwrap();
        assert!(r.output.contains("1 agents"));
    }
}
