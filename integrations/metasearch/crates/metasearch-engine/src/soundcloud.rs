//! SoundCloud engine — search tracks and playlists via SoundCloud API v2.
//! Translated from SearXNG `searx/engines/soundcloud.py`.
//!
//! Note: In the full implementation, client_id discovery would be needed.
//! This version uses a placeholder approach — the client_id should be
//! configured or discovered at runtime.

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

const RESULTS_PER_PAGE: u32 = 10;

pub struct SoundCloud {
    metadata: EngineMetadata,
    client: Client,
    client_id: Option<String>,
}

impl SoundCloud {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "soundcloud".to_string().into(),
                display_name: "SoundCloud".to_string().into(),
                homepage: "https://soundcloud.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            client_id: None,
        }
    }

    pub fn with_client_id(client: Client, client_id: String) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "soundcloud".to_string().into(),
                display_name: "SoundCloud".to_string().into(),
                homepage: "https://soundcloud.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            client_id: Some(client_id),
        }
    }
}

#[async_trait]
impl SearchEngine for SoundCloud {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // If no client_id, try to discover one from soundcloud.com HTML
        let cid = match &self.client_id {
            Some(id) => id.clone(),
            None => discover_client_id(&self.client)
                .await
                .unwrap_or_else(|| "iZIs9mchVcX5lhVRyQGGAYlNPVldzAoX".to_string()),
        };

        let page = query.page;
        let offset = (page - 1) * RESULTS_PER_PAGE;

        let url = format!(
            "https://api-v2.soundcloud.com/search?q={}&offset={}&limit={}&facet=model&client_id={}",
            urlencoding::encode(&query.query),
            offset,
            RESULTS_PER_PAGE,
            cid,
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

        if let Some(collection) = data["collection"].as_array() {
            for (i, item) in collection.iter().enumerate() {
                let kind = item["kind"].as_str().unwrap_or("");
                if kind != "track" && kind != "playlist" {
                    continue;
                }

                let title = item["title"].as_str().unwrap_or_default();
                let permalink_url = item["permalink_url"].as_str().unwrap_or_default();
                if permalink_url.is_empty() {
                    continue;
                }

                let description = item["description"].as_str().unwrap_or("");
                let artist = item["user"]["full_name"]
                    .as_str()
                    .or_else(|| item["user"]["username"].as_str())
                    .unwrap_or("");
                let duration_ms = item["duration"].as_u64().unwrap_or(0);
                let duration_secs = duration_ms / 1000;
                let minutes = duration_secs / 60;
                let seconds = duration_secs % 60;
                let plays = item["playback_count"].as_u64().unwrap_or(0);

                let snippet = format!(
                    "by {} [{}:{:02}] — {} plays — {}",
                    artist,
                    minutes,
                    seconds,
                    plays,
                    if description.len() > 200 {
                        &description[..200]
                    } else {
                        description
                    },
                );

                let mut result = SearchResult::new(
                    title.to_string(),
                    permalink_url.to_string(),
                    snippet,
                    "soundcloud".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Music.to_string();
                result.thumbnail = item["artwork_url"]
                    .as_str()
                    .or_else(|| item["user"]["avatar_url"].as_str())
                    .map(|s| s.to_string());
                results.push(result);
            }
        }

        Ok(results)
    }
}

/// Attempt to discover SoundCloud's guest client_id from the website.
async fn discover_client_id(client: &Client) -> Option<String> {
    let resp = client.get("https://soundcloud.com").send().await.ok()?;
    let html = resp.text().await.ok()?;

    // Find JavaScript asset URLs
    let re = regex::Regex::new(r#"src="(https://a-v2\.sndcdn\.com/assets/[^"]+\.js)"#).ok()?;
    let script_urls: Vec<&str> = re
        .captures_iter(&html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .collect();

    // Search JS assets in reverse order for client_id
    let cid_re = regex::Regex::new(r#"client_id:"([^"]*)""#).ok()?;
    for script_url in script_urls.iter().rev() {
        if let Ok(resp) = client.get(*script_url).send().await {
            if let Ok(js_text) = resp.text().await {
                if let Some(caps) = cid_re.captures(&js_text) {
                    if let Some(cid) = caps.get(1) {
                        let id = cid.as_str().to_string();
                        if !id.is_empty() {
                            return Some(id);
                        }
                    }
                }
            }
        }
    }

    None
}
