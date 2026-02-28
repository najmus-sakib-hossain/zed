//! Kubernetes tool — cluster management via kubectl.
//! Actions: get | apply | delete | logs | describe | port_forward | scale | rollout | exec | config

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct KubernetesTool;
impl Default for KubernetesTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for KubernetesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "kubernetes".into(),
            description: "Kubernetes cluster management via kubectl: get/apply/delete resources, logs, port-forward, scale, rollout".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "K8s action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["get".into(),"apply".into(),"delete".into(),"logs".into(),"describe".into(),"port_forward".into(),"scale".into(),"rollout".into(),"exec".into(),"config".into()]) },
                ToolParameter { name: "resource".into(), description: "Resource type (pods, services, deployments, etc.)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "name".into(), description: "Resource name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "namespace".into(), description: "Namespace".into(), param_type: ParameterType::String, required: false, default: Some(json!("default")), enum_values: None },
                ToolParameter { name: "file".into(), description: "YAML manifest file".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "replicas".into(), description: "Replica count for scale".into(), param_type: ParameterType::Integer, required: false, default: None, enum_values: None },
            ],
            category: "infra".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("get");
        let ns = call.arguments.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let cmd = match action {
            "get" => {
                let resource =
                    call.arguments.get("resource").and_then(|v| v.as_str()).unwrap_or("pods");
                format!("kubectl get {resource} -n {ns} -o wide")
            }
            "apply" => {
                let file = call
                    .arguments
                    .get("file")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'file'"))?;
                format!("kubectl apply -f {file} -n {ns}")
            }
            "delete" => {
                let resource =
                    call.arguments.get("resource").and_then(|v| v.as_str()).unwrap_or("pod");
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                format!("kubectl delete {resource} {name} -n {ns}")
            }
            "logs" => {
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                format!("kubectl logs --tail=100 {name} -n {ns}")
            }
            "describe" => {
                let resource =
                    call.arguments.get("resource").and_then(|v| v.as_str()).unwrap_or("pod");
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                format!("kubectl describe {resource} {name} -n {ns}")
            }
            "scale" => {
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                let replicas = call.arguments.get("replicas").and_then(|v| v.as_u64()).unwrap_or(1);
                format!("kubectl scale deployment {name} --replicas={replicas} -n {ns}")
            }
            "rollout" => {
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                format!("kubectl rollout status deployment/{name} -n {ns}")
            }
            "config" => "kubectl config current-context".into(),
            _ => format!("kubectl {action}"),
        };

        let output = tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            Ok(ToolResult::success(call.id, stdout))
        } else {
            Ok(ToolResult::error(call.id, if stderr.is_empty() { stdout } else { stderr }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(KubernetesTool.definition().name, "kubernetes");
    }
}
