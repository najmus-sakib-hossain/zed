//! Wallhaven — wallpaper search engine.
//!
//! Queries the Wallhaven JSON API for image results.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct Wallhaven {
    metadata: EngineMetadata,
    client: Client,
}

impl Wallhaven {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "wallhaven".to_string().into(),
                display_name: "Wallhaven".to_string().into(),
                homepage: "https://wallhaven.cc".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct WallhavenThumbs {
    small: Option<String>,
}

#[derive(Deserialize)]
struct WallhavenItem {
    url: Option<String>,
    category: Option<String>,
    purity: Option<String>,
    resolution: Option<String>,
    thumbs: Option<WallhavenThumbs>,
}

#[derive(Deserialize)]
struct WallhavenResponse {
    data: Vec<WallhavenItem>,
}

#[async_trait]
impl SearchEngine for Wallhaven {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = format!(
            "https://wallhaven.cc/api/v1/search?q={}&page={}&purity=100",
            urlencoding::encode(&query.query),
            page
        );

        let resp: WallhavenResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let results = resp
            .data
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let page_url = item.url.unwrap_or_default();
                let snippet = format!(
                    "{} / {}",
                    item.category.unwrap_or_default(),
                    item.purity.unwrap_or_default()
                );
                let mut r = SearchResult::new(
                    item.resolution.unwrap_or_default(),
                    page_url,
                    snippet,
                    self.metadata.name.clone(),
                );
                r.engine_rank = (i + 1) as u32;
                r.thumbnail = item.thumbs.and_then(|t| t.small);
                r
            })
            .collect();

        Ok(results)
    }
}
