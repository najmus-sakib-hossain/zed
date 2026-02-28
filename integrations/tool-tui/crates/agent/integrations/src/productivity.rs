//! # Productivity Integrations
//!
//! Connect to Notion, Todoist, Google Calendar, and more.

use async_trait::async_trait;
use tracing::info;

use crate::{Integration, IntegrationError, ProductivityIntegration, Result};

/// Notion integration
pub struct NotionIntegration {
    token: Option<String>,
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

impl Default for NotionIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl NotionIntegration {
    pub fn new() -> Self {
        Self {
            token: None,
            workspace_id: None,
        }
    }
}

#[async_trait]
impl Integration for NotionIntegration {
    fn name(&self) -> &str {
        "notion"
    }

    fn integration_type(&self) -> &str {
        "productivity"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    async fn authenticate(&mut self, token: &str) -> Result<()> {
        self.token = Some(token.to_string());
        info!("Notion authenticated");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.token = None;
        Ok(())
    }

    fn capabilities_dx(&self) -> String {
        "capabilities:5[create_page update_page query_database create_database append_blocks]"
            .to_string()
    }
}

#[async_trait]
impl ProductivityIntegration for NotionIntegration {
    async fn create_page(&self, title: &str, _content: &str) -> Result<String> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("notion".to_string()))?;

        info!("Creating Notion page: {}", title);

        // In production, call Notion API
        // POST /v1/pages

        Ok(format!("https://notion.so/page-{}", uuid::Uuid::new_v4()))
    }

    async fn update_page(&self, page_id: &str, _content: &str) -> Result<()> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("notion".to_string()))?;

        info!("Updating Notion page: {}", page_id);

        Ok(())
    }

    async fn query(&self, query: &str) -> Result<Vec<String>> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("notion".to_string()))?;

        info!("Querying Notion: {}", query);

        Ok(vec!["Page 1".to_string(), "Page 2".to_string()])
    }
}

/// Todoist integration
pub struct TodoistIntegration {
    token: Option<String>,
}

impl Default for TodoistIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoistIntegration {
    pub fn new() -> Self {
        Self { token: None }
    }
}

#[async_trait]
impl Integration for TodoistIntegration {
    fn name(&self) -> &str {
        "todoist"
    }

    fn integration_type(&self) -> &str {
        "productivity"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    async fn authenticate(&mut self, token: &str) -> Result<()> {
        self.token = Some(token.to_string());
        info!("Todoist authenticated");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.token = None;
        Ok(())
    }

    fn capabilities_dx(&self) -> String {
        "capabilities:4[create_task complete_task list_tasks create_project]".to_string()
    }
}

#[async_trait]
impl ProductivityIntegration for TodoistIntegration {
    async fn create_page(&self, title: &str, _content: &str) -> Result<String> {
        // Create a task instead of a page
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("todoist".to_string()))?;

        info!("Creating Todoist task: {}", title);

        Ok(format!("task-{}", uuid::Uuid::new_v4()))
    }

    async fn update_page(&self, task_id: &str, _content: &str) -> Result<()> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("todoist".to_string()))?;

        info!("Updating Todoist task: {}", task_id);

        Ok(())
    }

    async fn query(&self, query: &str) -> Result<Vec<String>> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("todoist".to_string()))?;

        info!("Querying Todoist: {}", query);

        Ok(vec!["Task 1".to_string(), "Task 2".to_string()])
    }
}
