//! Yandex Music search engine implementation.
//!
//! JSON API: <https://music.yandex.ru/handlers/music-search.jsx>
//! Website: https://music.yandex.ru
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct YandexMusic {
    metadata: EngineMetadata,
    client: Client,
}

impl YandexMusic {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "yandex_music".to_string().into(),
                display_name: "Yandex Music".to_string().into(),
                homepage: "https://music.yandex.ru".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for YandexMusic {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let page = query.page.max(1) - 1; // 0-indexed

        let url = format!(
            "https://music.yandex.ru/handlers/music-search.jsx?text={}&page={}",
            encoded, page
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        let items = match data.get("tracks").and_then(|t| t.get("items")).and_then(|i| i.as_array())
        {
            Some(items) => items,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if item_type != "music" {
                continue;
            }

            let title = item.get("title").and_then(|t| t.as_str()).unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let artist = item
                .get("artists")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .and_then(|a| a.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("Unknown");

            let album_title = item
                .get("albums")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .and_then(|a| a.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("");

            let album_id = item
                .get("albums")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .and_then(|a| a.get("id"))
                .and_then(|id| id.as_u64())
                .unwrap_or(0);

            let track_id = item.get("id").and_then(|id| id.as_u64()).unwrap_or(0);

            if album_id == 0 || track_id == 0 {
                continue;
            }

            let result_url = format!(
                "https://music.yandex.ru/album/{}/track/{}",
                album_id, track_id
            );

            let content = format!("by {} — album: {}", artist, album_title);

            let mut r = SearchResult::new(title, &result_url, &content, "yandex_music");
            r.engine_rank = i as u32;
            r.category = SearchCategory::Music.to_string();
            results.push(r);
        }

        info!(
            engine = "yandex_music",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
