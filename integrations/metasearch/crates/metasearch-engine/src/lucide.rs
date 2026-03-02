//! Lucide — open-source icon search.
//!
//! Fetches the Lucide tags.json and filters icons client-side.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use std::collections::HashMap;
use smallvec::smallvec;

pub struct Lucide {
    metadata: EngineMetadata,
    client: Client,
}

impl Lucide {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "lucide".to_string().into(),
                display_name: "Lucide".to_string().into(),
                homepage: "https://lucide.dev".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Lucide {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = "https://cdn.jsdelivr.net/npm/lucide-static/tags.json";

        let tags: HashMap<String, Vec<String>> = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
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

        for (icon_name, icon_tags) in &tags {
            let matches = query_parts.iter().any(|part| {
                icon_name.contains(part.as_str())
                    || icon_tags.iter().any(|tag| tag.contains(part.as_str()))
            });

            if matches {
                rank += 1;
                let img_src = format!(
                    "https://cdn.jsdelivr.net/npm/lucide-static/icons/{}.svg",
                    icon_name
                );
                let snippet = icon_tags.join(", ");
                let mut r = SearchResult::new(
                    icon_name.clone(),
                    img_src.clone(),
                    snippet,
                    self.metadata.name.clone(),
                );
                r.engine_rank = rank;
                r.thumbnail = Some(img_src);
                results.push(r);
            }
        }

        Ok(results)
    }
}
