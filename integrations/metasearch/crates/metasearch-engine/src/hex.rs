//! Hex.pm engine — search Elixir/Erlang packages.
//! Translated from SearXNG `searx/engines/hex.py`.

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

pub struct Hex {
    metadata: EngineMetadata,
    client: Client,
}

impl Hex {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "hex".to_string().into(),
                display_name: "Hex.pm".to_string().into(),
                homepage: "https://hex.pm".to_string().into(),
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
impl SearchEngine for Hex {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://hex.pm/api/packages?page={}&per_page=10&sort=recent_downloads&search={}",
            page,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        for (i, pkg) in data.iter().enumerate() {
            let name = pkg["name"].as_str().unwrap_or_default();
            let html_url = pkg["html_url"].as_str().unwrap_or_default();
            let meta = &pkg["meta"];
            let description = meta["description"].as_str().unwrap_or_default();
            let version = meta["latest_version"].as_str().unwrap_or("?");

            let maintainers: Vec<&str> = meta["maintainers"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let licenses: Vec<&str> = meta["licenses"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let snippet = format!(
                "v{} | {} | {}",
                version,
                maintainers.join(", "),
                licenses.join(", "),
            );

            let mut result = SearchResult::new(
                name.to_string(),
                html_url.to_string(),
                format!("{} — {}", snippet, description),
                "hex".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::IT.to_string();

            // Parse updated_at as published_date
            if let Some(date_str) = pkg["updated_at"].as_str() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
                    result.published_date = Some(dt.with_timezone(&chrono::Utc));
                }
            }

            results.push(result);
        }

        Ok(results)
    }
}
