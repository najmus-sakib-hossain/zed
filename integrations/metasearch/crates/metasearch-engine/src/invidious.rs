//! Invidious (alternative YouTube frontend) video search — configurable instance URL.
//! SearXNG equivalent: `invidious.py`
//!
//! Invidious provides a JSON API for searching YouTube videos without
//! Google tracking. You must configure a `base_url` pointing to a local
//! or public Invidious instance.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct Invidious {
    client: Client,
    base_url: String,
}

impl Invidious {
    pub fn new(client: Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Deserialize)]
struct InvidiousResult {
    #[serde(rename = "type")]
    result_type: Option<String>,
    #[serde(rename = "videoId")]
    video_id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    author: Option<String>,
    #[serde(rename = "viewCount", default)]
    #[allow(dead_code)]
    view_count: u64,
    #[serde(rename = "lengthSeconds", default)]
    #[allow(dead_code)]
    length_seconds: u64,
}

#[async_trait]
impl SearchEngine for Invidious {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Invidious".to_string().into(),
            display_name: "Invidious".to_string().into(),
            homepage: "https://invidious.io".to_string().into(),
            categories: smallvec![SearchCategory::Videos],
            enabled: !self.base_url.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/api/v1/search?q={}&page={}",
            self.base_url,
            urlencoding::encode(&query.query),
            query.page,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Invidious: {e}")))?;

        let items: Vec<InvidiousResult> = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Invidious JSON: {e}")))?;

        let mut results = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if item.result_type.as_deref() != Some("video") {
                continue;
            }
            let video_id = match &item.video_id {
                Some(id) if !id.is_empty() => id,
                _ => continue,
            };
            let title = item.title.clone().unwrap_or_default();
            let video_url = format!("{}/watch?v={}", self.base_url, video_id);

            let mut content_parts = Vec::new();
            if let Some(ref desc) = item.description {
                if !desc.is_empty() {
                    content_parts.push(desc.clone());
                }
            }
            if let Some(ref author) = item.author {
                if !author.is_empty() {
                    content_parts.push(format!("by {}", author));
                }
            }

            results.push(SearchResult {
                title,
                url: video_url,
                content: content_parts.join(" — "),
                engine: "Invidious".to_string(),
                engine_rank: (i + 1) as u32,
                score: 0.0,
                thumbnail: None,
                published_date: None,
                category: String::new(),
                metadata: serde_json::Value::Null,
            });
        }
        Ok(results)
    }
}
