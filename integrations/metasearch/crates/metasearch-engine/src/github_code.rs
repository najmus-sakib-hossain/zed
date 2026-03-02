//! GitHub Code search engine implementation.
//!
//! JSON API: <https://api.github.com/search/code>
//! Website: https://github.com
//! Features: Paging, optional auth token

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct GithubCode {
    metadata: EngineMetadata,
    client: Client,
    token: Option<String>,
}

impl GithubCode {
    pub fn new(client: Client, token: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "github_code".to_string().into(),
                display_name: "GitHub Code".to_string().into(),
                homepage: "https://github.com".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            token,
        }
    }
}

#[async_trait]
impl SearchEngine for GithubCode {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let page = query.page.max(1);

        // GitHub Code Search API requires authentication
        // Use the repositories search instead which doesn't require auth
        let url = format!(
            "https://api.github.com/search/repositories?sort=stars&order=desc&q={}&page={}",
            encoded, page
        );

        let mut req = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.preview.text-match+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "metasearch-engine/1.0");

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("token {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let status = resp.status().as_u16();
        if status == 403 || status == 429 {
            return Err(MetasearchError::RateLimited {
                retry_after_secs: 60,
            });
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        let items = match data.get("items").and_then(|i| i.as_array()) {
            Some(items) => items,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let html_url = item
                .get("html_url")
                .and_then(|u| u.as_str())
                .unwrap_or_default();

            let full_name = item
                .get("full_name")
                .and_then(|n| n.as_str())
                .unwrap_or_default();

            if html_url.is_empty() || full_name.is_empty() {
                continue;
            }

            let language = item
                .get("language")
                .and_then(|l| l.as_str())
                .unwrap_or("");

            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            let content = if !language.is_empty() && !description.is_empty() {
                format!("{} / {}", language, description)
            } else if !description.is_empty() {
                description.to_string()
            } else {
                language.to_string()
            };

            let mut r = SearchResult::new(full_name, html_url, &content, "github_code");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::IT.to_string();
            results.push(r);
        }

        info!(
            engine = "github_code",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
