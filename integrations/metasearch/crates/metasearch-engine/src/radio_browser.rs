//! Radio Browser — search internet radio stations.
//!
//! Queries the Radio Browser JSON API for radio stations by name,
//! sorted by popularity (votes).
//!
//! Reference: <https://www.radio-browser.info>

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::MetasearchError;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const API_URL: &str = "https://de1.api.radio-browser.info";
const RESULTS_PER_PAGE: u32 = 10;

pub struct RadioBrowser {
    metadata: EngineMetadata,
    client: Client,
}

impl RadioBrowser {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "radio_browser".to_string().into(),
                display_name: "Radio Browser".to_string().into(),
                homepage: "https://www.radio-browser.info".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct Station {
    name: Option<String>,
    base_url: Option<String>,
    url_resolved: Option<String>,
    tags: Option<String>,
    country: Option<String>,
    codec: Option<String>,
    bitrate: Option<u32>,
    votes: Option<u32>,
    favicon: Option<String>,
}

#[async_trait]
impl SearchEngine for RadioBrowser {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let offset = query.page.saturating_sub(1) * RESULTS_PER_PAGE;

        let url = format!(
            "{}/json/stations/search?name={}&order=votes&offset={}&limit={}&hidebroken=true&reverse=true",
            API_URL,
            urlencoding::encode(&query.query),
            offset,
            RESULTS_PER_PAGE,
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let stations: Vec<Station> = resp
            .json()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let results = stations
            .into_iter()
            .enumerate()
            .filter_map(|(i, station)| {
                let name = station.name.filter(|n| !n.is_empty())?;

                // Prefer base_url, fall back to stream URL
                let url = station
                    .base_url
                    .filter(|h| !h.is_empty())
                    .or(station.url_resolved)?;

                // Build descriptive content from tags, country, codec info
                let mut parts = Vec::new();
                if let Some(tags) = &station.tags {
                    let tags = tags.trim();
                    if !tags.is_empty() {
                        parts.push(tags.to_string());
                    }
                }
                if let Some(country) = &station.country {
                    let country = country.trim();
                    if !country.is_empty() {
                        parts.push(country.to_string());
                    }
                }
                let mut meta = Vec::new();
                if let Some(codec) = &station.codec {
                    if !codec.is_empty() && codec.to_lowercase() != "unknown" {
                        meta.push(format!("{} radio", codec));
                    }
                }
                if let Some(bitrate) = station.bitrate {
                    if bitrate > 0 {
                        meta.push(format!("{}kbps", bitrate));
                    }
                }
                if let Some(votes) = station.votes {
                    if votes > 0 {
                        meta.push(format!("{} votes", votes));
                    }
                }
                if !meta.is_empty() {
                    parts.push(meta.join(" | "));
                }

                let content = parts.join(" \u{00b7} ");

                let mut result = SearchResult::new(name, url, content, "radio_browser");
                result.engine_rank = (i + 1) as u32;
                if let Some(favicon) = station.favicon {
                    if !favicon.is_empty() {
                        result.thumbnail = Some(favicon.replace("http://", "https://"));
                    }
                }
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
