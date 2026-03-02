//! Sogou Videos search engine implementation.
//!
//! Queries the Sogou Videos JSON API for video results.
//! Website: https://v.sogou.com
//! Features: Paging

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

pub struct SogouVideos {
    metadata: EngineMetadata,
    client: Client,
}

impl SogouVideos {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "sogou_videos".to_string().into(),
                display_name: "Sogou Videos".to_string().into(),
                homepage: "https://v.sogou.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for SogouVideos {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.max(1);
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://v.sogou.com/api/video/shortVideoV2?page={}&pagesize=10&query={}",
            page, encoded
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        let items = data["data"]["list"].as_array();
        if let Some(items) = items {
            for (i, item) in items.iter().enumerate() {
                let title = item["titleEsc"].as_str().unwrap_or_default();
                let raw_url = item["url"].as_str().unwrap_or_default();

                if title.is_empty() || raw_url.is_empty() {
                    continue;
                }

                let item_url = if raw_url.starts_with('/') {
                    format!("https://v.sogou.com{}", raw_url)
                } else {
                    raw_url.to_string()
                };

                let content = item["site"].as_str().unwrap_or_default();
                let thumbnail = item["picurl"].as_str().map(|s| s.to_string());

                let mut r = SearchResult::new(title, &item_url, content, "sogou_videos");
                r.engine_rank = i as u32;
                r.category = SearchCategory::Videos.to_string();
                r.thumbnail = thumbnail;
                results.push(r);
            }
        }

        Ok(results)
    }
}
