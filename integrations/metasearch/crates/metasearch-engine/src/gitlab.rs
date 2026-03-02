//! GitLab engine — search projects via GitLab REST API.
//! Translated from SearXNG `searx/engines/gitlab.py`.
//!
//! Works with any GitLab instance (gitlab.com, self-hosted, etc.).

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct GitLab {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl GitLab {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "gitlab".to_string().into(),
                display_name: "GitLab".to_string().into(),
                homepage: "https://gitlab.com".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: "https://gitlab.com".to_string(),
        }
    }

    pub fn with_base_url(client: Client, base_url: &str) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "gitlab".to_string().into(),
                display_name: "GitLab".to_string().into(),
                homepage: base_url.to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for GitLab {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;

        let url = format!(
            "{}/api/v4/projects?search={}&page={}",
            self.base_url,
            urlencoding::encode(&query.query),
            page,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(projects) = data.as_array() {
            for (i, item) in projects.iter().enumerate() {
                let name = item["name"].as_str().unwrap_or_default();
                let web_url = item["web_url"].as_str().unwrap_or_default();
                let description = item["description"].as_str().unwrap_or("");
                let stars = item["star_count"].as_u64().unwrap_or(0);
                let forks = item["forks_count"].as_u64().unwrap_or(0);
                let namespace = item["namespace"]["name"].as_str().unwrap_or("");

                let snippet = format!(
                    "{} — ⭐ {} | 🍴 {} | by {}",
                    description, stars, forks, namespace,
                );

                let mut result = SearchResult::new(
                    name.to_string(),
                    web_url.to_string(),
                    snippet,
                    "gitlab".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                result.thumbnail = item["avatar_url"].as_str().map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
