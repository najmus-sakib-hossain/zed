//! Reuters news search engine implementation.
//!
//! Queries the Reuters JSON API for news articles.
//! Website: https://www.reuters.com
//! Features: Paging, Time Range

use async_trait::async_trait;
use chrono::Utc;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Reuters {
    metadata: EngineMetadata,
    client: Client,
}

impl Reuters {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "reuters".to_string().into(),
                display_name: "Reuters".to_string().into(),
                homepage: "https://www.reuters.com".to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map time_range string to number of days.
    fn time_range_days(time_range: Option<&str>) -> Option<i64> {
        match time_range {
            Some("day") => Some(1),
            Some("week") => Some(7),
            Some("month") => Some(30),
            Some("year") => Some(365),
            _ => None,
        }
    }
}

#[async_trait]
impl SearchEngine for Reuters {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let offset = (page - 1) * 20;

        let mut query_args = serde_json::json!({
            "keyword": query.query,
            "offset": offset,
            "orderby": "display_date:desc",
            "size": 20,
            "website": "reuters"
        });

        // Apply time range filter
        if let Some(days) = Self::time_range_days(query.time_range.as_deref()) {
            let start_date = (Utc::now() - chrono::Duration::days(days))
                .format("%Y-%m-%d")
                .to_string();
            query_args["start_date"] = serde_json::Value::String(start_date);
        }

        let json_str = match serde_json::to_string(&query_args) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let encoded_args = urlencoding::encode(&json_str);

        let url = format!(
            "https://www.reuters.com/pf/api/v3/content/fetch/articles-by-search-v2?query={}",
            encoded_args
        );

        let resp = match self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(7))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let data: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        let articles = data["result"]["articles"].as_array();
        if let Some(articles) = articles {
            for (i, article) in articles.iter().enumerate() {
                // URL: canonical_url, prepend base if relative
                let canonical = article["canonical_url"].as_str().unwrap_or_default();
                if canonical.is_empty() {
                    continue;
                }
                let item_url = if canonical.starts_with('/') {
                    format!("https://www.reuters.com{}", canonical)
                } else {
                    canonical.to_string()
                };

                // Title: prefer web.title, fall back to title
                let title = article["web"]["title"]
                    .as_str()
                    .or_else(|| article["title"].as_str())
                    .unwrap_or_default();

                if title.is_empty() {
                    continue;
                }

                let description = article["description"].as_str().unwrap_or_default();
                let thumbnail = article["thumbnail"].as_str().map(|s| s.to_string());

                let mut r = SearchResult::new(title, &item_url, description, "reuters");
                r.engine_rank = i as u32;
                r.category = SearchCategory::News.to_string();
                r.thumbnail = thumbnail;
                results.push(r);
            }
        }

        Ok(results)
    }
}
