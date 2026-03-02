//! Moviepilot — German movie database (JSON API)
//!
//! Searches moviepilot.de for movies via their internal suggest API.

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

const BASE_URL: &str = "https://www.moviepilot.de";

pub struct Moviepilot {
    client: Client,
}

impl Moviepilot {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct MoviepilotResult {
    title: Option<String>,
    url: Option<String>,
    image: Option<String>,
    class: Option<String>,
    info: Option<String>,
    more: Option<String>,
}

#[async_trait]
impl SearchEngine for Moviepilot {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "moviepilot".to_string().into(),
            display_name: "moviepilot".to_string().into(),
            homepage: "https://moviepilot.com".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "{}/api/search?q={}&page={}&type=suggest",
            BASE_URL,
            urlencoding::encode(&query.query),
            query.page
        );

        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                MetasearchError::Engine(format!("Moviepilot request failed: {}", e))
            })?;

        let items: Vec<MoviepilotResult> = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Moviepilot parse failed: {}", e)))?;

        let results = items
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let title = item.title?;
                let url = item.url?;
                let parts: Vec<&str> = [
                    item.class.as_deref(),
                    item.info.as_deref(),
                    item.more.as_deref(),
                ]
                .iter()
                .filter_map(|p| *p)
                .filter(|p| !p.is_empty())
                .collect();
                let snippet = parts.join(", ");

                let mut result = SearchResult::new(&title, &url, &snippet, "moviepilot");
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::General.to_string();
                result.thumbnail = item.image;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
