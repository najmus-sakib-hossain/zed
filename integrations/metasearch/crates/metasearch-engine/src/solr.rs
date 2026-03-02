//! Apache Solr — search via a self-hosted Solr instance.
//!
//! Uses the Solr JSON select API.
//!
//! Reference: <https://solr.apache.org>

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

pub struct Solr {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl Solr {
    pub fn new(client: Client, base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let enabled = !base.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "solr".to_string().into(),
                display_name: "Solr".to_string().into(),
                homepage: "https://solr.apache.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: base,
        }
    }
}

#[async_trait]
impl SearchEngine for Solr {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let start = (query.page.saturating_sub(1)) * 10;
        let url = format!(
            "{}/select?q={}&wt=json&rows=10&start={}",
            self.base_url,
            urlencoding::encode(&query.query),
            start,
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

        let docs = match json
            .get("response")
            .and_then(|r| r.get("docs"))
            .and_then(|d| d.as_array())
        {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, doc) in docs.iter().enumerate() {
            let obj = match doc.as_object() {
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
                .or_else(|| obj.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let doc_url = obj
                .get("url")
                .or_else(|| obj.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if doc_url.is_empty() {
                continue;
            }

            let mut result = SearchResult::new(&title, &doc_url, &content, "solr");
            result.engine_rank = (i + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
