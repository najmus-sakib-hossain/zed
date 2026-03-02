//! npm engine — search JavaScript packages via npms.io API.
//! Translated from SearXNG `searx/engines/npm.py`.

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

const PAGE_SIZE: u32 = 25;

pub struct Npm {
    metadata: EngineMetadata,
    client: Client,
}

impl Npm {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "npm".to_string().into(),
                display_name: "npm".to_string().into(),
                homepage: "https://www.npmjs.com".to_string().into(),
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
impl SearchEngine for Npm {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let from = (page - 1) * PAGE_SIZE;

        let url = format!(
            "https://api.npms.io/v2/search?q={}&from={}&size={}",
            urlencoding::encode(&query.query),
            from,
            PAGE_SIZE,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(entries) = data["results"].as_array() {
            for (i, entry) in entries.iter().enumerate() {
                let package = &entry["package"];
                let name = package["name"].as_str().unwrap_or_default();
                let description = package["description"].as_str().unwrap_or("");
                let version = package["version"].as_str().unwrap_or("");
                let npm_url = package["links"]["npm"].as_str().unwrap_or_default();
                let author = package["author"]["name"].as_str().unwrap_or("");

                let snippet = format!("v{} — {} — by {}", version, description, author,);

                let mut result = SearchResult::new(
                    name.to_string(),
                    npm_url.to_string(),
                    snippet,
                    "npm".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
