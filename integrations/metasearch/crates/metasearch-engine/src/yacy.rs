//! YaCy — self-hosted peer-to-peer search engine.
//! Uses the YaCy JSON search API.
//!
//! Reference: <https://yacy.net>

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

pub struct Yacy {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl Yacy {
    pub fn new(client: Client, base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let enabled = !base.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "yacy".to_string().into(),
                display_name: "YaCy".to_string().into(),
                homepage: "https://yacy.net".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 6000,
                weight: 0.7,
            },
            client,
            base_url: base,
        }
    }
}

#[async_trait]
impl SearchEngine for Yacy {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let start_record = (query.page.saturating_sub(1)) * 10;
        let url = format!(
            "{}/yacysearch.json?query={}&startRecord={}&maximumRecords=10&contentdom=text&resource=global",
            self.base_url,
            urlencoding::encode(&query.query),
            start_record,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let items = match json
            .get("channels")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|ch| ch.get("items"))
            .and_then(|i| i.as_array())
        {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let link = item["link"].as_str().unwrap_or_default();
            if link.is_empty() {
                continue;
            }

            let title = item["title"].as_str().unwrap_or("Untitled").to_string();
            let description = item["description"].as_str().unwrap_or_default().to_string();

            let mut result = SearchResult::new(&title, link, &description, "yacy");
            result.engine_rank = (i + 1) as u32;

            // Try to parse publication date
            if let Some(pub_date_str) = item["pubDate"].as_str() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(pub_date_str) {
                    result.published_date = Some(dt.with_timezone(&chrono::Utc));
                } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(pub_date_str) {
                    result.published_date = Some(dt.with_timezone(&chrono::Utc));
                }
            }

            results.push(result);
        }

        Ok(results)
    }
}
