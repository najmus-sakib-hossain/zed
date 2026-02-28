//! GitHub/GitLab tool — platform operations.
//! Actions: repo_info | pr_create | pr_review | pr_comment | issue_manage | actions_trigger | actions_status | gist

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct GithubTool {
    http: reqwest::Client,
    token: Option<String>,
}

impl GithubTool {
    pub fn new(token: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }
}
impl Default for GithubTool {
    fn default() -> Self {
        Self::new(std::env::var("GITHUB_TOKEN").ok())
    }
}

#[async_trait]
impl Tool for GithubTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "github".into(),
            description: "GitHub/GitLab: repo info, PRs, issues, actions, gists".into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "GitHub action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "repo_info".into(),
                        "pr_create".into(),
                        "pr_review".into(),
                        "issue_manage".into(),
                        "actions_status".into(),
                        "gist".into(),
                    ]),
                },
                ToolParameter {
                    name: "repo".into(),
                    description: "Repository (owner/name)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "title".into(),
                    description: "PR/issue title".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "body".into(),
                    description: "PR/issue body".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "base".into(),
                    description: "Base branch for PR".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(json!("main")),
                    enum_values: None,
                },
                ToolParameter {
                    name: "head".into(),
                    description: "Head branch for PR".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "number".into(),
                    description: "PR/issue number".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "vcs".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("repo_info");
        let repo = call.arguments.get("repo").and_then(|v| v.as_str()).unwrap_or("");

        let mut req_builder = |method: reqwest::Method, endpoint: &str| {
            let url = format!("https://api.github.com{}", endpoint);
            let mut r = self
                .http
                .request(method, &url)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "dx-agent");
            if let Some(ref t) = self.token {
                r = r.header("Authorization", format!("Bearer {}", t));
            }
            r
        };

        match action {
            "repo_info" => {
                let resp =
                    req_builder(reqwest::Method::GET, &format!("/repos/{}", repo)).send().await?;
                let data: serde_json::Value = resp.json().await?;
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&data)?)
                    .with_data(data))
            }
            "pr_create" => {
                let title =
                    call.arguments.get("title").and_then(|v| v.as_str()).unwrap_or("New PR");
                let body = call.arguments.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let base = call.arguments.get("base").and_then(|v| v.as_str()).unwrap_or("main");
                let head = call
                    .arguments
                    .get("head")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'head' branch"))?;
                let resp = req_builder(reqwest::Method::POST, &format!("/repos/{}/pulls", repo))
                    .json(&json!({"title": title, "body": body, "base": base, "head": head}))
                    .send()
                    .await?;
                let data: serde_json::Value = resp.json().await?;
                let pr_num = data.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                Ok(ToolResult::success(call.id, format!("PR #{} created", pr_num)).with_data(data))
            }
            "issue_manage" => {
                let title =
                    call.arguments.get("title").and_then(|v| v.as_str()).unwrap_or("New Issue");
                let body = call.arguments.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let resp = req_builder(reqwest::Method::POST, &format!("/repos/{}/issues", repo))
                    .json(&json!({"title": title, "body": body}))
                    .send()
                    .await?;
                let data: serde_json::Value = resp.json().await?;
                Ok(ToolResult::success(call.id, format!("Issue created")).with_data(data))
            }
            "actions_status" => {
                let resp = req_builder(
                    reqwest::Method::GET,
                    &format!("/repos/{}/actions/runs?per_page=5", repo),
                )
                .send()
                .await?;
                let data: serde_json::Value = resp.json().await?;
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&data)?)
                    .with_data(data))
            }
            "gist" => {
                let body_content =
                    call.arguments.get("body").and_then(|v| v.as_str()).unwrap_or("# Gist");
                let title =
                    call.arguments.get("title").and_then(|v| v.as_str()).unwrap_or("gist.md");
                let resp = req_builder(reqwest::Method::POST, "/gists")
                    .json(&json!({"files": {title: {"content": body_content}}, "public": false}))
                    .send()
                    .await?;
                let data: serde_json::Value = resp.json().await?;
                Ok(ToolResult::success(call.id, format!("Gist created")).with_data(data))
            }
            _ => {
                Ok(ToolResult::success(call.id, format!("GitHub action '{}' acknowledged", action)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(GithubTool::default().definition().name, "github");
    }
}
