//! Repology engine — search packages across repositories via Repology API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Repology {
    metadata: EngineMetadata,
    client: Client,
}

impl Repology {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "repology".to_string().into(),
                display_name: "Repology".to_string().into(),
                homepage: "https://repology.org".to_string().into(),
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
impl SearchEngine for Repology {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://repology.org/api/v1/projects/?search={}",
            urlencoding::encode(&query.query),
        );

        let resp = match self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "metasearch-engine/1.0 (https://github.com/najmus-sakib-hossain/metasearch)",
            )
            .timeout(std::time::Duration::from_secs(6))
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

        let projects = match data.as_object() {
            Some(obj) => obj,
            None => return Ok(results),
        };

        for (i, (pkg_name, repos)) in projects.iter().enumerate() {
            let repos_arr = match repos.as_array() {
                Some(arr) => arr,
                None => continue,
            };

            // Find the "newest" version
            let mut version = String::new();
            let mut repo_names = Vec::new();

            for repo in repos_arr {
                let repo_name = repo["repo"].as_str().unwrap_or_default();
                if !repo_name.is_empty() && !repo_names.contains(&repo_name) {
                    repo_names.push(repo_name);
                }

                let status = repo["status"].as_str().unwrap_or_default();
                if status == "newest" {
                    if let Some(v) = repo["version"].as_str() {
                        version = v.to_string();
                    }
                }
            }

            // Fall back to first version if no "newest" found
            if version.is_empty() {
                if let Some(first) = repos_arr.first() {
                    version = first["version"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();
                }
            }

            let pkg_url = format!("https://repology.org/project/{}/versions", pkg_name);

            let content = if repo_names.is_empty() {
                format!("v{}", version)
            } else {
                let repos_display: Vec<&str> = if repo_names.len() > 5 {
                    repo_names[..5].to_vec()
                } else {
                    repo_names.clone()
                };
                let suffix = if repo_names.len() > 5 {
                    format!(" (+{} more)", repo_names.len() - 5)
                } else {
                    String::new()
                };
                format!("v{} — {}{}", version, repos_display.join(", "), suffix)
            };

            let mut result = SearchResult::new(
                pkg_name.clone(),
                pkg_url,
                content,
                "repology".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::IT.to_string();
            results.push(result);
        }

        Ok(results)
    }
}
