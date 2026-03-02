//! Recoll desktop full-text search via recoll-webui — configurable instance URL.
//! SearXNG equivalent: `recoll.py`
//!
//! Recoll is a desktop full-text search tool based on Xapian. This engine
//! queries via recoll-webui's JSON API. Configure `base_url`, `mount_prefix`,
//! and `dl_prefix` to map local file paths to web-accessible URLs.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct RecollEngine {
    client: Client,
    base_url: String,
    mount_prefix: String,
    dl_prefix: String,
}

impl RecollEngine {
    pub fn new(client: Client, base_url: &str, mount_prefix: &str, dl_prefix: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            mount_prefix: mount_prefix.to_string(),
            dl_prefix: dl_prefix.to_string(),
        }
    }
}

#[derive(Deserialize)]
struct RecollResponse {
    results: Option<Vec<RecollResult>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RecollResult {
    label: Option<String>,
    url: Option<String>,
    snippet: Option<String>,
    filename: Option<String>,
    author: Option<String>,
    mtype: Option<String>,
}

#[async_trait]
impl SearchEngine for RecollEngine {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "recoll".to_string().into(),
            display_name: "Recoll".to_string().into(),
            homepage: "https://www.recoll.org".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.base_url.is_empty(),
            timeout_ms: 5000,
            weight: 0.8,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/json?query={}&page={}&highlight=0",
            self.base_url,
            urlencoding::encode(&query.query),
            query.page,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Recoll: {e}")))?;

        let data: RecollResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Recoll JSON: {e}")))?;

        let items = data.results.unwrap_or_default();

        let mut results = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let title = item.label.clone().unwrap_or_default();
            let raw_url = item.url.clone().unwrap_or_default();

            // Map local file:// paths to web-accessible URLs
            let result_url = if !self.mount_prefix.is_empty() && !self.dl_prefix.is_empty() {
                raw_url.replace(&format!("file://{}", self.mount_prefix), &self.dl_prefix)
            } else {
                raw_url
            };

            // Strip HTML from snippet
            let content = item
                .snippet
                .as_deref()
                .unwrap_or("")
                .replace("<b>", "")
                .replace("</b>", "")
                .replace("<em>", "")
                .replace("</em>", "");

            results.push(SearchResult {
                title,
                url: result_url,
                content,
                engine: "Recoll".to_string(),
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
