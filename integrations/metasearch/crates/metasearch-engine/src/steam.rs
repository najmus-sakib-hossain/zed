//! Steam Store engine — search games via Steam Store API.

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

pub struct SteamStore {
    metadata: EngineMetadata,
    client: Client,
}

impl SteamStore {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "steam".to_string().into(),
                display_name: "Steam".to_string().into(),
                homepage: "https://store.steampowered.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for SteamStore {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://store.steampowered.com/api/storesearch/?term={}&cc=us&l=en",
            urlencoding::encode(&query.query),
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

        if let Some(items) = data["items"].as_array() {
            for (i, item) in items.iter().enumerate() {
                let name = item["name"].as_str().unwrap_or_default();
                let id = item["id"].as_u64().unwrap_or(0);
                let tiny_image = item["tiny_image"].as_str();

                let item_url = format!("https://store.steampowered.com/app/{}", id);

                // Build content from price and platform info
                let mut content_parts = Vec::new();

                if let Some(price) = item.get("price") {
                    let final_cents = price["final"].as_u64().unwrap_or(0);
                    let currency = price["currency"].as_str().unwrap_or("USD");
                    if final_cents == 0 {
                        content_parts.push("Free".to_string());
                    } else {
                        let dollars = final_cents as f64 / 100.0;
                        content_parts.push(format!("{:.2} {}", dollars, currency));
                    }
                }

                if let Some(platforms) = item.get("platforms") {
                    let mut plats = Vec::new();
                    if platforms["windows"].as_bool().unwrap_or(false) {
                        plats.push("Windows");
                    }
                    if platforms["mac"].as_bool().unwrap_or(false) {
                        plats.push("Mac");
                    }
                    if platforms["linux"].as_bool().unwrap_or(false) {
                        plats.push("Linux");
                    }
                    if !plats.is_empty() {
                        content_parts.push(plats.join(", "));
                    }
                }

                let content = content_parts.join(" — ");

                let mut result = SearchResult::new(
                    name.to_string(),
                    item_url,
                    content,
                    "steam".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::General.to_string();
                if let Some(thumb) = tiny_image {
                    result.thumbnail = Some(thumb.to_string());
                }
                results.push(result);
            }
        }

        Ok(results)
    }
}
