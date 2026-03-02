//! crates.io engine — search Rust crates via crates.io API.
//! Translated from SearXNG `searx/engines/crates.py`.

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

pub struct CratesIo {
    metadata: EngineMetadata,
    client: Client,
}

impl CratesIo {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "crates_io".to_string().into(),
                display_name: "crates.io".to_string().into(),
                homepage: "https://crates.io".to_string().into(),
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
impl SearchEngine for CratesIo {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;

        let url = format!(
            "https://crates.io/api/v1/crates?q={}&page={}&per_page={}",
            urlencoding::encode(&query.query),
            page,
            PAGE_SIZE,
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header(
                "User-Agent",
                "metasearch-engine/1.0 (https://github.com/najmus-sakib-hossain/metasearch)",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(crates) = data["crates"].as_array() {
            for (i, krate) in crates.iter().enumerate() {
                let name = krate["name"].as_str().unwrap_or_default();
                let description = krate["description"].as_str().unwrap_or("");
                let version = krate["newest_version"]
                    .as_str()
                    .or_else(|| krate["max_version"].as_str())
                    .unwrap_or("?");
                let downloads = krate["downloads"].as_u64().unwrap_or(0);

                let crate_url = format!("https://crates.io/crates/{}", name);
                let snippet = format!("v{} — {} — {} downloads", version, description, downloads,);

                let mut result = SearchResult::new(
                    name.to_string(),
                    crate_url,
                    snippet,
                    "crates_io".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
