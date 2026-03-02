//! Library of Congress — image search via loc.gov JSON API.
//!
//! Searches the photos and prints collection by default.

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

pub struct Loc {
    metadata: EngineMetadata,
    client: Client,
}

impl Loc {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "loc".to_string().into(),
                display_name: "Library of Congress".to_string().into(),
                homepage: "https://www.loc.gov".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    #[serde(default)]
    results: Vec<LocResult>,
}

#[derive(Deserialize)]
struct LocResult {
    id: Option<String>,
    title: Option<String>,
    image_url: Option<Vec<String>>,
    #[serde(default)]
    description: Option<Vec<String>>,
}

#[async_trait]
impl SearchEngine for Loc {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;

        let url = format!(
            "https://www.loc.gov/photos/?q={}&sp={}&fo=json",
            urlencoding::encode(&query.query),
            page
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let api: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let results = api
            .results
            .into_iter()
            .enumerate()
            .filter_map(|(i, r)| {
                let title = r.title.unwrap_or_default();
                let link = r.id?;
                if title.is_empty() || link.is_empty() {
                    return None;
                }

                let snippet = r
                    .description
                    .and_then(|d| d.into_iter().next())
                    .unwrap_or_default();

                let thumbnail = r.image_url.and_then(|imgs| imgs.into_iter().next());

                let mut result =
                    SearchResult::new(title, link, snippet, self.metadata.name.clone());
                result.engine_rank = (i + 1) as u32;
                result.thumbnail = thumbnail;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
