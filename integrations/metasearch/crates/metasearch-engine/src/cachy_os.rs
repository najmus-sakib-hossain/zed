//! CachyOS package search engine.
//!
//! Queries the CachyOS package API for Arch-based package search results.

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

pub struct CachyOs {
    metadata: EngineMetadata,
    client: Client,
}

impl CachyOs {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "cachy_os".to_string().into(),
                display_name: "CachyOS".to_string().into(),
                homepage: "https://packages.cachyos.org".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    pkg_name: String,
    pkg_desc: Option<String>,
    pkg_version: Option<String>,
    pkg_arch: Option<String>,
    repo_name: Option<String>,
}

#[async_trait]
impl SearchEngine for CachyOs {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://packages.cachyos.org/api/search?search={}&page_size=15&current_page={}",
            urlencoding::encode(&query.query),
            page
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let api_resp: ApiResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        for pkg in api_resp.packages {
            let repo = pkg.repo_name.as_deref().unwrap_or("unknown");
            let arch = pkg.pkg_arch.as_deref().unwrap_or("x86_64");
            let version = pkg.pkg_version.as_deref().unwrap_or("");
            let desc = pkg.pkg_desc.as_deref().unwrap_or("");

            let title = format!("{} ({}) v{}", pkg.pkg_name, repo, version);
            let url = format!(
                "https://packages.cachyos.org/package/{}/{}/{}",
                repo, arch, pkg.pkg_name
            );
            let snippet = desc.to_string();

            let mut result = SearchResult::new(&title, &url, &snippet, "cachy_os");
            result.category = SearchCategory::IT.to_string();
            results.push(result);
        }

        Ok(results)
    }
}
