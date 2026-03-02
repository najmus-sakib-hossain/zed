//! Apple Maps — via Apple MapKit API with token from DuckDuckGo
//!
//! Obtains an API token from DuckDuckGo's local.js endpoint,
//! then queries Apple's MapKit search API.
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/apple_maps.py>

use async_trait::async_trait;
use reqwest::Client;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct AppleMaps {
    client: Client,
    token: Mutex<TokenCache>,
}

struct TokenCache {
    value: String,
    last_updated: u64,
}

impl AppleMaps {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            token: Mutex::new(TokenCache {
                value: String::new(),
                last_updated: 0,
            }),
        }
    }

    async fn obtain_token(&self) -> Result<String, MetasearchError> {
        // Step 1: Get DDG's MapKit token
        let token_resp = self
            .client
            .get("https://duckduckgo.com/local.js?get_mk_token=1")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Apple Maps token step 1 error: {e}")))?;

        let ddg_token = token_resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Apple Maps token read error: {e}")))?;

        // Step 2: Exchange for MapKit access token
        let bootstrap_resp = self
            .client
            .get("https://cdn.apple-mapkit.com/ma/bootstrap?apiVersion=2&mkjsVersion=5.72.53&poi=1")
            .header("Authorization", format!("Bearer {}", ddg_token.trim()))
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Apple Maps bootstrap error: {e}")))?;

        let json: serde_json::Value = bootstrap_resp.json().await.map_err(|e| {
            MetasearchError::Engine(format!("Apple Maps bootstrap parse error: {e}"))
        })?;

        let access_token = json
            .get("authInfo")
            .and_then(|a| a.get("access_token"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MetasearchError::Engine("Apple Maps: no access_token in bootstrap".to_string())
            })?
            .to_string();

        Ok(access_token)
    }

    async fn get_token(&self) -> Result<String, MetasearchError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check cache (refresh every 30 minutes = 1800 seconds)
        {
            let cache = self.token.lock().unwrap();
            if !cache.value.is_empty() && (now - cache.last_updated) < 1800 {
                return Ok(cache.value.clone());
            }
        }

        let new_token = self.obtain_token().await?;

        {
            let mut cache = self.token.lock().unwrap();
            cache.value = new_token.clone();
            cache.last_updated = now;
        }

        Ok(new_token)
    }
}

#[async_trait]
impl SearchEngine for AppleMaps {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "AppleMaps".to_string().into(),
            display_name: "AppleMaps".to_string().into(),
            homepage: "https://www.apple.com/maps/".to_string().into(),
            categories: smallvec![SearchCategory::Maps],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let token = self.get_token().await?;

        let url = format!(
            "https://api.apple-mapkit.com/v1/search?q={}&lang=en&mkjsVersion=5.72.53",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Apple Maps search error: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Apple Maps parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(items) = json.get("results").and_then(|v| v.as_array()) {
            for (i, item) in items.iter().enumerate() {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let placecard_url = item
                    .get("placecardUrl")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let lat = item
                    .get("center")
                    .and_then(|c| c.get("lat"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let lng = item
                    .get("center")
                    .and_then(|c| c.get("lng"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                let locality = item
                    .get("locality")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let country = item
                    .get("country")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let category = item
                    .get("poiCategory")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let mut address_parts = Vec::new();
                if let Some(st) = item.get("subThoroughfare").and_then(|v| v.as_str()) {
                    address_parts.push(st.to_string());
                }
                if let Some(road) = item.get("thoroughfare").and_then(|v| v.as_str()) {
                    address_parts.push(road.to_string());
                }
                if !locality.is_empty() {
                    address_parts.push(locality.to_string());
                }
                if !country.is_empty() {
                    address_parts.push(country.to_string());
                }

                let content = if !category.is_empty() {
                    format!(
                        "{} — {} (lat: {:.4}, lon: {:.4})",
                        address_parts.join(", "),
                        category,
                        lat,
                        lng
                    )
                } else {
                    format!(
                        "{} (lat: {:.4}, lon: {:.4})",
                        address_parts.join(", "),
                        lat,
                        lng
                    )
                };

                results.push(SearchResult {
                    title: name.to_string(),
                    url: placecard_url.to_string(),
                    content,
                    engine: "apple_maps".to_string(),
                    engine_rank: (i + 1) as u32,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });

                if results.len() >= 10 {
                    break;
                }
            }
        }

        Ok(results)
    }
}
