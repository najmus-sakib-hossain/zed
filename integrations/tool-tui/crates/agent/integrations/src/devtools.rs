//! # Dev Tool Integrations
//!
//! Connect to GitHub, GitLab, Linear, Jira, and more.

use async_trait::async_trait;
use tracing::info;

use crate::{DevToolIntegration, Integration, IntegrationError, Result};

/// GitHub integration
pub struct GitHubIntegration {
    token: Option<String>,
    username: Option<String>,
}

impl Default for GitHubIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubIntegration {
    pub fn new() -> Self {
        Self {
            token: None,
            username: None,
        }
    }
}

#[async_trait]
impl Integration for GitHubIntegration {
    fn name(&self) -> &str {
        "github"
    }

    fn integration_type(&self) -> &str {
        "devtool"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    async fn authenticate(&mut self, token: &str) -> Result<()> {
        self.token = Some(token.to_string());
        info!("GitHub authenticated");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.token = None;
        Ok(())
    }

    fn capabilities_dx(&self) -> String {
        "capabilities:6[create_pr create_issue list_repos commit push clone]".to_string()
    }
}

#[async_trait]
impl DevToolIntegration for GitHubIntegration {
    async fn create_pr(
        &self,
        repo: &str,
        title: &str,
        _body: &str,
        _branch: &str,
    ) -> Result<String> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("github".to_string()))?;

        info!("Creating PR on {}: {}", repo, title);

        // In production, call GitHub API
        // POST /repos/{owner}/{repo}/pulls

        Ok(format!("https://github.com/{}/pull/1", repo))
    }

    async fn create_issue(&self, repo: &str, title: &str, _body: &str) -> Result<String> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("github".to_string()))?;

        info!("Creating issue on {}: {}", repo, title);

        Ok(format!("https://github.com/{}/issues/1", repo))
    }

    async fn list_repos(&self) -> Result<Vec<String>> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("github".to_string()))?;

        // In production, call GitHub API
        // GET /user/repos

        Ok(vec!["user/repo1".to_string(), "user/repo2".to_string()])
    }
}

/// GitLab integration
pub struct GitLabIntegration {
    token: Option<String>,
    base_url: String,
}

impl Default for GitLabIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl GitLabIntegration {
    pub fn new() -> Self {
        Self {
            token: None,
            base_url: "https://gitlab.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }
}

#[async_trait]
impl Integration for GitLabIntegration {
    fn name(&self) -> &str {
        "gitlab"
    }

    fn integration_type(&self) -> &str {
        "devtool"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    async fn authenticate(&mut self, token: &str) -> Result<()> {
        self.token = Some(token.to_string());
        info!("GitLab authenticated");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.token = None;
        Ok(())
    }

    fn capabilities_dx(&self) -> String {
        "capabilities:6[create_mr create_issue list_projects commit push clone]".to_string()
    }
}

#[async_trait]
impl DevToolIntegration for GitLabIntegration {
    async fn create_pr(
        &self,
        repo: &str,
        title: &str,
        _body: &str,
        _branch: &str,
    ) -> Result<String> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("gitlab".to_string()))?;

        info!("Creating MR on {}: {}", repo, title);

        Ok(format!("{}/{}/merge_requests/1", self.base_url, repo))
    }

    async fn create_issue(&self, repo: &str, title: &str, _body: &str) -> Result<String> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("gitlab".to_string()))?;

        info!("Creating issue on {}: {}", repo, title);

        Ok(format!("{}/{}/issues/1", self.base_url, repo))
    }

    async fn list_repos(&self) -> Result<Vec<String>> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("gitlab".to_string()))?;

        Ok(vec![
            "user/project1".to_string(),
            "user/project2".to_string(),
        ])
    }
}
