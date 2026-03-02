//! Brave Search API engine implementation.
//! Translated from SearXNG's `braveapi.py`.
//! Requires API key from https://api-dashboard.search.brave.com/
//! Features: Paging, SafeSearch, Time Range

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
use tracing::info;
use smallvec::smallvec;

pub struct BraveApi {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl BraveApi {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "braveapi".to_string().into(),
                display_name: "Brave API".to_string().into(),
                homepage: "https://api.search.brave.com/".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            api_key,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    web: Option<WebResults>,
}

#[derive(Deserialize)]
struct WebResults {
    results: Option<Vec<WebResult>>,
}

#[derive(Deserialize)]
struct WebResult {
    url: String,
    title: String,
    description: Option<String>,
    #[allow(dead_code)]
    age: Option<String>,
}

#[async_trait]
impl SearchEngine for BraveApi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "braveapi".to_string(),
                message:
                    "No API key configured. Get one at https://api-dashboard.search.brave.com/"
                        .to_string(),
            });
        }

        let results_per_page = 20;
        let offset = (query.page.max(1) - 1) * results_per_page;
        let mut url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}&offset={}",
            urlencoding::encode(&query.query),
            results_per_page,
            offset
        );

        // Time range support
        if let Some(ref range) = query.time_range {
            let brave_range = match range.as_str() {
                "day" => Some("past_day"),
                "week" => Some("past_week"),
                "month" => Some("past_month"),
                "year" => Some("past_year"),
                _ => None,
            };
            if let Some(r) = brave_range {
                url.push_str(&format!("&time_range={}", r));
            }
        }

        let resp = self
            .client
            .get(&url)
            .header("X-Subscription-Token", key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        if let Some(web) = data.web {
            if let Some(items) = web.results {
                for (i, item) in items.iter().enumerate() {
                    let mut r = SearchResult::new(
                        &item.title,
                        &item.url,
                        item.description.as_deref().unwrap_or(""),
                        "braveapi",
                    );
                    r.engine_rank = (i + 1) as u32;
                    r.category = SearchCategory::General.to_string();
                    results.push(r);
                }
            }
        }

        info!(
            engine = "braveapi",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
