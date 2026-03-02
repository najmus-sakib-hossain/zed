//! GitHub engine — search repositories via GitHub REST API.
//! Translated from SearXNG `searx/engines/github.py`.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct GitHub {
    metadata: EngineMetadata,
    client: Client,
}

impl GitHub {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "github".to_string().into(),
                display_name: "GitHub".to_string().into(),
                homepage: "https://github.com".to_string().into(),
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
impl SearchEngine for GitHub {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://api.github.com/search/repositories?sort=stars&order=desc&q={}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.preview.text-match+json")
            .header("User-Agent", "metasearch-engine/1.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(items) = data["items"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let full_name = item["full_name"].as_str().unwrap_or_default();
                let html_url = item["html_url"].as_str().unwrap_or_default();
                let description = item["description"].as_str().unwrap_or("");
                let language = item["language"].as_str().unwrap_or("");
                let stars = item["stargazers_count"].as_u64().unwrap_or(0);

                let snippet = if language.is_empty() {
                    format!("{} — ⭐ {}", description, stars)
                } else {
                    format!("{} — {} — ⭐ {}", description, language, stars)
                };

                let mut result = SearchResult::new(
                    full_name.to_string(),
                    html_url.to_string(),
                    snippet,
                    "github".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                result.thumbnail = item["owner"]["avatar_url"].as_str().map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
