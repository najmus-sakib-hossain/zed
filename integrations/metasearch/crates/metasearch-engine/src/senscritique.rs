//! SensCritique — search movies, series, games, etc. via GraphQL API.
//!
//! Reference: <https://www.senscritique.com>

use async_trait::async_trait;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};

const GRAPHQL_URL: &str = "https://apollo.senscritique.com/";

const GRAPHQL_QUERY: &str = r#"query SearchProductExplorer($query: String!, $offset: Int, $limit: Int) { searchProductExplorer(query: $query, offset: $offset, limit: $limit) { items { title yearOfProduction originalTitle url medias { picture } category directors { name } countries { name } genresInfos { label } duration rating stats { ratingCount } } } }"#;

pub struct SensCritique {
    metadata: EngineMetadata,
    client: Client,
}

impl SensCritique {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "senscritique".to_string().into(),
                display_name: "SensCritique".to_string().into(),
                homepage: "https://www.senscritique.com".to_string().into(),
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
impl SearchEngine for SensCritique {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let limit = 16i64;
        let offset = (query.page.saturating_sub(1) as i64) * limit;

        let body = serde_json::json!({
            "query": GRAPHQL_QUERY,
            "variables": {
                "query": query.query,
                "offset": offset,
                "limit": limit
            }
        });

        let resp = match self
            .client
            .post(GRAPHQL_URL)
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0")
            .timeout(std::time::Duration::from_secs(6))
            .json(&body)
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

        let items = match data
            .pointer("/data/searchProductExplorer/items")
            .and_then(|v| v.as_array())
        {
            Some(arr) => arr,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let title_raw = item["title"].as_str().unwrap_or_default();
            if title_raw.is_empty() {
                continue;
            }

            let year = item["yearOfProduction"]
                .as_u64()
                .map(|y| y.to_string())
                .unwrap_or_default();

            let title = if year.is_empty() {
                title_raw.to_string()
            } else {
                format!("{} ({})", title_raw, year)
            };

            let url = item["url"].as_str().unwrap_or_default();
            if url.is_empty() {
                continue;
            }

            // Build content from category, directors, countries, rating
            let mut parts = Vec::new();

            if let Some(cat) = item["category"].as_str() {
                if !cat.is_empty() {
                    parts.push(cat.to_string());
                }
            }

            if let Some(directors) = item["directors"].as_array() {
                let names: Vec<&str> = directors
                    .iter()
                    .filter_map(|d| d["name"].as_str())
                    .collect();
                if !names.is_empty() {
                    parts.push(format!("Director: {}", names.join(", ")));
                }
            }

            if let Some(countries) = item["countries"].as_array() {
                let names: Vec<&str> = countries
                    .iter()
                    .filter_map(|c| c["name"].as_str())
                    .collect();
                if !names.is_empty() {
                    parts.push(names.join(", "));
                }
            }

            if let Some(rating) = item["rating"].as_f64() {
                parts.push(format!("Rating: {:.1}", rating));
            }

            let content = parts.join(" — ");

            let thumbnail = item
                .pointer("/medias/picture")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let mut result = SearchResult::new(
                title,
                url.to_string(),
                content,
                self.metadata.name.clone(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::General.to_string();
            if !thumbnail.is_empty() {
                result.thumbnail = Some(thumbnail.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}
