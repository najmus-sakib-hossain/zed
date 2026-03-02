//! YouTube API search engine implementation.
//! Requires API key from https://console.developers.google.com/
//! JSON API: https://www.googleapis.com/youtube/v3/search
//! Features: No pagination

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct YoutubeApi {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl YoutubeApi {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "youtube_api".to_string().into(),
                display_name: "YouTube API".to_string().into(),
                homepage: "https://www.youtube.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos, SearchCategory::Music],
                enabled: api_key.is_some(),
                timeout_ms: 5000,
                weight: 1.5,
            },
            client,
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for YoutubeApi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "youtube_api".to_string(),
                message:
                    "No API key configured. Get one at https://console.developers.google.com/"
                        .to_string(),
            });
        }

        let url = format!(
            "https://www.googleapis.com/youtube/v3/search?part=snippet&q={}&maxResults=20&key={}",
            urlencoding::encode(&query.query),
            urlencoding::encode(key),
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
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        let items = match data.get("items").and_then(|i| i.as_array()) {
            Some(items) => items,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let video_id = match item
                .get("id")
                .and_then(|id| id.get("videoId"))
                .and_then(|v| v.as_str())
            {
                Some(id) => id,
                None => continue,
            };

            let snippet = match item.get("snippet") {
                Some(s) => s,
                None => continue,
            };

            let title = snippet
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let description = snippet
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or_default();

            let video_url = format!("https://www.youtube.com/watch?v={}", video_id);

            let thumbnail = snippet
                .get("thumbnails")
                .and_then(|t| t.get("high"))
                .and_then(|h| h.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());

            let published_date = snippet
                .get("publishedAt")
                .and_then(|p| p.as_str())
                .and_then(|s| s.parse::<DateTime<Utc>>().ok());

            let mut r = SearchResult::new(title, &video_url, description, "youtube_api");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::Videos.to_string();
            r.thumbnail = thumbnail;
            r.published_date = published_date;
            results.push(r);
        }

        info!(
            engine = "youtube_api",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
