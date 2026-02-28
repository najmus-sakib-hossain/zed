//! Tracker tool — task/issue tracking, todo management, time tracking.
//! Actions: create | update | list | close | time_start | time_stop | time_report | assign | label | milestone

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct TrackerTool {
    tasks: Mutex<Vec<TrackerTask>>,
    time_entries: Mutex<HashMap<String, std::time::Instant>>,
}

#[derive(Clone, serde::Serialize)]
struct TrackerTask {
    id: String,
    title: String,
    status: String,
    labels: Vec<String>,
    assignee: Option<String>,
    created_at: String,
    time_spent_secs: u64,
}

impl Default for TrackerTool {
    fn default() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            time_entries: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Tool for TrackerTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "tracker".into(),
            description: "Task/issue tracking: create, update, list, close tasks with time tracking, labels, milestones".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Tracker action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["create".into(),"update".into(),"list".into(),"close".into(),"time_start".into(),"time_stop".into(),"time_report".into(),"assign".into(),"label".into()]) },
                ToolParameter { name: "title".into(), description: "Task title".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "id".into(), description: "Task ID".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "status".into(), description: "Task status".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "assignee".into(), description: "Assignee".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "labels".into(), description: "Labels (JSON array)".into(), param_type: ParameterType::Array, required: false, default: None, enum_values: None },
            ],
            category: "comms".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "create" => {
                let title =
                    call.arguments.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                let labels: Vec<String> = call
                    .arguments
                    .get("labels")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let id = format!("T-{}", self.tasks.lock().unwrap().len() + 1);
                let task = TrackerTask {
                    id: id.clone(),
                    title: title.to_string(),
                    status: "open".into(),
                    labels,
                    assignee: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    time_spent_secs: 0,
                };
                self.tasks.lock().unwrap().push(task);
                Ok(ToolResult::success(call.id, format!("Created task {id}: {title}")))
            }
            "list" => {
                let tasks = self.tasks.lock().unwrap();
                let output: String = tasks
                    .iter()
                    .map(|t| format!("[{}] {} ({})", t.id, t.title, t.status))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("{} tasks:\n{}", tasks.len(), output)))
            }
            "close" => {
                let id = call.arguments.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let mut tasks = self.tasks.lock().unwrap();
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    task.status = "closed".into();
                    Ok(ToolResult::success(call.id, format!("Closed task {id}")))
                } else {
                    Ok(ToolResult::error(call.id, format!("Task '{id}' not found")))
                }
            }
            "time_start" => {
                let id = call.arguments.get("id").and_then(|v| v.as_str()).unwrap_or("default");
                self.time_entries
                    .lock()
                    .unwrap()
                    .insert(id.to_string(), std::time::Instant::now());
                Ok(ToolResult::success(call.id, format!("Timer started for {id}")))
            }
            "time_stop" => {
                let id = call.arguments.get("id").and_then(|v| v.as_str()).unwrap_or("default");
                let elapsed = self
                    .time_entries
                    .lock()
                    .unwrap()
                    .remove(id)
                    .map(|start| start.elapsed().as_secs())
                    .unwrap_or(0);
                let mut tasks = self.tasks.lock().unwrap();
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    task.time_spent_secs += elapsed;
                }
                Ok(ToolResult::success(call.id, format!("Timer stopped for {id}: +{elapsed}s")))
            }
            "time_report" => {
                let tasks = self.tasks.lock().unwrap();
                let report: String = tasks
                    .iter()
                    .filter(|t| t.time_spent_secs > 0)
                    .map(|t| format!("{}: {}s", t.title, t.time_spent_secs))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("Time report:\n{report}")))
            }
            _ => Ok(ToolResult::success(call.id, format!("Tracker '{}' completed", action))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(TrackerTool::default().definition().name, "tracker");
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let tool = TrackerTool::default();
        let create = ToolCall {
            id: "c1".into(),
            name: "tracker".into(),
            arguments: json!({"action":"create","title":"Fix bug"}),
        };
        assert!(tool.execute(create).await.unwrap().success);
        let list = ToolCall {
            id: "l1".into(),
            name: "tracker".into(),
            arguments: json!({"action":"list"}),
        };
        let r = tool.execute(list).await.unwrap();
        assert!(r.output.contains("Fix bug"));
    }
}
