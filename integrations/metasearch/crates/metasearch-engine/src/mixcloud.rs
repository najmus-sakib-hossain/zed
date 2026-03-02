//! Mixcloud engine — search music mixes via Mixcloud JSON API.
//! Translated from SearXNG `searx/engines/mixcloud.py`.

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

pub struct Mixcloud {
    metadata: EngineMetadata,
    client: Client,
}

impl Mixcloud {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "mixcloud".to_string().into(),
                display_name: "Mixcloud".to_string().into(),
                homepage: "https://www.mixcloud.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Mixcloud {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let offset = (page - 1) * 10;

        let url = format!(
            "https://api.mixcloud.com/search/?q={}&type=cloudcast&limit=10&offset={}",
            urlencoding::encode(&query.query),
            offset,
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

        if let Some(items) = data["data"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let title = item["name"].as_str().unwrap_or_default();
                let link = item["url"].as_str().unwrap_or_default();
                let user_name = item["user"]["name"].as_str().unwrap_or_default();

                let mut result = SearchResult::new(
                    title.to_string(),
                    link.to_string(),
                    format!("by {}", user_name),
                    "mixcloud".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Music.to_string();
                result.thumbnail = item["pictures"]["medium"].as_str().map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
