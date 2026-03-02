//! Arch Linux engine — searches Arch Linux packages via the JSON API.
//! Falls back to `archlinux.org/packages/search/json/` which is bot-free.
//!
//! The original SearXNG engine targets the wiki; the wiki now requires
//! JavaScript for bot verification, so we switched to the packages API.

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

pub struct ArchLinux {
    metadata: EngineMetadata,
    client: Client,
}

impl ArchLinux {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "archlinux".to_string().into(),
                display_name: "Arch Linux".to_string().into(),
                homepage: "https://archlinux.org".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for ArchLinux {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;

        // Arch Linux packages JSON API — no bot challenges
        let url = format!(
            "https://archlinux.org/packages/search/json/?q={}&limit=20&page={}",
            urlencoding::encode(&query.query),
            page,
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "Mozilla/5.0 (compatible; metasearch/1.0)")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        if let Some(packages) = data["results"].as_array() {
            for (i, pkg) in packages.iter().enumerate() {
                let name = pkg["pkgname"].as_str().unwrap_or_default();
                let desc = pkg["pkgdesc"].as_str().unwrap_or("");
                let version = pkg["pkgver"].as_str().unwrap_or("?");
                let repo = pkg["repo"].as_str().unwrap_or("extra");
                let arch = pkg["arch"].as_str().unwrap_or("x86_64");

                if name.is_empty() {
                    continue;
                }

                let pkg_url = format!(
                    "https://archlinux.org/packages/{}/{}/{}/",
                    repo, arch, name
                );
                let snippet = format!("{} — {} [{}]", desc, version, repo);

                let mut result = SearchResult::new(
                    name.to_string(),
                    pkg_url,
                    snippet,
                    "archlinux".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
