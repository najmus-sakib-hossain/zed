//! Openverse — Creative Commons image search
//!
//! Openverse (formerly Creative Commons search) provides a free JSON API
//! for searching openly licensed images. No API key required.
//!
//! Reference: <https://api.openverse.org/v1/>

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

pub struct Openverse {
    client: Client,
}

impl Openverse {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    results: Vec<ApiResult>,
}

#[derive(Debug, Deserialize)]
struct ApiResult {
    title: Option<String>,
    foreign_landing_url: Option<String>,
    url: Option<String>,
    creator: Option<String>,
}

#[async_trait]
impl SearchEngine for Openverse {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Openverse".to_string().into(),
            display_name: "Openverse".to_string().into(),
            homepage: "https://Openverse.com".to_string().into(),
            categories: smallvec![SearchCategory::Images],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://api.openverse.org/v1/images/?q={}&page={}&page_size=20&format=json",
            urlencoding::encode(&query.query),
            query.page
        );

        let resp = self.client.get(&url).send().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;
        let data: ApiResponse = resp.json().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;

        let results = data
            .results
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let title = item.title.unwrap_or_default();
                let url = item.foreign_landing_url?;
                let img_src = item.url.unwrap_or_default();
                let content = match &item.creator {
                    Some(c) => format!("By {} — Creative Commons", c),
                    None => "Creative Commons licensed image".to_string(),
                };
                let mut result = SearchResult::new(
                    title,
                    url,
                    content,
                    "openverse",
                );
                result.engine_rank = (i + 1) as u32;
                result.thumbnail = Some(img_src);
                Some(result)
            })
            .collect();

        Ok(results)
    }
}

