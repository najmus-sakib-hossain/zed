//! SepiaSearch — federated PeerTube video search (JSON API)
//!
//! Searches sepiasearch.org, a search index for PeerTube videos.

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

const BASE_URL: &str = "https://sepiasearch.org";
const RESULTS_PER_PAGE: u32 = 10;

pub struct SepiaSearch {
    client: Client,
}

impl SepiaSearch {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct SepiaResponse {
    data: Option<Vec<SepiaVideo>>,
}

#[derive(Debug, Deserialize)]
struct SepiaVideo {
    name: Option<String>,
    url: Option<String>,
    description: Option<String>,
    #[serde(rename = "thumbnailUrl")]
    thumbnail: Option<String>,
    #[serde(rename = "previewPath")]
    preview_path: Option<String>,
    channel: Option<SepiaChannel>,
    duration: Option<i64>,
    #[serde(rename = "publishedAt")]
    #[allow(dead_code)]
    published_at: Option<String>,
    #[allow(dead_code)]
    account: Option<SepiaAccount>,
}

#[derive(Debug, Deserialize)]
struct SepiaChannel {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    host: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SepiaAccount {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[async_trait]
impl SearchEngine for SepiaSearch {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "sepiasearch".to_string().into(),
            display_name: "sepiasearch".to_string().into(),
            homepage: "https://sepiasearch.com".to_string().into(),
            categories: smallvec![SearchCategory::Videos],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let start = (query.page - 1) * RESULTS_PER_PAGE;
        let url = format!(
            "{}/api/v1/search/videos?search={}&start={}&count={}&sort=-match",
            BASE_URL,
            urlencoding::encode(&query.query),
            start,
            RESULTS_PER_PAGE
        );

        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                MetasearchError::Engine(format!("SepiaSearch request failed: {}", e))
            })?;

        let data: SepiaResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("SepiaSearch parse failed: {}", e)))?;

        let results = data
            .data
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, video)| {
                let title = video.name?;
                let video_url = video.url?;

                let mut snippet_parts = Vec::new();
                if let Some(channel) = &video.channel {
                    if let Some(name) = &channel.display_name {
                        let host = channel.host.as_deref().unwrap_or("");
                        if host.is_empty() {
                            snippet_parts.push(name.clone());
                        } else {
                            snippet_parts.push(format!("{}@{}", name, host));
                        }
                    }
                }
                if let Some(dur) = video.duration {
                    let mins = dur / 60;
                    let secs = dur % 60;
                    snippet_parts.push(format!("{}:{:02}", mins, secs));
                }
                if let Some(desc) = &video.description {
                    let short: String = desc.chars().take(150).collect();
                    if !short.is_empty() {
                        snippet_parts.push(short);
                    }
                }
                let snippet = snippet_parts.join(" — ");

                let thumbnail = video.thumbnail.or(video.preview_path);

                let mut result = SearchResult::new(&title, &video_url, &snippet, "sepiasearch");
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Videos.to_string();
                result.thumbnail = thumbnail;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
