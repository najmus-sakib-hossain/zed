//! Piped (alternative YouTube frontend) video search — configurable instance URLs.
//! SearXNG equivalent: `piped.py`
//!
//! Piped consists of a backend (API) and frontend (UI). Both URLs must be
//! configured. The backend URL is used for API calls, the frontend URL
//! is used for constructing user-facing links.

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

pub struct Piped {
    client: Client,
    backend_url: String,
    frontend_url: String,
}

impl Piped {
    pub fn new(client: Client, backend_url: &str, frontend_url: &str) -> Self {
        Self {
            client,
            backend_url: backend_url.trim_end_matches('/').to_string(),
            frontend_url: frontend_url.trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Deserialize)]
struct PipedResponse {
    items: Vec<PipedItem>,
    #[allow(dead_code)]
    nextpage: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct PipedItem {
    url: Option<String>,
    title: Option<String>,
    #[serde(rename = "shortDescription")]
    short_description: Option<String>,
    thumbnail: Option<String>,
    views: Option<i64>,
    duration: Option<i64>,
    #[serde(rename = "uploaderName")]
    uploader_name: Option<String>,
}

#[async_trait]
impl SearchEngine for Piped {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "piped".to_string().into(),
            display_name: "Piped".to_string().into(),
            homepage: "https://piped.video".to_string().into(),
            categories: smallvec![SearchCategory::Videos],
            enabled: !self.backend_url.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.backend_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/search?q={}&filter=all",
            self.backend_url,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Piped: {e}")))?;

        let data: PipedResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Piped JSON: {e}")))?;

        let frontend = if self.frontend_url.is_empty() {
            "https://piped.video"
        } else {
            &self.frontend_url
        };

        let mut results = Vec::new();
        for (i, item) in data.items.iter().enumerate() {
            let path = match &item.url {
                Some(u) if !u.is_empty() => u,
                _ => continue,
            };
            let title = item.title.clone().unwrap_or_default();
            let video_url = format!("{}{}", frontend, path);
            let content = item.short_description.clone().unwrap_or_default();

            results.push(SearchResult {
                title,
                url: video_url,
                content,
                engine: "Piped".to_string(),
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
