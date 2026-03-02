//! Sogou Images search engine implementation.
//!
//! Scrapes Sogou Images by extracting JSON from `window.__INITIAL_STATE__` in HTML.
//! Website: https://pic.sogou.com
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use smallvec::smallvec;

pub struct SogouImages {
    metadata: EngineMetadata,
    client: Client,
}

impl SogouImages {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "sogou_images".to_string().into(),
                display_name: "Sogou Images".to_string().into(),
                homepage: "https://pic.sogou.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for SogouImages {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.max(1);
        let offset = (page - 1) * 48;
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://pic.sogou.com/pics?query={}&start={}",
            encoded, offset
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        // Extract JSON from window.__INITIAL_STATE__ = {...};
        let re = Regex::new(r"window\.__INITIAL_STATE__\s*=\s*(\{.*?\})\s*;")
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let json_str = match re.captures(&body) {
            Some(caps) => caps
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or("{}"),
            None => {
                return Ok(Vec::new());
            }
        };

        let data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| MetasearchError::ParseError(format!("JSON parse error: {}", e)))?;

        let mut results = Vec::new();

        let items = data["searchList"]["searchList"].as_array();
        if let Some(items) = items {
            for (i, item) in items.iter().enumerate() {
                let title = item["title"].as_str().unwrap_or_default();
                let item_url = item["url"].as_str().unwrap_or_default();

                if title.is_empty() || item_url.is_empty() {
                    continue;
                }

                let content = item["content_major"].as_str().unwrap_or_default();
                let thumbnail = item["picUrl"].as_str().map(|s| s.to_string());

                let mut r = SearchResult::new(title, item_url, content, "sogou_images");
                r.engine_rank = i as u32;
                r.category = SearchCategory::Images.to_string();
                r.thumbnail = thumbnail;
                results.push(r);
            }
        }

        Ok(results)
    }
}
