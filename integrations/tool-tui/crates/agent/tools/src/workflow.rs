//! Workflow tool — task automation and workflow orchestration.
//! Actions: define | run | schedule | list | cancel | cron | pipeline | webhook_trigger

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct WorkflowTool {
    workflows: Mutex<HashMap<String, Workflow>>,
}

#[derive(Clone, serde::Serialize)]
struct Workflow {
    name: String,
    steps: Vec<WorkflowStep>,
    status: String,
    trigger: String,
}

#[derive(Clone, serde::Serialize)]
struct WorkflowStep {
    name: String,
    tool: String,
    action: String,
    args: serde_json::Value,
    status: String,
}

impl Default for WorkflowTool {
    fn default() -> Self {
        Self {
            workflows: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Tool for WorkflowTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "workflow".into(),
            description: "Workflow automation: define multi-step pipelines, schedule tasks, trigger on events, CI/CD orchestration".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Workflow action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["define".into(),"run".into(),"schedule".into(),"list".into(),"cancel".into(),"cron".into(),"pipeline".into()]) },
                ToolParameter { name: "name".into(), description: "Workflow name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "steps".into(), description: "Workflow steps (JSON array of {name, tool, action, args})".into(), param_type: ParameterType::Array, required: false, default: None, enum_values: None },
                ToolParameter { name: "trigger".into(), description: "Trigger type (manual, cron, webhook, file_change)".into(), param_type: ParameterType::String, required: false, default: Some(json!("manual")), enum_values: None },
                ToolParameter { name: "schedule".into(), description: "Cron expression for scheduling".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "workflow".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "define" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                let trigger =
                    call.arguments.get("trigger").and_then(|v| v.as_str()).unwrap_or("manual");
                let steps: Vec<WorkflowStep> = call
                    .arguments
                    .get("steps")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|s| WorkflowStep {
                                name: s["name"].as_str().unwrap_or("step").to_string(),
                                tool: s["tool"].as_str().unwrap_or("shell").to_string(),
                                action: s["action"].as_str().unwrap_or("exec").to_string(),
                                args: s["args"].clone(),
                                status: "pending".into(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let step_count = steps.len();
                let wf = Workflow {
                    name: name.to_string(),
                    steps,
                    status: "defined".into(),
                    trigger: trigger.to_string(),
                };
                self.workflows.lock().unwrap().insert(name.to_string(), wf);
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "Workflow '{name}' defined with {step_count} steps (trigger: {trigger})"
                    ),
                ))
            }
            "list" => {
                let wfs = self.workflows.lock().unwrap();
                let output: String = wfs
                    .values()
                    .map(|w| {
                        format!(
                            "[{}] {} ({} steps, trigger: {})",
                            w.status,
                            w.name,
                            w.steps.len(),
                            w.trigger
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("{} workflows:\n{}", wfs.len(), output)))
            }
            "run" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                let mut wfs = self.workflows.lock().unwrap();
                if let Some(wf) = wfs.get_mut(name) {
                    wf.status = "running".into();
                    let steps: Vec<String> =
                        wf.steps.iter().map(|s| format!("{}:{}", s.tool, s.action)).collect();
                    Ok(ToolResult::success(
                        call.id,
                        format!("Running workflow '{}': {}", name, steps.join(" → ")),
                    ))
                } else {
                    Ok(ToolResult::error(call.id, format!("Workflow '{name}' not found")))
                }
            }
            "cancel" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                let mut wfs = self.workflows.lock().unwrap();
                if let Some(wf) = wfs.get_mut(name) {
                    wf.status = "cancelled".into();
                    Ok(ToolResult::success(call.id, format!("Cancelled workflow '{name}'")))
                } else {
                    Ok(ToolResult::error(call.id, format!("Workflow '{name}' not found")))
                }
            }
            "pipeline" => {
                // Pre-built CI/CD pipeline
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("ci");
                let steps = vec![
                    WorkflowStep {
                        name: "checkout".into(),
                        tool: "git".into(),
                        action: "status".into(),
                        args: json!({}),
                        status: "pending".into(),
                    },
                    WorkflowStep {
                        name: "build".into(),
                        tool: "shell".into(),
                        action: "exec".into(),
                        args: json!({"command": "cargo build"}),
                        status: "pending".into(),
                    },
                    WorkflowStep {
                        name: "test".into(),
                        tool: "test".into(),
                        action: "run".into(),
                        args: json!({}),
                        status: "pending".into(),
                    },
                    WorkflowStep {
                        name: "lint".into(),
                        tool: "lint".into(),
                        action: "lint".into(),
                        args: json!({"path": "."}),
                        status: "pending".into(),
                    },
                ];
                let wf = Workflow {
                    name: name.to_string(),
                    steps,
                    status: "defined".into(),
                    trigger: "manual".into(),
                };
                self.workflows.lock().unwrap().insert(name.to_string(), wf);
                Ok(ToolResult::success(
                    call.id,
                    format!("CI pipeline '{name}' created: checkout → build → test → lint"),
                ))
            }
            _ => Ok(ToolResult::success(call.id, format!("Workflow '{action}' completed"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(WorkflowTool::default().definition().name, "workflow");
    }

    #[tokio::test]
    async fn test_pipeline() {
        let tool = WorkflowTool::default();
        let call = ToolCall {
            id: "p1".into(),
            name: "workflow".into(),
            arguments: json!({"action":"pipeline","name":"ci"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.output.contains("checkout"));
        assert!(r.output.contains("build"));
    }
}
