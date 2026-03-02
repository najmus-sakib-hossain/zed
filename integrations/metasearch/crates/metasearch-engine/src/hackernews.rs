//! Hacker News engine — search via Algolia HN Search API.
//! Translated from SearXNG `searx/engines/hackernews.py`.

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

pub struct HackerNews {
    metadata: EngineMetadata,
    client: Client,
}

impl HackerNews {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "hackernews".to_string().into(),
                display_name: "Hacker News".to_string().into(),
                homepage: "https://news.ycombinator.com".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for HackerNews {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.saturating_sub(1);

        let url = format!(
            "https://hn.algolia.com/api/v1/search?query={}&page={}&hitsPerPage=30&tags=story",
            urlencoding::encode(&query.query),
            page,
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

        if let Some(hits) = data["hits"].as_array() {
            for (i, hit) in hits.iter().enumerate() {
                let object_id = hit["objectID"].as_str().unwrap_or_default();
                let title = hit["title"].as_str().unwrap_or("Untitled");
                let points = hit["points"].as_u64().unwrap_or(0);
                let num_comments = hit["num_comments"].as_u64().unwrap_or(0);
                let author = hit["author"].as_str().unwrap_or("");

                let url_str = hit["url"].as_str().unwrap_or("");
                let hn_url = format!("https://news.ycombinator.com/item?id={}", object_id);

                let snippet = format!(
                    "{} — points: {} | comments: {} | by {}",
                    if url_str.is_empty() { &hn_url } else { url_str },
                    points,
                    num_comments,
                    author,
                );

                let mut result =
                    SearchResult::new(title.to_string(), hn_url, snippet, "hackernews".to_string());
                result.engine_rank = (i + 1) as u32;
                result.category = SearchCategory::IT.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
