//! Devicons engine — search developer technology icons.
//! Translated from SearXNG `searx/engines/devicons.py`.

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

const CDN_BASE: &str = "https://cdn.jsdelivr.net/gh/devicons/devicon@latest";

pub struct Devicons {
    metadata: EngineMetadata,
    client: Client,
}

impl Devicons {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "devicons".to_string().into(),
                display_name: "Devicons".to_string().into(),
                homepage: "https://devicon.dev".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Devicons {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!("{}/devicon.json", CDN_BASE);

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

        let query_parts: Vec<String> = query
            .query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut results = Vec::new();
        let mut rank = 0u32;

        for item in &data {
            let name = item["name"].as_str().unwrap_or_default();

            // Check if any query part matches the name, altnames, or tags
            let altnames: Vec<&str> = item["altnames"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let tags: Vec<&str> = item["tags"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let matches = query_parts.iter().any(|part| {
                name.contains(part.as_str())
                    || altnames
                        .iter()
                        .any(|a| a.to_lowercase().contains(part.as_str()))
                    || tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(part.as_str()))
            });

            if !matches {
                continue;
            }

            // Get SVG variants
            let svg_variants: Vec<&str> = item["versions"]["svg"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let color = item["color"].as_str().unwrap_or("#000");

            for variant in &svg_variants {
                let img_src = format!("{}/icons/{}/{}-{}.svg", CDN_BASE, name, name, variant,);

                rank += 1;
                let mut result = SearchResult::new(
                    format!("{} ({})", name, variant),
                    img_src.clone(),
                    format!("Base color: {}", color),
                    "devicons".to_string(),
                );
                result.engine_rank = rank;
                result.category = SearchCategory::Images.to_string();
                result.thumbnail = Some(img_src);
                results.push(result);
            }
        }

        Ok(results)
    }
}
