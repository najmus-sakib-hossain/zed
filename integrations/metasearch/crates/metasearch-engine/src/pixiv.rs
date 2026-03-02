//! Pixiv illustration search engine.
//! Uses the Pixiv AJAX API to search for illustrations.
//!
//! Reference: <https://www.pixiv.net>

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

pub struct Pixiv {
    metadata: EngineMetadata,
    client: Client,
}

impl Pixiv {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "pixiv".to_string().into(),
                display_name: "Pixiv".to_string().into(),
                homepage: "https://www.pixiv.net".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 6000,
                weight: 0.8,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Pixiv {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.max(1);
        let encoded = urlencoding::encode(&query.query);
        let url = format!(
            "https://www.pixiv.net/ajax/search/illustrations/{}?word={}&order=date_d&mode=all&p={}&s_mode=s_tag_full&type=illust_and_ugoira&lang=en",
            encoded, encoded, page,
        );

        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://www.pixiv.net/")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let items = match json
            .get("body")
            .and_then(|b| b.get("illust"))
            .and_then(|i| i.get("data"))
            .and_then(|d| d.as_array())
        {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let title = item["title"].as_str().unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let id = match item["id"].as_str().or_else(|| {
                item["id"].as_u64().map(|_| "")
            }) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => match item["id"].as_u64() {
                    Some(n) => n.to_string(),
                    None => continue,
                },
            };

            let artwork_url = format!("https://www.pixiv.net/en/artworks/{}", id);
            let user_name = item["userName"].as_str().unwrap_or("Unknown");
            let alt = item["alt"].as_str().unwrap_or_default();

            let content = if !alt.is_empty() {
                format!("{} — by {}", alt, user_name)
            } else {
                format!("by {}", user_name)
            };

            let thumbnail = item["url"].as_str().unwrap_or_default();

            let mut result = SearchResult::new(title, &artwork_url, &content, "pixiv");
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            if !thumbnail.is_empty() {
                result.thumbnail = Some(thumbnail.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}
