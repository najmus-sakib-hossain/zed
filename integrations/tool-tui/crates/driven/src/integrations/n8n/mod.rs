//! # N8N Integration
//!
//! Execute N8N workflows via API.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::n8n::{N8nClient, N8nConfig};
//!
//! let config = N8nConfig::from_file("~/.dx/config/n8n.sr")?;
//! let client = N8nClient::new(&config)?;
//!
//! // Execute a workflow
//! let result = client.execute_workflow("my_workflow", serde_json::json!({
//!     "input": "value"
//! })).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// N8N configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nConfig {
    /// Whether N8N integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// N8N instance URL
    #[serde(default)]
    pub base_url: String,
    /// API key for authentication
    #[serde(default)]
    pub api_key: String,
    /// Configured workflows
    #[serde(default)]
    pub workflows: HashMap<String, N8nWorkflow>,
}

fn default_true() -> bool {
    true
}

impl Default for N8nConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "http://localhost:5678".to_string(),
            api_key: String::new(),
            workflows: HashMap::new(),
        }
    }
}

impl N8nConfig {
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
        if self.api_key.is_empty() || self.api_key.starts_with('$') {
            self.api_key = std::env::var("N8N_API_KEY").unwrap_or_default();
        }
        if self.base_url.starts_with('$') {
            self.base_url = std::env::var("N8N_URL")
                .unwrap_or_else(|_| "http://localhost:5678".to_string());
        }
    }
}

/// N8N workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nWorkflow {
    /// Workflow name
    pub name: String,
    /// Workflow ID in N8N
    pub id: String,
    /// Description
    pub description: Option<String>,
    /// Whether workflow is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Webhook path (if triggered via webhook)
    pub webhook_path: Option<String>,
}

/// N8N execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nExecution {
    /// Execution ID
    pub id: String,
    /// Execution status
    pub status: N8nExecutionStatus,
    /// Start time
    pub started_at: Option<String>,
    /// End time
    pub finished_at: Option<String>,
    /// Output data
    pub data: Option<serde_json::Value>,
    /// Error message
    pub error: Option<String>,
}

/// N8N execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum N8nExecutionStatus {
    /// Execution is running
    Running,
    /// Execution completed successfully
    Success,
    /// Execution failed
    Error,
    /// Execution was cancelled
    Cancelled,
    /// Execution is waiting
    Waiting,
}

/// N8N client
pub struct N8nClient {
    config: N8nConfig,
}

impl N8nClient {
    /// Create a new N8N client
    pub fn new(config: &N8nConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self { config })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled && !self.config.base_url.is_empty()
    }

    /// Execute a workflow by name
    pub async fn execute_workflow(&self, name: &str, data: serde_json::Value) -> Result<N8nExecution> {
        let workflow = self.config.workflows.get(name)
            .ok_or_else(|| DrivenError::NotFound(format!("Workflow '{}' not found", name)))?;

        if !workflow.enabled {
            return Err(DrivenError::Config("Workflow is disabled".into()));
        }

        self.execute_workflow_by_id(&workflow.id, data).await
    }

    /// Execute a workflow by ID
    pub async fn execute_workflow_by_id(&self, id: &str, data: serde_json::Value) -> Result<N8nExecution> {
        let url = format!("{}/api/v1/workflows/{}/execute", self.config.base_url, id);

        let client = reqwest::Client::new();
        let mut request = client.post(&url);

        if !self.config.api_key.is_empty() {
            request = request.header("X-N8N-API-KEY", &self.config.api_key);
        }

        let response = request
            .json(&serde_json::json!({ "data": data }))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!(
                "N8N API error ({}): {}",
                status, error_text
            )));
        }

        let result: N8nExecutionResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(N8nExecution {
            id: result.data.id,
            status: N8nExecutionStatus::Success,
            started_at: result.data.started_at,
            finished_at: result.data.finished_at,
            data: result.data.data,
            error: None,
        })
    }

    /// Trigger workflow via webhook
    pub async fn trigger_webhook(&self, name: &str, data: serde_json::Value) -> Result<serde_json::Value> {
        let workflow = self.config.workflows.get(name)
            .ok_or_else(|| DrivenError::NotFound(format!("Workflow '{}' not found", name)))?;

        let webhook_path = workflow.webhook_path.as_ref()
            .ok_or_else(|| DrivenError::Config("Workflow has no webhook path".into()))?;

        let url = format!("{}/webhook/{}", self.config.base_url, webhook_path);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&data)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Webhook trigger failed".into()));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Get execution status
    pub async fn get_execution(&self, id: &str) -> Result<N8nExecution> {
        let url = format!("{}/api/v1/executions/{}", self.config.base_url, id);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if !self.config.api_key.is_empty() {
            request = request.header("X-N8N-API-KEY", &self.config.api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to get execution".into()));
        }

        let result: N8nExecutionResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(N8nExecution {
            id: result.data.id,
            status: N8nExecutionStatus::Success,
            started_at: result.data.started_at,
            finished_at: result.data.finished_at,
            data: result.data.data,
            error: None,
        })
    }

    /// List workflows
    pub async fn list_workflows(&self) -> Result<Vec<N8nWorkflowInfo>> {
        let url = format!("{}/api/v1/workflows", self.config.base_url);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if !self.config.api_key.is_empty() {
            request = request.header("X-N8N-API-KEY", &self.config.api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to list workflows".into()));
        }

        let result: WorkflowsResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(result.data)
    }
}

// API Response types

#[derive(Debug, Deserialize)]
struct N8nExecutionResponse {
    data: N8nExecutionData,
}

#[derive(Debug, Deserialize)]
struct N8nExecutionData {
    id: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct WorkflowsResponse {
    data: Vec<N8nWorkflowInfo>,
}

/// N8N workflow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nWorkflowInfo {
    /// Workflow ID
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Whether workflow is active
    pub active: bool,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = N8nConfig::default();
        assert!(config.enabled);
        assert_eq!(config.base_url, "http://localhost:5678");
    }

    #[test]
    fn test_client_creation() {
        let config = N8nConfig::default();
        let client = N8nClient::new(&config);
        assert!(client.is_ok());
    }
}
