//! Infra tool — cloud infrastructure and IaC management.
//! Actions: terraform_plan | terraform_apply | ansible_run | cloud_status | dns_manage | ssl_cert | cdn_purge | env_provision

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct InfraTool;
impl Default for InfraTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for InfraTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "infra".into(),
            description: "Cloud infrastructure: Terraform plan/apply, Ansible, cloud status, DNS, SSL, CDN, environment provisioning".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Infra action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["terraform_plan".into(),"terraform_apply".into(),"ansible_run".into(),"cloud_status".into(),"dns_manage".into(),"ssl_cert".into(),"cdn_purge".into(),"env_provision".into()]) },
                ToolParameter { name: "directory".into(), description: "Working directory for IaC".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "playbook".into(), description: "Ansible playbook path".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "provider".into(), description: "Cloud provider (aws, gcp, azure)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "infra".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action =
            call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("cloud_status");
        let dir = call.arguments.get("directory").and_then(|v| v.as_str()).unwrap_or(".");
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let cmd = match action {
            "terraform_plan" => "terraform plan -no-color".to_string(),
            "terraform_apply" => "terraform apply -auto-approve -no-color".to_string(),
            "ansible_run" => {
                let playbook = call
                    .arguments
                    .get("playbook")
                    .and_then(|v| v.as_str())
                    .unwrap_or("playbook.yml");
                format!("ansible-playbook {playbook}")
            }
            "cloud_status" => {
                let provider =
                    call.arguments.get("provider").and_then(|v| v.as_str()).unwrap_or("aws");
                match provider {
                    "aws" => "aws sts get-caller-identity".to_string(),
                    "gcp" => "gcloud config list".to_string(),
                    "azure" => "az account show".to_string(),
                    _ => format!("echo 'Unknown provider: {provider}'"),
                }
            }
            "ssl_cert" => {
                return Ok(ToolResult::success(
                    call.id,
                    "SSL cert management — use certbot or cloud provider ACM/Let's Encrypt".into(),
                ));
            }
            _ => {
                return Ok(ToolResult::success(
                    call.id,
                    format!("Infra '{}' — install required IaC tools", action),
                ));
            }
        };

        let output = tokio::process::Command::new(shell)
            .arg(flag)
            .arg(&cmd)
            .current_dir(dir)
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            Ok(ToolResult::success(call.id, stdout))
        } else {
            Ok(ToolResult::error(call.id, format!("{stdout}\n{stderr}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(InfraTool.definition().name, "infra");
    }
}
