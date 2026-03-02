//! Deezer engine — search music tracks via Deezer API.
//! Translated from SearXNG `searx/engines/deezer.py`.

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

pub struct Deezer {
    metadata: EngineMetadata,
    client: Client,
}

impl Deezer {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "deezer".to_string().into(),
                display_name: "Deezer".to_string().into(),
                homepage: "https://www.deezer.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Deezer {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let offset = (page - 1) * 25;

        let url = format!(
            "https://api.deezer.com/search?q={}&index={}",
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
                if item["type"].as_str() != Some("track") {
                    continue;
                }

                let title = item["title"].as_str().unwrap_or_default();
                let mut link = item["link"].as_str().unwrap_or_default().to_string();
                if link.starts_with("http://") {
                    link = format!("https{}", &link[4..]);
                }

                let artist = item["artist"]["name"].as_str().unwrap_or("");
                let album = item["album"]["title"].as_str().unwrap_or("");
                let duration = item["duration"].as_u64().unwrap_or(0);
                let minutes = duration / 60;
                let seconds = duration % 60;

                let snippet = format!("{} — {} [{}:{:02}]", artist, album, minutes, seconds,);

                let mut result =
                    SearchResult::new(title.to_string(), link, snippet, "deezer".to_string());
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Music.to_string();
                result.thumbnail = item["album"]["cover_medium"]
                    .as_str()
                    .map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
