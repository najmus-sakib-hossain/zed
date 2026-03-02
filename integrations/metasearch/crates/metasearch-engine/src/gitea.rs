//! Gitea / Forgejo — search repositories on Gitea-based instances.
//!
//! Uses the Gitea REST API v1 (JSON).

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Gitea {
    metadata: EngineMetadata,
    client: Client,
}

impl Gitea {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "gitea".to_string().into(),
                display_name: "Gitea".to_string().into(),
                homepage: "https://gitea.com".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Gitea {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;

        let resp = self
            .client
            .get("https://gitea.com/api/v1/repos/search")
            .query(&[
                ("q", query.query.as_str()),
                ("limit", "10"),
                ("sort", "updated"),
                ("order", "desc"),
                ("page", &page.to_string()),
            ])
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
            for (i, item) in data.iter().enumerate() {
                let full_name = item
                    .get("full_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let html_url = item
                    .get("html_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let description = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let language = item
                    .get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let stars = item
                    .get("stars_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                if html_url.is_empty() {
                    continue;
                }

                let snippet = if language.is_empty() {
                    format!("{description} — ⭐ {stars}")
                } else {
                    format!("{language} — {description} — ⭐ {stars}")
                };

                let mut result = SearchResult::new(
                    full_name.to_string(),
                    html_url.to_string(),
                    snippet,
                    self.metadata.name.clone(),
                );
                result.engine_rank = (i + 1) as u32;
                results.push(result);
            }
        }

        Ok(results)
    }
}
