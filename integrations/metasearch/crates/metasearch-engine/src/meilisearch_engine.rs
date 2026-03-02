//! MeiliSearch search — configurable instance URL, index, and auth key.
//! SearXNG equivalent: `meilisearch.py`
//!
//! MeiliSearch is a lightweight search engine for small-scale data.
//! Configure `base_url`, `index`, and optionally `auth_key` for authentication.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct MeilisearchEngine {
    client: Client,
    base_url: String,
    index: String,
    auth_key: Option<String>,
}

impl MeilisearchEngine {
    pub fn new(client: Client, base_url: &str, index: &str, auth_key: Option<String>) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            index: index.to_string(),
            auth_key,
        }
    }
}

#[async_trait]
impl SearchEngine for MeilisearchEngine {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "MeiliSearch".to_string().into(),
            display_name: "MeiliSearch".to_string().into(),
            homepage: "https://www.meilisearch.com".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.base_url.is_empty() && !self.index.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() || self.index.is_empty() {
            return Ok(Vec::new());
        }

        let offset = (query.page - 1) * 10;
        let body = serde_json::json!({
            "q": query.query,
            "offset": offset,
            "limit": 10
        });

        let url = format!("{}/indexes/{}/search", self.base_url, self.index);

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string());

        if let Some(ref key) = self.auth_key {
            req = req.header("Authorization", key.as_str());
        }

        let resp = req
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("MeiliSearch: {e}")))?;

        let json: Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("MeiliSearch JSON: {e}")))?;

        let hits = json["hits"].as_array().cloned().unwrap_or_default();

        let mut results = Vec::new();
        for (i, hit) in hits.iter().enumerate() {
            let obj = match hit.as_object() {
                Some(o) => o,
                None => continue,
            };
            let title = obj
                .get("title")
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let content = obj
                .get("content")
                .or_else(|| obj.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let doc_url = obj
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SearchResult {
                title,
                url: if doc_url.is_empty() {
                    format!("{}#result-{}", self.base_url, i)
                } else {
                    doc_url
                },
                content,
                engine: "MeiliSearch".to_string(),
                engine_rank: (i + 1) as u32,
                score: 0.0,
                thumbnail: None,
                published_date: None,
                category: String::new(),
                metadata: serde_json::Value::Null,
            });
        }
        Ok(results)
    }
}
