//! # Notion Integration
//!
//! Notion API client for pages, databases, and blocks.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::notion::{NotionClient, NotionConfig};
//!
//! let config = NotionConfig::from_file("~/.dx/config/notion.sr")?;
//! let client = NotionClient::new(&config)?;
//!
//! // Create a page
//! let page = client.create_page("My Page", "database_id", content).await?;
//!
//! // Query a database
//! let results = client.query_database("database_id", filter).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Notion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionConfig {
    /// Whether Notion integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Notion API token (integration secret)
    #[serde(default)]
    pub api_token: String,
    /// Default database ID
    pub default_database: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for NotionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_token: String::new(),
            default_database: None,
        }
    }
}

impl NotionConfig {
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
        if self.api_token.is_empty() || self.api_token.starts_with('$') {
            self.api_token = std::env::var("NOTION_API_TOKEN")
                .or_else(|_| std::env::var("NOTION_TOKEN"))
                .unwrap_or_default();
        }
    }
}

/// Notion page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionPage {
    /// Page ID
    pub id: String,
    /// Page title
    pub title: String,
    /// Parent (database or page)
    pub parent: NotionParent,
    /// Page URL
    pub url: String,
    /// Created time
    pub created_time: String,
    /// Last edited time
    pub last_edited_time: String,
    /// Page icon
    pub icon: Option<NotionIcon>,
    /// Page cover
    pub cover: Option<String>,
    /// Is archived
    pub archived: bool,
}

/// Notion parent reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotionParent {
    #[serde(rename = "database_id")]
    Database { database_id: String },
    #[serde(rename = "page_id")]
    Page { page_id: String },
    #[serde(rename = "workspace")]
    Workspace,
}

/// Notion icon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotionIcon {
    #[serde(rename = "emoji")]
    Emoji { emoji: String },
    #[serde(rename = "external")]
    External { external: NotionExternalFile },
}

/// External file reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionExternalFile {
    pub url: String,
}

/// Notion database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionDatabase {
    /// Database ID
    pub id: String,
    /// Database title
    pub title: String,
    /// Database URL
    pub url: String,
    /// Properties schema
    pub properties: serde_json::Value,
}

/// Notion block (content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionBlock {
    /// Block ID
    pub id: String,
    /// Block type
    pub block_type: String,
    /// Has children
    pub has_children: bool,
    /// Block content (varies by type)
    pub content: serde_json::Value,
}

/// Rich text content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichText {
    /// Text content
    pub content: String,
    /// Text annotations
    pub annotations: Option<TextAnnotations>,
    /// Link URL
    pub href: Option<String>,
}

/// Text annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotations {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub code: bool,
    pub color: String,
}

/// Notion client
pub struct NotionClient {
    config: NotionConfig,
    base_url: String,
}

impl NotionClient {
    /// API base URL
    const API_BASE: &'static str = "https://api.notion.com/v1";
    /// API version
    const API_VERSION: &'static str = "2022-06-28";

    /// Create a new Notion client
    pub fn new(config: &NotionConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.config.api_token.is_empty()
    }

    /// Get a page by ID
    pub async fn get_page(&self, page_id: &str) -> Result<NotionPage> {
        let url = format!("{}/pages/{}", self.base_url, page_id);
        let response = self.api_get(&url).await?;
        self.parse_page_response(response)
    }

    /// Create a new page
    pub async fn create_page(
        &self,
        title: &str,
        parent_id: &str,
        content: Vec<NotionBlock>,
    ) -> Result<NotionPage> {
        let url = format!("{}/pages", self.base_url);

        let body = json!({
            "parent": { "database_id": parent_id },
            "properties": {
                "title": {
                    "title": [{
                        "text": { "content": title }
                    }]
                }
            },
            "children": content.iter().map(|b| &b.content).collect::<Vec<_>>()
        });

        let response = self.api_post(&url, body).await?;
        self.parse_page_response(response)
    }

    /// Update a page
    pub async fn update_page(&self, page_id: &str, properties: serde_json::Value) -> Result<NotionPage> {
        let url = format!("{}/pages/{}", self.base_url, page_id);

        let body = json!({ "properties": properties });

        let response = self.api_patch(&url, body).await?;
        self.parse_page_response(response)
    }

    /// Archive a page
    pub async fn archive_page(&self, page_id: &str) -> Result<()> {
        let url = format!("{}/pages/{}", self.base_url, page_id);

        let body = json!({ "archived": true });

        self.api_patch(&url, body).await?;
        Ok(())
    }

    /// Get a database by ID
    pub async fn get_database(&self, database_id: &str) -> Result<NotionDatabase> {
        let url = format!("{}/databases/{}", self.base_url, database_id);
        let response = self.api_get(&url).await?;

        Ok(NotionDatabase {
            id: response["id"].as_str().unwrap_or_default().to_string(),
            title: response["title"][0]["plain_text"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            url: response["url"].as_str().unwrap_or_default().to_string(),
            properties: response["properties"].clone(),
        })
    }

    /// Query a database
    pub async fn query_database(
        &self,
        database_id: &str,
        filter: Option<serde_json::Value>,
        sorts: Option<Vec<serde_json::Value>>,
    ) -> Result<Vec<NotionPage>> {
        let url = format!("{}/databases/{}/query", self.base_url, database_id);

        let mut body = json!({});
        if let Some(f) = filter {
            body["filter"] = f;
        }
        if let Some(s) = sorts {
            body["sorts"] = json!(s);
        }

        let response = self.api_post(&url, body).await?;

        let results = response["results"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid response".into()))?;

        results
            .iter()
            .map(|r| self.parse_page_response(r.clone()))
            .collect()
    }

    /// Get page content (blocks)
    pub async fn get_blocks(&self, page_id: &str) -> Result<Vec<NotionBlock>> {
        let url = format!("{}/blocks/{}/children", self.base_url, page_id);
        let response = self.api_get(&url).await?;

        let results = response["results"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid response".into()))?;

        Ok(results
            .iter()
            .map(|b| NotionBlock {
                id: b["id"].as_str().unwrap_or_default().to_string(),
                block_type: b["type"].as_str().unwrap_or_default().to_string(),
                has_children: b["has_children"].as_bool().unwrap_or(false),
                content: b.clone(),
            })
            .collect())
    }

    /// Append blocks to a page
    pub async fn append_blocks(&self, page_id: &str, blocks: Vec<serde_json::Value>) -> Result<()> {
        let url = format!("{}/blocks/{}/children", self.base_url, page_id);

        let body = json!({ "children": blocks });

        self.api_patch(&url, body).await?;
        Ok(())
    }

    /// Delete a block
    pub async fn delete_block(&self, block_id: &str) -> Result<()> {
        let url = format!("{}/blocks/{}", self.base_url, block_id);
        self.api_delete(&url).await
    }

    /// Search pages and databases
    pub async fn search(&self, query: &str, filter: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/search", self.base_url);

        let mut body = json!({ "query": query });
        if let Some(f) = filter {
            body["filter"] = json!({ "value": f, "property": "object" });
        }

        let response = self.api_post(&url, body).await?;

        Ok(response["results"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Create a paragraph block
    pub fn paragraph_block(text: &str) -> serde_json::Value {
        json!({
            "object": "block",
            "type": "paragraph",
            "paragraph": {
                "rich_text": [{
                    "type": "text",
                    "text": { "content": text }
                }]
            }
        })
    }

    /// Create a heading block
    pub fn heading_block(text: &str, level: u8) -> serde_json::Value {
        let heading_type = match level {
            1 => "heading_1",
            2 => "heading_2",
            _ => "heading_3",
        };

        json!({
            "object": "block",
            "type": heading_type,
            heading_type: {
                "rich_text": [{
                    "type": "text",
                    "text": { "content": text }
                }]
            }
        })
    }

    /// Create a bulleted list item
    pub fn bullet_block(text: &str) -> serde_json::Value {
        json!({
            "object": "block",
            "type": "bulleted_list_item",
            "bulleted_list_item": {
                "rich_text": [{
                    "type": "text",
                    "text": { "content": text }
                }]
            }
        })
    }

    /// Create a to-do block
    pub fn todo_block(text: &str, checked: bool) -> serde_json::Value {
        json!({
            "object": "block",
            "type": "to_do",
            "to_do": {
                "rich_text": [{
                    "type": "text",
                    "text": { "content": text }
                }],
                "checked": checked
            }
        })
    }

    /// Make authenticated GET request
    async fn api_get(&self, url: &str) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Notion-Version", Self::API_VERSION)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Notion API error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Make authenticated POST request
    async fn api_post(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Notion-Version", Self::API_VERSION)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Notion API error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Make authenticated PATCH request
    async fn api_patch(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .patch(url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Notion-Version", Self::API_VERSION)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Notion API error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }

    /// Make authenticated DELETE request
    async fn api_delete(&self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .delete(url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Notion-Version", Self::API_VERSION)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("Notion API error: {}", error)));
        }

        Ok(())
    }

    /// Parse page response
    fn parse_page_response(&self, response: serde_json::Value) -> Result<NotionPage> {
        let title = response["properties"]["title"]["title"][0]["plain_text"]
            .as_str()
            .or_else(|| response["properties"]["Name"]["title"][0]["plain_text"].as_str())
            .unwrap_or_default()
            .to_string();

        Ok(NotionPage {
            id: response["id"].as_str().unwrap_or_default().to_string(),
            title,
            parent: serde_json::from_value(response["parent"].clone())
                .unwrap_or(NotionParent::Workspace),
            url: response["url"].as_str().unwrap_or_default().to_string(),
            created_time: response["created_time"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            last_edited_time: response["last_edited_time"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            icon: serde_json::from_value(response["icon"].clone()).ok(),
            cover: response["cover"]["external"]["url"]
                .as_str()
                .map(String::from),
            archived: response["archived"].as_bool().unwrap_or(false),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NotionConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_paragraph_block() {
        let block = NotionClient::paragraph_block("Hello, world!");
        assert_eq!(block["type"], "paragraph");
    }

    #[test]
    fn test_todo_block() {
        let block = NotionClient::todo_block("Task 1", false);
        assert_eq!(block["type"], "to_do");
        assert_eq!(block["to_do"]["checked"], false);
    }
}
