//! 360 Search Videos — Chinese video search from 360kan.
//!
//! Simple JSON API, no authentication required.
//!
//! Reference: <https://tv.360kan.com>

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct ThreeSixtySearchVideos {
    metadata: EngineMetadata,
    client: Client,
}

impl ThreeSixtySearchVideos {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "360search_videos".to_string().into(),
                display_name: "360 Search Videos".to_string().into(),
                homepage: "https://tv.360kan.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.5,
            },
            client,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    data: Option<ApiData>,
}

#[derive(Debug, Deserialize)]
struct ApiData {
    result: Option<Vec<VideoItem>>,
}

#[derive(Debug, Deserialize)]
struct VideoItem {
    title: Option<String>,
    play_url: Option<String>,
    description: Option<String>,
    cover_img: Option<String>,
}

#[async_trait]
impl SearchEngine for ThreeSixtySearchVideos {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let start = query.page.saturating_sub(1) * 10;

        let url = format!(
            "https://tv.360kan.com/v1/video/list?count=10&q={}&start={}",
            urlencoding::encode(&query.query),
            start,
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "application/json, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", "https://tv.360kan.com/")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let api: ApiResponse = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let items = api.data.and_then(|d| d.result).unwrap_or_default();

        let mut results = Vec::new();

        for (rank, item) in items.iter().enumerate() {
            let title = match &item.title {
                Some(t) if !t.is_empty() => t.clone(),
                _ => continue,
            };
            let play_url = match &item.play_url {
                Some(u) if !u.is_empty() => u.clone(),
                _ => continue,
            };
            let description = item.description.clone().unwrap_or_default();

            let mut result =
                SearchResult::new(title, play_url, description, self.metadata.name.clone());
            result.engine_rank = (rank + 1) as u32;
            result.thumbnail = item.cover_img.clone();
            results.push(result);
        }

        Ok(results)
    }
}
