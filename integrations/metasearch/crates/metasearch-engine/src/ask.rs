//! Ask.com search engine implementation.
//!
//! Translated from SearXNG's `ask.py` (75 lines, HTML + JS JSON).
//! Ask.com is a general web search engine.
//! Website: https://www.ask.com/
//! Features: Paging (max 5 pages)

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
// Removed unused imports - using regex and JSON parsing instead
use tracing::info;
use smallvec::smallvec;

pub struct Ask {
    metadata: EngineMetadata,
    client: Client,
}

impl Ask {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "ask".to_string().into(),
                display_name: "Ask.com".to_string().into(),
                homepage: "https://www.ask.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 4000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Ask {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Ask.com has at max 5 pages
        let page = query.page.clamp(1, 5);
        let encoded = urlencoding::encode(&query.query);

        let url = format!("https://www.ask.com/web?q={}&page={}", encoded, page);

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Ask.com embeds results in JavaScript: window.MESON.initialState = {...}
        let start_tag = "window.MESON.initialState = {";
        let end_tag = "}};";

        let start_pos = match body.find(start_tag) {
            Some(pos) => pos + start_tag.len() - 1,
            None => return Ok(Vec::new()),
        };

        let remaining = &body[start_pos..];
        let end_pos = match remaining.find(end_tag) {
            Some(pos) => pos + end_tag.len() - 1,
            None => return Ok(Vec::new()),
        };

        let json_str = &remaining[..end_pos];

        // Parse the JSON
        let data: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        // Navigate to search.webResults.results
        if let Some(items) = data
            .get("search")
            .and_then(|s| s.get("webResults"))
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
        {
            for (i, item) in items.iter().enumerate() {
                let title = item["title"].as_str().unwrap_or_default();
                let mut url = item["url"].as_str().unwrap_or_default().to_string();
                let abstract_text = item["abstract"].as_str().unwrap_or_default();

                if title.is_empty() || url.is_empty() {
                    continue;
                }

                // Strip &ueid parameter
                if let Some(idx) = url.find("&ueid") {
                    url.truncate(idx);
                }

                let mut r = SearchResult::new(title, &url, abstract_text, "ask");
                r.engine_rank = (i + 1) as u32;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        info!(engine = "ask", count = results.len(), "Search complete");
        Ok(results)
    }
}
