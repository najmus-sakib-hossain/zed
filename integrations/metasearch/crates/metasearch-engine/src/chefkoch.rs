//! Chefkoch engine — search German recipes via Chefkoch JSON API.
//! Translated from SearXNG `searx/engines/chefkoch.py`.

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

pub struct Chefkoch {
    metadata: EngineMetadata,
    client: Client,
}

impl Chefkoch {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "chefkoch".to_string().into(),
                display_name: "Chefkoch".to_string().into(),
                homepage: "https://www.chefkoch.de".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.5,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Chefkoch {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let limit: u32 = 20;
        let offset = (page - 1) * limit;

        let url = format!(
            "https://api.chefkoch.de/v2/search-gateway/recipes?query={}&limit={}&offset={}",
            urlencoding::encode(&query.query),
            limit,
            offset,
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
                let recipe = &item["recipe"];

                // Skip premium recipes
                let is_premium = recipe["isPremium"].as_bool().unwrap_or(false);
                let is_plus = recipe["isPlus"].as_bool().unwrap_or(false);
                if is_premium || is_plus {
                    continue;
                }

                let title = recipe["title"].as_str().unwrap_or_default();
                let site_url = recipe["siteUrl"].as_str().unwrap_or_default();
                let difficulty = recipe["difficulty"].as_u64().unwrap_or(0);
                let prep_time = recipe["preparationTime"].as_u64().unwrap_or(0);
                let ingredient_count = recipe["ingredientCount"].as_u64().unwrap_or(0);

                let subtitle = recipe["subtitle"].as_str().unwrap_or_default();
                let mut parts = Vec::new();
                if !subtitle.is_empty() {
                    parts.push(subtitle.to_string());
                }
                parts.push(format!("Difficulty: {}/3", difficulty));
                parts.push(format!("Prep: {}min", prep_time));
                parts.push(format!("{} ingredients", ingredient_count));

                let snippet = parts.join(" | ");

                let thumbnail = recipe["previewImageUrlTemplate"]
                    .as_str()
                    .map(|t| t.replace("<format>", "crop-240x300"));

                let mut result = SearchResult::new(
                    title.to_string(),
                    site_url.to_string(),
                    snippet,
                    "chefkoch".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::General.to_string();
                result.thumbnail = thumbnail;
                results.push(result);
            }
        }

        Ok(results)
    }
}
