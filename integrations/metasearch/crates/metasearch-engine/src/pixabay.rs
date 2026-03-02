//! Pixabay image search engine implementation.
//!
//! JSON API with special headers: <https://pixabay.com/images/search/>
//! Website: https://pixabay.com
//! Features: Paging, Time range

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

pub struct Pixabay {
    metadata: EngineMetadata,
    client: Client,
}

impl Pixabay {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "pixabay".to_string().into(),
                display_name: "Pixabay".to_string().into(),
                homepage: "https://pixabay.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map time_range string to Pixabay date parameter.
    fn map_time_range(time_range: Option<&str>) -> Option<&'static str> {
        match time_range {
            Some("day") => Some("1d"),
            Some("week") => Some("1w"),
            Some("month") => Some("1m"),
            Some("year") => Some("1y"),
            _ => None,
        }
    }
}

#[async_trait]
impl SearchEngine for Pixabay {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let page = query.page.max(1);

        let mut url = format!(
            "https://pixabay.com/images/search/{}/?pagi={}",
            encoded, page
        );

        if let Some(date_param) = Self::map_time_range(query.time_range.as_deref()) {
            url.push_str(&format!("&date={}", date_param));
        }

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept", "application/json")
            .header("x-bootstrap-cache-miss", "1")
            .header("x-fetch-bootstrap", "1")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        // If redirect (301/302), return empty results
        let status = resp.status().as_u16();
        if status == 301 || status == 302 {
            return Ok(Vec::new());
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        let items = match data
            .get("page")
            .and_then(|p| p.get("results"))
            .and_then(|r| r.as_array())
        {
            Some(items) => items,
            None => return Ok(results),
        };

        for (i, item) in items.iter().enumerate() {
            let href = item.get("href").and_then(|h| h.as_str()).unwrap_or_default();
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or_default();

            if href.is_empty() || name.is_empty() {
                continue;
            }

            let result_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://pixabay.com{}", href)
            };

            // Extract first source as thumbnail
            let thumbnail = item
                .get("sources")
                .and_then(|s| s.as_array())
                .and_then(|a| a.first())
                .and_then(|s| s.as_str())
                .or_else(|| {
                    item.get("source")
                        .and_then(|s| s.as_str())
                })
                .map(|s| s.to_string());

            let mut r = SearchResult::new(name, &result_url, description, "pixabay");
            r.engine_rank = i as u32;
            r.category = SearchCategory::Images.to_string();
            r.thumbnail = thumbnail;
            results.push(r);
        }

        info!(
            engine = "pixabay",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
