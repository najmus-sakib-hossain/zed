//! OpenStreetMap — Nominatim geocoding search (JSON API)
//!
//! Queries the Nominatim API: `https://nominatim.openstreetmap.org/search`
//! Returns map/location results.
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/openstreetmap.py>

use async_trait::async_trait;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct OpenStreetMap {
    client: Client,
}

impl OpenStreetMap {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for OpenStreetMap {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "OpenStreetMap".to_string().into(),
            display_name: "OpenStreetMap".to_string().into(),
            homepage: "https://nominatim.openstreetmap.org".to_string().into(),
            categories: smallvec![SearchCategory::Maps],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://nominatim.openstreetmap.org/search?q={}&format=jsonv2&addressdetails=1&extratags=1&dedupe=1&limit=10",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "metasearch/1.0")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("OpenStreetMap request error: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("OpenStreetMap parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(items) = json.as_array() {
            for (i, item) in items.iter().enumerate() {
                let display_name = item
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let osm_type = item
                    .get("osm_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("node");
                let osm_id = item.get("osm_id").and_then(|v| v.as_u64()).unwrap_or(0);
                let lat = item.get("lat").and_then(|v| v.as_str()).unwrap_or("0");
                let lon = item.get("lon").and_then(|v| v.as_str()).unwrap_or("0");
                let category = item
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let r_type = item
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let osm_url = if osm_id > 0 {
                    format!("https://openstreetmap.org/{}/{}", osm_type, osm_id)
                } else {
                    format!(
                        "https://www.openstreetmap.org/?mlat={}&mlon={}&zoom=12&layers=M",
                        lat, lon
                    )
                };

                let content = if !category.is_empty() && !r_type.is_empty() {
                    format!("{} ({}) — lat: {}, lon: {}", category, r_type, lat, lon)
                } else {
                    format!("lat: {}, lon: {}", lat, lon)
                };

                results.push(SearchResult {
                    title: display_name.to_string(),
                    url: osm_url,
                    content,
                    engine: "openstreetmap".to_string(),
                    engine_rank: (i + 1) as u32,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });
            }
        }

        Ok(results)
    }
}
