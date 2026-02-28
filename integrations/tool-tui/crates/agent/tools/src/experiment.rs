//! Experiment tool — A/B testing, feature rollout, hypothesis tracking.
//! Actions: create | run | compare | rollback | list

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ExperimentTool {
    experiments: Mutex<HashMap<String, Experiment>>,
}

#[derive(Clone, serde::Serialize)]
struct Experiment {
    name: String,
    status: String,
    variants: Vec<String>,
    results: HashMap<String, f64>,
}

impl Default for ExperimentTool {
    fn default() -> Self {
        Self {
            experiments: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Tool for ExperimentTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "experiment".into(),
            description:
                "A/B testing and experiment tracking: create, run, compare variants, rollback"
                    .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Experiment action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "create".into(),
                        "run".into(),
                        "compare".into(),
                        "rollback".into(),
                        "list".into(),
                    ]),
                },
                ToolParameter {
                    name: "name".into(),
                    description: "Experiment name".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "variants".into(),
                    description: "Variant names (JSON array)".into(),
                    param_type: ParameterType::Array,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "variant".into(),
                    description: "Specific variant".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "metric".into(),
                    description: "Metric value for run".into(),
                    param_type: ParameterType::Number,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "execution".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        match action {
            "create" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                let variants: Vec<String> = call
                    .arguments
                    .get("variants")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_else(|| vec!["control".into(), "variant_a".into()]);
                let exp = Experiment {
                    name: name.to_string(),
                    status: "active".into(),
                    variants: variants.clone(),
                    results: HashMap::new(),
                };
                self.experiments.lock().unwrap().insert(name.to_string(), exp);
                Ok(ToolResult::success(
                    call.id,
                    format!("Experiment '{name}' created with variants: {}", variants.join(", ")),
                ))
            }
            "run" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                let variant =
                    call.arguments.get("variant").and_then(|v| v.as_str()).unwrap_or("control");
                let metric = call.arguments.get("metric").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let mut exps = self.experiments.lock().unwrap();
                if let Some(exp) = exps.get_mut(name) {
                    exp.results.insert(variant.to_string(), metric);
                    Ok(ToolResult::success(
                        call.id,
                        format!("Recorded {variant}={metric} for '{name}'"),
                    ))
                } else {
                    Ok(ToolResult::error(call.id, format!("Experiment '{name}' not found")))
                }
            }
            "compare" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                let exps = self.experiments.lock().unwrap();
                if let Some(exp) = exps.get(name) {
                    let summary: String = exp
                        .results
                        .iter()
                        .map(|(k, v)| format!("{k}: {v:.4}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    Ok(ToolResult::success(call.id, format!("Experiment '{}': {}", name, summary))
                        .with_data(json!(exp)))
                } else {
                    Ok(ToolResult::error(call.id, format!("Experiment '{name}' not found")))
                }
            }
            "list" => {
                let exps = self.experiments.lock().unwrap();
                let names: Vec<&String> = exps.keys().collect();
                Ok(ToolResult::success(
                    call.id,
                    format!("{} experiments: {:?}", names.len(), names),
                ))
            }
            "rollback" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("default");
                self.experiments.lock().unwrap().remove(name);
                Ok(ToolResult::success(call.id, format!("Experiment '{name}' rolled back")))
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
        assert_eq!(ExperimentTool::default().definition().name, "experiment");
    }

    #[tokio::test]
    async fn test_experiment_lifecycle() {
        let tool = ExperimentTool::default();
        // Create
        let call = ToolCall {
            id: "1".into(),
            name: "experiment".into(),
            arguments: json!({"action":"create","name":"test_exp","variants":["a","b"]}),
        };
        assert!(tool.execute(call).await.unwrap().success);
        // Run
        let call = ToolCall {
            id: "2".into(),
            name: "experiment".into(),
            arguments: json!({"action":"run","name":"test_exp","variant":"a","metric":0.95}),
        };
        assert!(tool.execute(call).await.unwrap().success);
        // Compare
        let call = ToolCall {
            id: "3".into(),
            name: "experiment".into(),
            arguments: json!({"action":"compare","name":"test_exp"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.output.contains("0.95"));
    }
}
