//! Freesound engine — search audio samples via Freesound API.
//! Translated from SearXNG `searx/engines/freesound.py`.
//!
//! **Requires an API key** from https://freesound.org/apiv2/apply/.
//! Disabled by default until a key is configured.

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

pub struct Freesound {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl Freesound {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "freesound".to_string().into(),
                display_name: "Freesound".to_string().into(),
                homepage: "https://freesound.org".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: false, // Disabled by default — needs API key
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            api_key: None,
        }
    }

    pub fn with_api_key(client: Client, api_key: String) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "freesound".to_string().into(),
                display_name: "Freesound".to_string().into(),
                homepage: "https://freesound.org".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: false, // Disabled by default — needs API key
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            api_key: Some(api_key),
        }
    }
}

#[async_trait]
impl SearchEngine for Freesound {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let api_key = match &self.api_key {
            Some(k) if !k.is_empty() => k.clone(),
            _ => {
                return Err(MetasearchError::ConfigError(
                    "Freesound requires an API key. Get one at https://freesound.org/apiv2/apply/"
                        .to_string(),
                ));
            }
        };

        let page = query.page;

        let url = format!(
            "https://freesound.org/apiv2/search/text/?query={}&page={}&fields=name,url,download,created,description,type&token={}",
            urlencoding::encode(&query.query),
            page,
            api_key,
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
                let name = item["name"].as_str().unwrap_or("Untitled");
                let sound_url = item["url"].as_str().unwrap_or_default();
                let download_url = item["download"].as_str().unwrap_or("");
                let description = item["description"].as_str().unwrap_or("");
                let sound_type = item["type"].as_str().unwrap_or("");

                if sound_url.is_empty() {
                    continue;
                }

                // Truncate description to 128 chars (matching Python)
                let truncated_desc = if description.len() > 128 {
                    format!("{}…", &description[..128])
                } else {
                    description.to_string()
                };

                let snippet = if sound_type.is_empty() {
                    truncated_desc
                } else {
                    format!("[{}] {}", sound_type, truncated_desc)
                };

                let mut sr = SearchResult::new(
                    name.to_string(),
                    sound_url.to_string(),
                    snippet,
                    "freesound".to_string(),
                );
                sr.engine_rank = (i + 1) as u32;
                sr.category = SearchCategory::Music.to_string();
                if !download_url.is_empty() {
                    sr.thumbnail = Some(download_url.to_string());
                }
                results.push(sr);
            }
        }

        Ok(results)
    }
}
