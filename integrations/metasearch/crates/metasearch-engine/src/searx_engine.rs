//! SearXNG meta-search — query another SearXNG instance via its JSON API.
//!
//! Useful for federating results from a trusted SearXNG deployment.
//!
//! Reference: <https://docs.searxng.org>

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

pub struct SearxEngine {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl SearxEngine {
    pub fn new(client: Client, base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let enabled = !base.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "searx_engine".to_string().into(),
                display_name: "SearXNG".to_string().into(),
                homepage: "https://docs.searxng.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 10000,
                weight: 1.0,
            },
            client,
            base_url: base,
        }
    }
}

#[async_trait]
impl SearchEngine for SearxEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/search?q={}&format=json&pageno={}",
            self.base_url,
            urlencoding::encode(&query.query),
            query.page,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let items = match json.get("results").and_then(|r| r.as_array()) {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let item_url = item["url"].as_str().unwrap_or_default();
            if item_url.is_empty() {
                continue;
            }

            let title = item["title"].as_str().unwrap_or("Untitled").to_string();
            let content = item["content"].as_str().unwrap_or_default().to_string();
            let thumbnail = item["thumbnail"].as_str().map(|s| s.to_string());

            let mut result = SearchResult::new(&title, item_url, &content, "searx_engine");
            result.engine_rank = (i + 1) as u32;
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
