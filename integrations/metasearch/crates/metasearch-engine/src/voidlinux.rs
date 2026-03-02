//! Void Linux — Void Linux package search via xq-api JSON API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct VoidLinux {
    metadata: EngineMetadata,
    client: Client,
}

impl VoidLinux {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "voidlinux".to_string().into(),
                display_name: "Void Linux".to_string().into(),
                homepage: "https://voidlinux.org".to_string().into(),
                categories: smallvec![SearchCategory::IT],
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
    data: Option<Vec<PackageItem>>,
}

#[derive(Deserialize)]
struct PackageItem {
    name: Option<String>,
    short_desc: Option<String>,
    version: Option<String>,
    repository: Option<String>,
}

#[async_trait]
impl SearchEngine for VoidLinux {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://xq-api.voidlinux.org/v1/query/x86_64?q={}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Void Linux request failed: {e}")))?;

        let api: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Void Linux parse failed: {e}")))?;

        let results = api
            .data
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, pkg)| {
                let name = pkg.name?;
                let version = pkg.version.unwrap_or_default();
                let repo = pkg.repository.unwrap_or_else(|| "current".to_string());
                let title = format!("{} {}", name, version);
                let result_url = format!(
                    "https://voidlinux.org/packages/?arch=x86_64&q={}",
                    urlencoding::encode(&name)
                );
                let snippet = format!("[{}] {}", repo, pkg.short_desc.unwrap_or_default());
                let mut result = SearchResult::new(
                    title,
                    result_url,
                    snippet,
                    "voidlinux",
                );
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
