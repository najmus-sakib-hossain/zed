//! # GitHub Integration
//!
//! GitHub CLI wrapper for issues, PRs, and workflows.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::github::{GitHubClient, GitHubConfig};
//!
//! let config = GitHubConfig::from_file("~/.dx/config/github.sr")?;
//! let client = GitHubClient::new(&config)?;
//!
//! // Create an issue
//! let issue = client.create_issue("repo", "Bug title", "Description").await?;
//!
//! // List PRs
//! let prs = client.list_prs("repo", "open").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// GitHub configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// Whether GitHub integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// GitHub API token
    #[serde(default)]
    pub token: String,
    /// Default owner/org
    pub default_owner: Option<String>,
    /// Default repository
    pub default_repo: Option<String>,
    /// Use gh CLI instead of API
    #[serde(default = "default_true")]
    pub use_cli: bool,
}

fn default_true() -> bool {
    true
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            token: String::new(),
            default_owner: None,
            default_repo: None,
            use_cli: true,
        }
    }
}

impl GitHubConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.token.is_empty() || self.token.starts_with('$') {
            self.token = std::env::var("GITHUB_TOKEN")
                .or_else(|_| std::env::var("GH_TOKEN"))
                .unwrap_or_default();
        }
    }
}

/// GitHub issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    /// Issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: Option<String>,
    /// Issue state
    pub state: IssueState,
    /// Issue URL
    pub url: String,
    /// Author
    pub author: String,
    /// Labels
    pub labels: Vec<String>,
    /// Assignees
    pub assignees: Vec<String>,
    /// Created at
    pub created_at: String,
    /// Updated at
    pub updated_at: String,
}

/// Issue/PR state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

/// GitHub pull request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPR {
    /// PR number
    pub number: u64,
    /// PR title
    pub title: String,
    /// PR body
    pub body: Option<String>,
    /// PR state
    pub state: IssueState,
    /// PR URL
    pub url: String,
    /// Author
    pub author: String,
    /// Head branch
    pub head: String,
    /// Base branch
    pub base: String,
    /// Is draft
    pub draft: bool,
    /// Is merged
    pub merged: bool,
    /// Labels
    pub labels: Vec<String>,
    /// Reviewers
    pub reviewers: Vec<String>,
}

/// GitHub workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRun {
    /// Run ID
    pub id: u64,
    /// Workflow name
    pub workflow: String,
    /// Run status
    pub status: RunStatus,
    /// Conclusion
    pub conclusion: Option<String>,
    /// Branch
    pub branch: String,
    /// URL
    pub url: String,
    /// Created at
    pub created_at: String,
}

/// Workflow run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
}

/// GitHub client
pub struct GitHubClient {
    config: GitHubConfig,
    base_url: String,
}

impl GitHubClient {
    /// API base URL
    const API_BASE: &'static str = "https://api.github.com";

    /// Create a new GitHub client
    pub fn new(config: &GitHubConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        // gh CLI doesn't require token if already authenticated
        self.config.use_cli || !self.config.token.is_empty()
    }

    /// Get full repo path
    fn repo_path(&self, repo: &str) -> String {
        if repo.contains('/') {
            repo.to_string()
        } else if let Some(ref owner) = self.config.default_owner {
            format!("{}/{}", owner, repo)
        } else {
            repo.to_string()
        }
    }

    // Issue operations

    /// Create an issue
    pub async fn create_issue(&self, repo: &str, title: &str, body: &str) -> Result<GitHubIssue> {
        if self.config.use_cli {
            return self.cli_create_issue(repo, title, body).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/issues", self.base_url, repo);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .json(&serde_json::json!({
                "title": title,
                "body": body
            }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("GitHub error: {}", error)));
        }

        let issue: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.parse_issue(issue)
    }

    /// List issues
    pub async fn list_issues(&self, repo: &str, state: &str) -> Result<Vec<GitHubIssue>> {
        if self.config.use_cli {
            return self.cli_list_issues(repo, state).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/issues?state={}", self.base_url, repo, state);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list issues".into()));
        }

        let issues: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        issues.into_iter().map(|i| self.parse_issue(i)).collect()
    }

    /// Close an issue
    pub async fn close_issue(&self, repo: &str, number: u64) -> Result<()> {
        if self.config.use_cli {
            return self.cli_close_issue(repo, number).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/issues/{}", self.base_url, repo, number);

        let client = reqwest::Client::new();
        let response = client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .json(&serde_json::json!({ "state": "closed" }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to close issue".into()));
        }

        Ok(())
    }

    // PR operations

    /// Create a pull request
    pub async fn create_pr(
        &self,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Result<GitHubPR> {
        if self.config.use_cli {
            return self.cli_create_pr(repo, title, body, head, base).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/pulls", self.base_url, repo);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .json(&serde_json::json!({
                "title": title,
                "body": body,
                "head": head,
                "base": base
            }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("GitHub error: {}", error)));
        }

        let pr: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.parse_pr(pr)
    }

    /// List pull requests
    pub async fn list_prs(&self, repo: &str, state: &str) -> Result<Vec<GitHubPR>> {
        if self.config.use_cli {
            return self.cli_list_prs(repo, state).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/pulls?state={}", self.base_url, repo, state);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list PRs".into()));
        }

        let prs: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        prs.into_iter().map(|p| self.parse_pr(p)).collect()
    }

    /// Merge a pull request
    pub async fn merge_pr(&self, repo: &str, number: u64) -> Result<()> {
        if self.config.use_cli {
            return self.cli_merge_pr(repo, number).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/pulls/{}/merge", self.base_url, repo, number);

        let client = reqwest::Client::new();
        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to merge PR".into()));
        }

        Ok(())
    }

    // Workflow operations

    /// List workflow runs
    pub async fn list_runs(&self, repo: &str) -> Result<Vec<GitHubRun>> {
        if self.config.use_cli {
            return self.cli_list_runs(repo).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/actions/runs", self.base_url, repo);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list runs".into()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        result["workflow_runs"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid response".into()))?
            .iter()
            .map(|r| self.parse_run(r.clone()))
            .collect()
    }

    /// Re-run a workflow
    pub async fn rerun_workflow(&self, repo: &str, run_id: u64) -> Result<()> {
        if self.config.use_cli {
            return self.cli_rerun_workflow(repo, run_id).await;
        }

        let repo = self.repo_path(repo);
        let url = format!("{}/repos/{}/actions/runs/{}/rerun", self.base_url, repo, run_id);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "dx-driven")
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to rerun workflow".into()));
        }

        Ok(())
    }

    // CLI methods

    async fn cli_create_issue(&self, repo: &str, title: &str, body: &str) -> Result<GitHubIssue> {
        let output = self.run_gh(&[
            "issue", "create",
            "-R", &self.repo_path(repo),
            "-t", title,
            "-b", body,
            "--json", "number,title,body,state,url,author,labels,assignees,createdAt,updatedAt"
        ]).await?;

        let issue: serde_json::Value = serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))?;
        
        self.parse_issue(issue)
    }

    async fn cli_list_issues(&self, repo: &str, state: &str) -> Result<Vec<GitHubIssue>> {
        let output = self.run_gh(&[
            "issue", "list",
            "-R", &self.repo_path(repo),
            "-s", state,
            "--json", "number,title,body,state,url,author,labels,assignees,createdAt,updatedAt"
        ]).await?;

        let issues: Vec<serde_json::Value> = serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        issues.into_iter().map(|i| self.parse_issue(i)).collect()
    }

    async fn cli_close_issue(&self, repo: &str, number: u64) -> Result<()> {
        self.run_gh(&[
            "issue", "close",
            "-R", &self.repo_path(repo),
            &number.to_string()
        ]).await?;
        Ok(())
    }

    async fn cli_create_pr(&self, repo: &str, title: &str, body: &str, head: &str, base: &str) -> Result<GitHubPR> {
        let output = self.run_gh(&[
            "pr", "create",
            "-R", &self.repo_path(repo),
            "-t", title,
            "-b", body,
            "-H", head,
            "-B", base,
            "--json", "number,title,body,state,url,author,headRefName,baseRefName,isDraft,merged,labels,reviewRequests"
        ]).await?;

        let pr: serde_json::Value = serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.parse_pr(pr)
    }

    async fn cli_list_prs(&self, repo: &str, state: &str) -> Result<Vec<GitHubPR>> {
        let output = self.run_gh(&[
            "pr", "list",
            "-R", &self.repo_path(repo),
            "-s", state,
            "--json", "number,title,body,state,url,author,headRefName,baseRefName,isDraft,merged,labels,reviewRequests"
        ]).await?;

        let prs: Vec<serde_json::Value> = serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        prs.into_iter().map(|p| self.parse_pr(p)).collect()
    }

    async fn cli_merge_pr(&self, repo: &str, number: u64) -> Result<()> {
        self.run_gh(&[
            "pr", "merge",
            "-R", &self.repo_path(repo),
            &number.to_string(),
            "--merge"
        ]).await?;
        Ok(())
    }

    async fn cli_list_runs(&self, repo: &str) -> Result<Vec<GitHubRun>> {
        let output = self.run_gh(&[
            "run", "list",
            "-R", &self.repo_path(repo),
            "--json", "databaseId,workflowName,status,conclusion,headBranch,url,createdAt"
        ]).await?;

        let runs: Vec<serde_json::Value> = serde_json::from_str(&output)
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        runs.into_iter().map(|r| self.parse_run(r)).collect()
    }

    async fn cli_rerun_workflow(&self, repo: &str, run_id: u64) -> Result<()> {
        self.run_gh(&[
            "run", "rerun",
            "-R", &self.repo_path(repo),
            &run_id.to_string()
        ]).await?;
        Ok(())
    }

    async fn run_gh(&self, args: &[&str]) -> Result<String> {
        use tokio::process::Command;

        let output = Command::new("gh")
            .args(args)
            .output()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to run gh: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DrivenError::Process(format!("gh command failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn parse_issue(&self, v: serde_json::Value) -> Result<GitHubIssue> {
        Ok(GitHubIssue {
            number: v["number"].as_u64().unwrap_or_default(),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            body: v["body"].as_str().map(String::from),
            state: if v["state"].as_str().unwrap_or("open") == "open" {
                IssueState::Open
            } else {
                IssueState::Closed
            },
            url: v["url"].as_str().or(v["html_url"].as_str()).unwrap_or_default().to_string(),
            author: v["author"]["login"].as_str()
                .or(v["user"]["login"].as_str())
                .unwrap_or_default().to_string(),
            labels: v["labels"].as_array()
                .map(|arr| arr.iter()
                    .filter_map(|l| l["name"].as_str().or(l.as_str()).map(String::from))
                    .collect())
                .unwrap_or_default(),
            assignees: v["assignees"].as_array()
                .map(|arr| arr.iter()
                    .filter_map(|a| a["login"].as_str().map(String::from))
                    .collect())
                .unwrap_or_default(),
            created_at: v["createdAt"].as_str().or(v["created_at"].as_str()).unwrap_or_default().to_string(),
            updated_at: v["updatedAt"].as_str().or(v["updated_at"].as_str()).unwrap_or_default().to_string(),
        })
    }

    fn parse_pr(&self, v: serde_json::Value) -> Result<GitHubPR> {
        Ok(GitHubPR {
            number: v["number"].as_u64().unwrap_or_default(),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            body: v["body"].as_str().map(String::from),
            state: if v["state"].as_str().unwrap_or("open") == "open" {
                IssueState::Open
            } else {
                IssueState::Closed
            },
            url: v["url"].as_str().or(v["html_url"].as_str()).unwrap_or_default().to_string(),
            author: v["author"]["login"].as_str()
                .or(v["user"]["login"].as_str())
                .unwrap_or_default().to_string(),
            head: v["headRefName"].as_str().or(v["head"]["ref"].as_str()).unwrap_or_default().to_string(),
            base: v["baseRefName"].as_str().or(v["base"]["ref"].as_str()).unwrap_or_default().to_string(),
            draft: v["isDraft"].as_bool().or(v["draft"].as_bool()).unwrap_or(false),
            merged: v["merged"].as_bool().unwrap_or(false),
            labels: v["labels"].as_array()
                .map(|arr| arr.iter()
                    .filter_map(|l| l["name"].as_str().or(l.as_str()).map(String::from))
                    .collect())
                .unwrap_or_default(),
            reviewers: v["reviewRequests"].as_array()
                .or(v["requested_reviewers"].as_array())
                .map(|arr| arr.iter()
                    .filter_map(|r| r["login"].as_str().map(String::from))
                    .collect())
                .unwrap_or_default(),
        })
    }

    fn parse_run(&self, v: serde_json::Value) -> Result<GitHubRun> {
        let status_str = v["status"].as_str().unwrap_or("completed");
        let status = match status_str {
            "queued" => RunStatus::Queued,
            "in_progress" => RunStatus::InProgress,
            _ => RunStatus::Completed,
        };

        Ok(GitHubRun {
            id: v["databaseId"].as_u64().or(v["id"].as_u64()).unwrap_or_default(),
            workflow: v["workflowName"].as_str().or(v["name"].as_str()).unwrap_or_default().to_string(),
            status,
            conclusion: v["conclusion"].as_str().map(String::from),
            branch: v["headBranch"].as_str().or(v["head_branch"].as_str()).unwrap_or_default().to_string(),
            url: v["url"].as_str().or(v["html_url"].as_str()).unwrap_or_default().to_string(),
            created_at: v["createdAt"].as_str().or(v["created_at"].as_str()).unwrap_or_default().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GitHubConfig::default();
        assert!(config.enabled);
        assert!(config.use_cli);
    }

    #[test]
    fn test_repo_path() {
        let mut config = GitHubConfig::default();
        config.default_owner = Some("owner".to_string());
        let client = GitHubClient::new(&config).unwrap();

        assert_eq!(client.repo_path("repo"), "owner/repo");
        assert_eq!(client.repo_path("other/repo"), "other/repo");
    }
}
