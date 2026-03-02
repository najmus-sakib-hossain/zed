//! Chinaso — Chinese national search engine.
//!
//! Queries the Chinaso JSON API for web search results.
//!
//! Reference: <https://www.chinaso.com>

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::MetasearchError;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const BASE_URL: &str = "https://www.chinaso.com";

pub struct Chinaso {
    metadata: EngineMetadata,
    client: Client,
}

impl Chinaso {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "chinaso".to_string().into(),
                display_name: "Chinaso".to_string().into(),
                homepage: BASE_URL.to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    data: Option<ApiData>,
}

#[derive(Deserialize)]
struct ApiData {
    #[serde(default)]
    list: Vec<ApiItem>,
}

#[derive(Deserialize)]
struct ApiItem {
    title: Option<String>,
    url: Option<String>,
    snippet: Option<String>,
}

#[async_trait]
impl SearchEngine for Chinaso {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "{}/v5/general/v2/search?q={}&pn={}&ps=10",
            BASE_URL,
            urlencoding::encode(&query.query),
            query.page
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let api: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let items = api.data.map(|d| d.list).unwrap_or_default();

        let results = items
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let title = item.title?;
                let url = item.url?;
                let clean_title = html_escape::decode_html_entities(&title).to_string();
                let snippet = item
                    .snippet
                    .map(|s| html_escape::decode_html_entities(&s).to_string())
                    .unwrap_or_default();
                let mut result = SearchResult::new(clean_title, url, snippet, "chinaso");
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
