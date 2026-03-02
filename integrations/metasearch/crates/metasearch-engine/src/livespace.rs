//! LiveSpace — live stream video search.
//!
//! Uses the LiveSpace JSON API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct LiveSpace {
    metadata: EngineMetadata,
    client: Client,
}

impl LiveSpace {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "livespace".to_string().into(),
                display_name: "LiveSpace".to_string().into(),
                homepage: "https://live.space".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for LiveSpace {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;

        let resp = self
            .client
            .get("https://backend.live.space/search/public/stream")
            .query(&[
                ("searchKey", query.query.as_str()),
                ("page", &(page.saturating_sub(1)).to_string()),
                ("size", "10"),
            ])
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .send()
            .await;

        // live.space may be unreachable — return empty gracefully
        let resp = match resp {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(items) = json.get("result").and_then(|v| v.as_array()) {
            for (i, item) in items.iter().enumerate() {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let username = item
                    .get("user")
                    .and_then(|u| u.get("userName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let thumbnail = item
                    .get("thumbnailUrl")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let category = item
                    .get("category/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if username.is_empty() {
                    continue;
                }

                let url = format!("https://live.space/watch/{username}");

                let snippet = if category.is_empty() {
                    format!("Live stream by @{username}")
                } else {
                    format!("Live stream by @{username} — {category}")
                };

                let mut result =
                    SearchResult::new(title.to_string(), url, snippet, self.metadata.name.clone());
                result.engine_rank = (i + 1) as u32;
                result.thumbnail = thumbnail;
                result.category = SearchCategory::Videos.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
