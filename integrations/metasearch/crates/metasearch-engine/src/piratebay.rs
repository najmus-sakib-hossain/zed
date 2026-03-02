//! The Pirate Bay — torrent search engine
//!
//! Uses the apibay.org JSON API to search for torrents.
//! No authentication required.
//!
//! Reference: <https://apibay.org/>

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct PirateBay {
    client: Client,
}

impl PirateBay {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct TorrentResult {
    name: Option<String>,
    id: Option<String>,
    info_hash: Option<String>,
    seeders: Option<String>,
    leechers: Option<String>,
    size: Option<String>,
}

const TRACKERS: &[&str] = &[
    "udp://tracker.coppersurfer.tk:6969/announce",
    "udp://9.rarbg.to:2920/announce",
    "udp://tracker.opentrackr.org:1337",
    "udp://tracker.internetwarriors.net:1337/announce",
    "udp://tracker.leechers-paradise.org:6969/announce",
    "udp://tracker.pirateparty.gr:6969/announce",
    "udp://tracker.cyberia.is:6969/announce",
];

#[async_trait]
impl SearchEngine for PirateBay {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "The Pirate Bay".to_string().into(),
            display_name: "The Pirate Bay".to_string().into(),
            homepage: "https://The Pirate Bay.com".to_string().into(),
            categories: smallvec![SearchCategory::Files],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://apibay.org/q.php?q={}&cat=0",
            urlencoding::encode(&query.query)
        );

        let resp = self.client.get(&url).send().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;
        let data: Vec<TorrentResult> = resp.json().await.map_err(|e| MetasearchError::Engine(e.to_string()))?;

        let results = data
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let name = item.name?;
                let info_hash = item.info_hash?;
                let id = item.id.unwrap_or_default();

                // Skip "No results" placeholder
                if name == "No results returned"
                    || info_hash == "0000000000000000000000000000000000000000"
                {
                    return None;
                }

                // Build magnet link
                let tracker_params: String = TRACKERS
                    .iter()
                    .map(|t| format!("&tr={}", urlencoding::encode(t)))
                    .collect();
                let magnetlink = format!(
                    "magnet:?xt=urn:btih:{}&dn={}{}",
                    info_hash,
                    urlencoding::encode(&name),
                    tracker_params
                );

                let link = format!("https://thepiratebay.org/description.php?id={}", id);

                let seeders = item.seeders.unwrap_or_default();
                let leechers = item.leechers.unwrap_or_default();
                let size_bytes: u64 = item.size.unwrap_or_default().parse().unwrap_or(0);
                let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
                let size_str = if size_mb >= 1024.0 {
                    format!("{:.1} GB", size_mb / 1024.0)
                } else {
                    format!("{:.1} MB", size_mb)
                };

                let content = format!(
                    "Size: {} — Seeders: {} — Leechers: {} — Magnet: {}",
                    size_str, seeders, leechers, magnetlink
                );

                let mut result = SearchResult::new(
                    name,
                    link,
                    content,
                    "piratebay",
                );
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}

