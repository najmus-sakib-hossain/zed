//! MediathekViewWeb — German public broadcaster media library search.
//!
//! Uses the MediathekViewWeb JSON POST API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde_json::json;
use smallvec::smallvec;

pub struct MediathekViewWeb {
    metadata: EngineMetadata,
    client: Client,
}

impl MediathekViewWeb {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "mediathekviewweb".to_string().into(),
                display_name: "MediathekViewWeb".to_string().into(),
                homepage: "https://mediathekviewweb.de".to_string().into(),
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
impl SearchEngine for MediathekViewWeb {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let offset = (page.saturating_sub(1)) * 10;

        let body = json!({
            "queries": [{
                "fields": ["title", "topic"],
                "query": query.query
            }],
            "sortBy": "timestamp",
            "sortOrder": "desc",
            "future": true,
            "offset": offset,
            "size": 10
        });

        let resp = self
            .client
            .post("https://mediathekviewweb.de/api/query")
            .header("Content-Type", "text/plain")
            .body(serde_json::to_string(&body).unwrap_or_default())
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(items) = json
            .get("result")
            .and_then(|r| r.get("results"))
            .and_then(|v| v.as_array())
        {
            for (i, item) in items.iter().enumerate() {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let channel = item
                    .get("channel")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let description = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let url_video = item
                    .get("url_video_hd")
                    .or_else(|| item.get("url_video"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .replace("http://", "https://");
                let duration = item.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);

                if url_video.is_empty() {
                    continue;
                }

                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let seconds = duration % 60;
                let dur_str = if hours > 0 {
                    format!("{hours}:{minutes:02}:{seconds:02}")
                } else {
                    format!("{minutes}:{seconds:02}")
                };

                let display_title = format!("{channel}: {title} ({dur_str})");

                let mut result = SearchResult::new(
                    display_title,
                    url_video,
                    description.to_string(),
                    self.metadata.name.clone(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Videos.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
