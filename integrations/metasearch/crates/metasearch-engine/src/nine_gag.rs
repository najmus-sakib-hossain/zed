//! 9GAG search engine implementation.
//!
//! Translated from SearXNG's `9gag.py` (76 lines, JSON API).
//! 9GAG is a social media platform for memes and viral content.
//! Website: https://9gag.com/
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
use serde::Deserialize;
use tracing::info;
use smallvec::smallvec;

pub struct NineGag {
    metadata: EngineMetadata,
    client: Client,
}

impl NineGag {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "9gag".to_string().into(),
                display_name: "9GAG".to_string().into(),
                homepage: "https://9gag.com".to_string().into(),
                categories: smallvec![SearchCategory::SocialMedia],
                enabled: true,
                timeout_ms: 3000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[derive(Deserialize, Debug)]
struct GagResponse {
    data: GagData,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct GagData {
    posts: Vec<GagPost>,
    #[serde(default)]
    tags: Vec<GagTag>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct GagPost {
    #[serde(rename = "type")]
    post_type: String,
    url: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "creationTs")]
    creation_ts: Option<i64>,
    images: GagImages,
}

#[derive(Deserialize, Debug)]
struct GagImages {
    image700: GagImage,
    #[serde(rename = "imageFbThumbnail")]
    fb_thumbnail: Option<GagImage>,
}

#[derive(Deserialize, Debug)]
struct GagImage {
    url: String,
    #[serde(default)]
    height: u32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct GagTag {
    key: String,
}

#[async_trait]
impl SearchEngine for NineGag {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page_size = 10;
        let cursor = ((query.page.max(1) - 1) * page_size) as u64;
        let encoded = urlencoding::encode(&query.query);

        let url = format!(
            "https://9gag.com/v1/search-posts?query={}&c={}",
            encoded, cursor
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: GagResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        for (i, post) in data.data.posts.iter().enumerate() {
            // Choose thumbnail: use fb_thumbnail for tall images, image700 for others
            let thumbnail_url = if post.images.image700.height > 400 {
                post.images
                    .fb_thumbnail
                    .as_ref()
                    .map(|t| t.url.clone())
                    .unwrap_or_else(|| post.images.image700.url.clone())
            } else {
                post.images.image700.url.clone()
            };

            let mut r = SearchResult::new(&post.title, &post.url, &post.description, "9gag");
            r.engine_rank = i as u32;
            r.category = SearchCategory::SocialMedia.to_string();
            r.thumbnail = Some(thumbnail_url);

            if let Some(ts) = post.creation_ts {
                if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                    r.published_date = Some(dt);
                }
            }

            results.push(r);
        }

        info!(engine = "9gag", count = results.len(), "Search complete");
        Ok(results)
    }
}
