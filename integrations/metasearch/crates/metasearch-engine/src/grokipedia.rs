//! Grokipedia — AI-powered encyclopedia search.
//!
//! Uses the Grokipedia full-text-search JSON API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct Grokipedia {
    metadata: EngineMetadata,
    client: Client,
}

impl Grokipedia {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "grokipedia".to_string().into(),
                display_name: "Grokipedia".to_string().into(),
                homepage: "https://grokipedia.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    #[serde(default)]
    results: Vec<ApiResult>,
}

#[derive(Deserialize)]
struct ApiResult {
    slug: Option<String>,
    title: Option<String>,
    snippet: Option<String>,
}

#[async_trait]
impl SearchEngine for Grokipedia {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let offset = (page.saturating_sub(1)) * 10;

        let resp = match self
            .client
            .get("https://grokipedia.com/api/full-text-search")
            .timeout(std::time::Duration::from_secs(6))
            .query(&[
                ("q", query.query.as_str()),
                ("offset", &offset.to_string()),
                ("limit", "10"),
            ])
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let api: ApiResponse = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let results = api
            .results
            .into_iter()
            .enumerate()
            .filter_map(|(i, r)| {
                let title = r.title.unwrap_or_default();
                let slug = r.slug?;
                if title.is_empty() {
                    return None;
                }
                let url = format!("https://grokipedia.com/wiki/{slug}");
                let snippet = r.snippet.unwrap_or_default();
                let mut result = SearchResult::new(title, url, snippet, self.metadata.name.clone());
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
