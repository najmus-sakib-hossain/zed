//! Marginalia Search engine implementation.
//! Translated from SearXNG's `marginalia.py`.
//! Requires API key from https://about.marginalia-search.com/article/api/
//! Independent open-source search engine from Sweden.

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

pub struct Marginalia {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl Marginalia {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "marginalia".to_string().into(),
                display_name: "Marginalia".to_string().into(),
                homepage: "https://marginalia.nu".to_string().into(),
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
struct MarginaliaResponse {
    results: Option<Vec<MarginaliaResult>>,
}

#[derive(Deserialize)]
struct MarginaliaResult {
    url: String,
    title: String,
    description: Option<String>,
}

#[async_trait]
impl SearchEngine for Marginalia {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let key = self.api_key.as_deref().unwrap_or("");
        if key.is_empty() {
            return Err(MetasearchError::EngineError {
                engine: "marginalia".to_string(),
                message: "No API key configured. Get one at https://about.marginalia-search.com/article/api/".to_string(),
            });
        }

        let url = format!(
            "https://api2.marginalia-search.com/search?count=20&query={}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("API-Key", key)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: MarginaliaResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        if let Some(items) = data.results {
            for (i, item) in items.iter().enumerate() {
                let mut r = SearchResult::new(
                    &item.title,
                    &item.url,
                    item.description.as_deref().unwrap_or(""),
                    "marginalia",
                );
                r.engine_rank = (i + 1) as u32;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        info!(
            engine = "marginalia",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
