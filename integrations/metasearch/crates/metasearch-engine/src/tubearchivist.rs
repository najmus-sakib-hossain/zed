//! TubeArchivist — search videos via a self-hosted TubeArchivist instance.
//!
//! Uses the TubeArchivist JSON API.
//!
//! Reference: <https://www.tubearchivist.com>

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

pub struct TubeArchivist {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl TubeArchivist {
    pub fn new(client: Client, base_url: &str, token: Option<String>) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let enabled = !base.is_empty() && token.is_some();
        Self {
            metadata: EngineMetadata {
                name: "tubearchivist".to_string().into(),
                display_name: "TubeArchivist".to_string().into(),
                homepage: "https://www.tubearchivist.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: base,
            token,
        }
    }
}

#[async_trait]
impl SearchEngine for TubeArchivist {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let token = match &self.token {
            Some(t) => t.clone(),
            None => return Ok(Vec::new()),
        };

        let url = format!(
            "{}/api/search?query={}",
            self.base_url,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", token))
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let video_results = match json
            .get("results")
            .and_then(|r| r.get("video_results"))
            .and_then(|v| v.as_array())
        {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in video_results.iter().enumerate() {
            let youtube_id = item["youtube_id"].as_str().unwrap_or_default();
            if youtube_id.is_empty() {
                continue;
            }

            let title = item["title"].as_str().unwrap_or("Untitled").to_string();

            let description = item["description"].as_str().unwrap_or_default();
            let content = if description.len() > 300 {
                let mut end = 300;
                while !description.is_char_boundary(end) && end > 0 {
                    end -= 1;
                }
                format!("{}…", &description[..end])
            } else {
                description.to_string()
            };

            let video_url = format!("https://www.youtube.com/watch?v={}", youtube_id);
            let thumbnail = item["vid_thumb_url"].as_str().map(|s| s.to_string());

            let mut result = SearchResult::new(&title, &video_url, &content, "tubearchivist");
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Videos.to_string();
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
