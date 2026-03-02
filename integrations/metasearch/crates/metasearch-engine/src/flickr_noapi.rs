//! Flickr (no API) — image search on Flickr without using the official API.
//!
//! Uses the Flickr public feeds JSON endpoint, which requires no API key.
//!
//! Reference: <https://www.flickr.com/services/feeds/photos_public.gne>

use async_trait::async_trait;
use reqwest::Client;

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::MetasearchError;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const BASE_URL: &str = "https://www.flickr.com";

pub struct FlickrNoapi {
    metadata: EngineMetadata,
    client: Client,
}

impl FlickrNoapi {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "flickr_noapi".to_string().into(),
                display_name: "Flickr (no API)".to_string().into(),
                homepage: BASE_URL.to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for FlickrNoapi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // Flickr public feeds JSON API — no API key required.
        // Note: this endpoint does not support paging; always returns 20 recent public photos.
        let url = format!(
            "https://www.flickr.com/services/feeds/photos_public.gne?q={}&format=json&nojsoncallback=1",
            urlencoding::encode(&query.query)
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "application/json, */*")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => return Ok(Vec::new()),
        };

        let data: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let items = match data["items"].as_array() {
            Some(a) => a,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let title = item["title"].as_str().unwrap_or("Flickr photo").trim().to_string();
            let link = item["link"].as_str().unwrap_or_default();
            if link.is_empty() {
                continue;
            }

            // Thumbnail URL is in media.m
            let thumbnail = item["media"]["m"].as_str().unwrap_or_default().to_string();

            // Author looks like: `nobody@flickr.com ("username")`
            let author_raw = item["author"].as_str().unwrap_or_default();
            let author = if let Some(start) = author_raw.find("(\"") {
                let s = &author_raw[start + 2..];
                s.trim_end_matches("\")").to_string()
            } else {
                String::new()
            };

            let tags = item["tags"].as_str().unwrap_or_default();
            let content = if !author.is_empty() {
                if !tags.is_empty() {
                    format!("by {author} | tags: {tags}")
                } else {
                    format!("by {author}")
                }
            } else {
                tags.to_string()
            };

            let mut result = SearchResult::new(
                if title.is_empty() { "Flickr photo" } else { &title },
                link,
                &content,
                "flickr_noapi",
            );
            result.engine_rank = i as u32;
            result.category = SearchCategory::Images.to_string();
            if !thumbnail.is_empty() {
                result.thumbnail = Some(thumbnail);
            }
            results.push(result);
        }

        Ok(results)
    }
}

