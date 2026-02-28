//! Web search tool using SearXNG or direct search APIs.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::definition::*;

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Web search tool
pub struct WebSearchTool {
    /// SearXNG instance URL
    pub searxng_url: Option<String>,
    /// HTTP client
    http: reqwest::Client,
}

impl WebSearchTool {
    pub fn new(searxng_url: Option<String>) -> Self {
        Self {
            searxng_url,
            http: reqwest::Client::new(),
        }
    }

    /// Search using SearXNG
    async fn search_searxng(&self, query: &str, count: usize) -> Result<Vec<SearchResult>> {
        let url = self
            .searxng_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SearXNG URL not configured"))?;

        #[derive(Deserialize)]
        struct SearXResponse {
            results: Vec<SearXResult>,
        }

        #[derive(Deserialize)]
        struct SearXResult {
            title: String,
            url: String,
            content: Option<String>,
        }

        let resp = self
            .http
            .get(format!("{}/search", url))
            .query(&[
                ("q", query),
                ("format", "json"),
                ("results", &count.to_string()),
            ])
            .send()
            .await?
            .json::<SearXResponse>()
            .await?;

        Ok(resp
            .results
            .into_iter()
            .take(count)
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.content.unwrap_or_default(),
            })
            .collect())
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".into(),
            description: "Search the web for information".into(),
            parameters: vec![
                ToolParameter {
                    name: "query".into(),
                    description: "Search query".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "count".into(),
                    description: "Number of results (default: 5)".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(serde_json::json!(5)),
                    enum_values: None,
                },
            ],
            category: "search".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let query = call
            .arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query'"))?;

        let count = call.arguments.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        info!("Web search: {} (count={})", query, count);

        let results = self.search_searxng(query, count).await?;

        let output = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. [{}]({})\n   {}", i + 1, r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(ToolResult::success(call.id, output).with_data(serde_json::to_value(&results)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_definition() {
        let tool = WebSearchTool::new(None);
        let def = tool.definition();
        assert_eq!(def.name, "web_search");
        assert_eq!(def.category, "search");
    }
}
