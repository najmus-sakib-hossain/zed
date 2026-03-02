//! Apple App Store search engine implementation.
//!
//! Translated from SearXNG's `apple_app_store.py` (54 lines, JSON API).
//! Search the Apple App Store via the iTunes Search API.
//! Website: https://www.apple.com/app-store/
//! Features: SafeSearch
//! API: https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/iTuneSearchAPI/

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;
use smallvec::smallvec;

pub struct AppleAppStore {
    metadata: EngineMetadata,
    client: Client,
}

impl AppleAppStore {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "apple_app_store".to_string().into(),
                display_name: "Apple App Store".to_string().into(),
                homepage: "https://www.apple.com/app-store/".to_string().into(),
                categories: smallvec![SearchCategory::Files, SearchCategory::IT],
                enabled: true,
                timeout_ms: 3000,
                weight: 0.9,
            },
            client,
        }
    }
}

#[derive(Deserialize, Debug)]
struct ItunesResponse {
    results: Vec<ItunesResult>,
}

#[derive(Deserialize, Debug)]
struct ItunesResult {
    #[serde(rename = "trackViewUrl")]
    track_view_url: String,
    #[serde(rename = "trackName")]
    track_name: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "artworkUrl100", default)]
    artwork_url: String,
    #[serde(rename = "sellerName", default)]
    seller_name: String,
}

#[async_trait]
impl SearchEngine for AppleAppStore {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let explicit = if query.safe_search > 0 { "No" } else { "Yes" };
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://itunes.apple.com/search?term={}&media=software&explicit={}",
            encoded, explicit
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: ItunesResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        for (i, item) in data.results.iter().enumerate() {
            let snippet = if item.seller_name.is_empty() {
                item.description.chars().take(300).collect()
            } else {
                format!(
                    "by {} — {}",
                    item.seller_name,
                    item.description.chars().take(250).collect::<String>()
                )
            };

            let mut r = SearchResult::new(
                &item.track_name,
                &item.track_view_url,
                &snippet,
                "apple_app_store",
            );
            r.engine_rank = i as u32;
            r.category = SearchCategory::Files.to_string();
            if !item.artwork_url.is_empty() {
                r.thumbnail = Some(item.artwork_url.clone());
            }

            results.push(r);
        }

        info!(
            engine = "apple_app_store",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
