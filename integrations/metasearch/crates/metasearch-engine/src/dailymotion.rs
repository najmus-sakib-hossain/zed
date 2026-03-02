//! Dailymotion engine — search videos via Dailymotion API.
//! Translated from SearXNG `searx/engines/dailymotion.py`.

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

const RESULTS_PER_PAGE: u32 = 10;

pub struct Dailymotion {
    metadata: EngineMetadata,
    client: Client,
}

impl Dailymotion {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "dailymotion".to_string().into(),
                display_name: "Dailymotion".to_string().into(),
                homepage: "https://www.dailymotion.com".to_string().into(),
                categories: smallvec![SearchCategory::Videos],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Dailymotion {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let fields = "allow_embed,description,title,created_time,duration,url,thumbnail_360_url,id";

        let url = format!(
            "https://api.dailymotion.com/videos?search={}&page={}&limit={}&fields={}&sort=relevance&family_filter=true&password_protected=false&private=false",
            urlencoding::encode(&query.query),
            page,
            RESULTS_PER_PAGE,
            fields,
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

        if let Some(list) = data["list"].as_array() {
            for (i, item) in list.iter().enumerate() {
                let title = item["title"].as_str().unwrap_or_default();
                let video_url = item["url"].as_str().unwrap_or_default();
                let description = item["description"].as_str().unwrap_or("");
                let duration = item["duration"].as_u64().unwrap_or(0);

                // Format duration
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let seconds = duration % 60;
                let duration_str = if hours > 0 {
                    format!("{}:{:02}:{:02}", hours, minutes, seconds)
                } else {
                    format!("{}:{:02}", minutes, seconds)
                };

                let content = if description.len() > 300 {
                    format!("{}... [{}]", &description[..300], duration_str)
                } else {
                    format!("{} [{}]", description, duration_str)
                };

                let mut result = SearchResult::new(
                    title.to_string(),
                    video_url.to_string(),
                    content,
                    "dailymotion".to_string(),
                );
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::Videos.to_string();
                result.thumbnail = item["thumbnail_360_url"]
                    .as_str()
                    .map(|t| t.replace("http://", "https://"));
                results.push(result);
            }
        }

        Ok(results)
    }
}
