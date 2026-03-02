//! Spotify engine — search tracks via Spotify Web API.
//! Translated from SearXNG `searx/engines/spotify.py`.
//!
//! Note: Requires `api_client_id` and `api_client_secret` for
//! OAuth client_credentials flow. Without credentials, search will
//! return an auth error.

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

pub struct Spotify {
    metadata: EngineMetadata,
    client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
}

impl Spotify {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "spotify".to_string().into(),
                display_name: "Spotify".to_string().into(),
                homepage: "https://www.spotify.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: false, // Disabled by default — needs API credentials
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            client_id: None,
            client_secret: None,
        }
    }

    pub fn with_credentials(client: Client, client_id: String, client_secret: String) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "spotify".to_string().into(),
                display_name: "Spotify".to_string().into(),
                homepage: "https://www.spotify.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            client_id: Some(client_id),
            client_secret: Some(client_secret),
        }
    }

    async fn get_access_token(&self) -> Result<String, MetasearchError> {
        let cid = self.client_id.as_deref().unwrap_or("");
        let csecret = self.client_secret.as_deref().unwrap_or("");

        if cid.is_empty() || csecret.is_empty() {
            return Err(MetasearchError::ConfigError(
                "Spotify requires api_client_id and api_client_secret".to_string(),
            ));
        }

        use base64::Engine as _;
        let credentials = format!("{}:{}", cid, csecret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());

        let resp = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", encoded))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        data["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| MetasearchError::Other("Failed to get Spotify access token".to_string()))
    }
}

#[async_trait]
impl SearchEngine for Spotify {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let token = self.get_access_token().await?;
        let page = query.page;
        let offset = (page - 1) * 20;

        let url = format!(
            "https://api.spotify.com/v1/search?q={}&type=track&offset={}",
            urlencoding::encode(&query.query),
            offset,
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(items) = data["tracks"]["items"].as_array() {
            for (i, item) in items.iter().enumerate() {
                if item["type"].as_str() != Some("track") {
                    continue;
                }

                let title = item["name"].as_str().unwrap_or_default();
                let link = item["external_urls"]["spotify"]
                    .as_str()
                    .unwrap_or_default();
                let artist = item["artists"][0]["name"].as_str().unwrap_or("");
                let album = item["album"]["name"].as_str().unwrap_or("");
                let duration_ms = item["duration_ms"].as_u64().unwrap_or(0);
                let minutes = duration_ms / 60000;
                let seconds = (duration_ms % 60000) / 1000;

                let snippet = format!("{} — {} [{}:{:02}]", artist, album, minutes, seconds,);

                let mut result = SearchResult::new(
                    title.to_string(),
                    link.to_string(),
                    snippet,
                    "spotify".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Music.to_string();
                result.thumbnail = item["album"]["images"]
                    .as_array()
                    .and_then(|imgs| imgs.first())
                    .and_then(|img| img["url"].as_str())
                    .map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}
