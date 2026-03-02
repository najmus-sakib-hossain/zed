//! IMDb engine — search movies/TV shows via IMDb suggestion API.
//! Translated from SearXNG `searx/engines/imdb.py`.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use std::collections::HashMap;
use smallvec::smallvec;

pub struct Imdb {
    metadata: EngineMetadata,
    client: Client,
}

impl Imdb {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "imdb".to_string().into(),
                display_name: "IMDb".to_string().into(),
                homepage: "https://www.imdb.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Imdb {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let q = query.query.replace(' ', "_").to_lowercase();
        let first_char = q.chars().next().unwrap_or('a');

        let url = format!(
            "https://v2.sg.media-imdb.com/suggestion/{}/{}.json",
            first_char, q,
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

        let category_map: HashMap<&str, &str> = [
            ("nm", "name"),
            ("tt", "title"),
            ("kw", "keyword"),
            ("co", "company"),
            ("ep", "episode"),
        ]
        .iter()
        .cloned()
        .collect();

        let mut results = Vec::new();

        if let Some(entries) = data["d"].as_array() {
            for (i, entry) in entries.iter().enumerate() {
                let entry_id = entry["id"].as_str().unwrap_or_default();
                let category_prefix = if entry_id.len() >= 2 {
                    &entry_id[..2]
                } else {
                    ""
                };
                let categ = match category_map.get(category_prefix) {
                    Some(c) => c,
                    None => continue,
                };

                let mut title = entry["l"].as_str().unwrap_or("Untitled").to_string();
                if let Some(q_val) = entry["q"].as_str() {
                    title = format!("{} ({})", title, q_val);
                }

                let mut content = String::new();
                if let Some(rank) = entry["rank"].as_u64() {
                    content.push_str(&format!("({}) ", rank));
                }
                if let Some(year) = entry["y"].as_u64() {
                    content.push_str(&format!("{} - ", year));
                }
                if let Some(stars) = entry["s"].as_str() {
                    content.push_str(stars);
                }

                let imdb_url = format!("https://imdb.com/{}/{}", categ, entry_id);

                let mut result = SearchResult::new(title, imdb_url, content, "imdb".to_string());
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::General.to_string();

                // Try to get thumbnail image
                if let Some(image_url) = entry["i"]["imageUrl"].as_str() {
                    result.thumbnail = Some(image_url.to_string());
                }

                results.push(result);
            }
        }

        Ok(results)
    }
}
