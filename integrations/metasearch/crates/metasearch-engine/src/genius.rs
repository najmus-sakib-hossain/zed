//! Genius engine — search lyrics, songs, artists, albums via Genius API.
//! Translated from SearXNG `searx/engines/genius.py`.

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

const PAGE_SIZE: u32 = 5;

pub struct Genius {
    metadata: EngineMetadata,
    client: Client,
}

impl Genius {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "genius".to_string().into(),
                display_name: "Genius".to_string().into(),
                homepage: "https://genius.com".to_string().into(),
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
impl SearchEngine for Genius {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;

        let url = format!(
            "https://genius.com/api/search/multi?q={}&page={}&per_page={}",
            urlencoding::encode(&query.query),
            page,
            PAGE_SIZE,
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

        if let Some(sections) = data["response"]["sections"].as_array() {
            for section in sections {
                if let Some(hits) = section["hits"].as_array() {
                    for hit in hits {
                        let hit_type = hit["type"].as_str().unwrap_or("");
                        let res = &hit["result"];

                        match hit_type {
                            "song" | "lyric" => {
                                let title = res["full_title"].as_str().unwrap_or_default();
                                let url = res["url"].as_str().unwrap_or_default();

                                let content = hit["highlights"]
                                    .as_array()
                                    .and_then(|h| h.first())
                                    .and_then(|h| h["value"].as_str())
                                    .or_else(|| res["title_with_featured"].as_str())
                                    .unwrap_or("")
                                    .to_string();

                                let mut result = SearchResult::new(
                                    title.to_string(),
                                    url.to_string(),
                                    content,
                                    "genius".to_string(),
                                );
                                result.engine_rank = (results.len() + 1) as u32;
                                result.category = SearchCategory::Music.to_string();
                                result.thumbnail = res["song_art_image_thumbnail_url"]
                                    .as_str()
                                    .map(|s| s.to_string());
                                results.push(result);
                            }
                            "artist" => {
                                let name = res["name"].as_str().unwrap_or_default();
                                let url = res["url"].as_str().unwrap_or_default();

                                let mut result = SearchResult::new(
                                    name.to_string(),
                                    url.to_string(),
                                    "Artist on Genius".to_string(),
                                    "genius".to_string(),
                                );
                                result.engine_rank = (results.len() + 1) as u32;
                                result.category = SearchCategory::Music.to_string();
                                result.thumbnail = res["image_url"].as_str().map(|s| s.to_string());
                                results.push(result);
                            }
                            "album" => {
                                let full_title = res["full_title"].as_str().unwrap_or_default();
                                let url = res["url"].as_str().unwrap_or_default();
                                let name_with_artist = res["name_with_artist"]
                                    .as_str()
                                    .or_else(|| res["name"].as_str())
                                    .unwrap_or("");
                                let year = res["release_date_components"]["year"].as_u64();

                                let content = match year {
                                    Some(y) => format!("{} / {}", y, name_with_artist),
                                    None => name_with_artist.to_string(),
                                };

                                let mut result = SearchResult::new(
                                    full_title.to_string(),
                                    url.to_string(),
                                    content,
                                    "genius".to_string(),
                                );
                                result.engine_rank = (results.len() + 1) as u32;
                                result.category = SearchCategory::Music.to_string();
                                result.thumbnail =
                                    res["cover_art_url"].as_str().map(|s| s.to_string());
                                results.push(result);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}
