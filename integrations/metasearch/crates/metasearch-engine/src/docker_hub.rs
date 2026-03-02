//! Docker Hub engine — search container images via Docker Hub API.
//! Translated from SearXNG `searx/engines/docker_hub.py`.

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

const PAGE_SIZE: u32 = 10;

pub struct DockerHub {
    metadata: EngineMetadata,
    client: Client,
}

impl DockerHub {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "docker_hub".to_string().into(),
                display_name: "Docker Hub".to_string().into(),
                homepage: "https://hub.docker.com".to_string().into(),
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
impl SearchEngine for DockerHub {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let from = PAGE_SIZE * (page - 1);

        let url = format!(
            "https://hub.docker.com/api/search/v3/catalog/search?query={}&from={}&size={}",
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

        if let Some(items) = data["results"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let name = item["name"].as_str().unwrap_or_default();
                let slug = item["slug"].as_str().unwrap_or_default();
                let description = item["short_description"].as_str().unwrap_or("");
                let source = item["source"].as_str().unwrap_or("");
                let stars = item["star_count"].as_u64().unwrap_or(0);

                let is_official = source == "store" || source == "official";
                let prefix = if is_official { "/_/" } else { "/r/" };
                let page_url = format!("https://hub.docker.com{}{}", prefix, slug);

                let snippet = format!("{} — ⭐ {} stars", description, stars);

                let mut result = SearchResult::new(
                    name.to_string(),
                    page_url,
                    snippet,
                    "docker_hub".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
