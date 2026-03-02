//! Frinkiac engine — search Simpsons screenshots.
//! Translated from SearXNG `searx/engines/frinkiac.py`.

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

const BASE: &str = "https://frinkiac.com/";

pub struct Frinkiac {
    metadata: EngineMetadata,
    client: Client,
}

impl Frinkiac {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "frinkiac".to_string().into(),
                display_name: "Frinkiac".to_string().into(),
                homepage: "https://frinkiac.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Frinkiac {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!("{}api/search?q={}", BASE, urlencoding::encode(&query.query),);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        for (i, item) in data.iter().enumerate() {
            let episode = item["Episode"].as_str().unwrap_or_default();
            let timestamp = item["Timestamp"].as_u64().unwrap_or(0);

            let result_url = format!("{}?p=caption&e={}&t={}", BASE, episode, timestamp,);
            let thumb = format!("{}img/{}/{}/medium.jpg", BASE, episode, timestamp,);
            let img = format!("{}img/{}/{}.jpg", BASE, episode, timestamp,);

            let mut result = SearchResult::new(
                episode.to_string(),
                result_url,
                String::new(),
                "frinkiac".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            result.thumbnail = Some(thumb);
            result.metadata = serde_json::json!({ "img_src": img });
            results.push(result);
        }

        Ok(results)
    }
}
