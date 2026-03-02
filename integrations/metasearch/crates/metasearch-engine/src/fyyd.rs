//! Fyyd engine — search podcasts via Fyyd JSON API.
//! Translated from SearXNG `searx/engines/fyyd.py`.

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

pub struct Fyyd {
    metadata: EngineMetadata,
    client: Client,
}

impl Fyyd {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "fyyd".to_string().into(),
                display_name: "Fyyd".to_string().into(),
                homepage: "https://fyyd.de".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Fyyd {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let page_index = (page as i32) - 1;
        let count = 10;

        let url = format!(
            "https://api.fyyd.de/0.2/search/podcast?term={}&count={}&page={}",
            urlencoding::encode(&query.query),
            count,
            page_index,
        );

        let resp = match self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .timeout(std::time::Duration::from_secs(6))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        // Use text() to handle non-UTF-8 encoding issues, then parse manually
        let text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let data: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(items) = data["data"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let title = item["title"].as_str().unwrap_or_default();
                let link = item["htmlURL"].as_str().unwrap_or_default();
                let description = item["description"].as_str().unwrap_or_default();
                let episode_count = item["episode_count"].as_u64().unwrap_or(0);

                let snippet = if description.is_empty() {
                    format!("{} episodes", episode_count)
                } else {
                    let desc_short = if description.len() > 200 {
                        format!("{}...", &description[..200])
                    } else {
                        description.to_string()
                    };
                    format!("{} | {} episodes", desc_short, episode_count)
                };

                let mut result = SearchResult::new(
                    title.to_string(),
                    link.to_string(),
                    snippet,
                    "fyyd".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Music.to_string();
                result.thumbnail = item["smallImageURL"].as_str().map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
