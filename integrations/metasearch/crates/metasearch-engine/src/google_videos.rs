//! Google Videos search — HTML scraping.
//! SearXNG equivalent: `google_videos.py`

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct GoogleVideos {
    client: Client,
}

impl GoogleVideos {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for GoogleVideos {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Google Videos".to_string().into(),
            display_name: "Google Videos".to_string().into(),
            homepage: "https://Google Videos.com".to_string().into(),
            categories: smallvec![metasearch_core::category::SearchCategory::Videos],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let start = (query.page - 1) * 10;
        let url = format!(
            "https://www.google.com/search?q={}&tbm=vid&start={}&hl=en",
            urlencoding::encode(&query.query),
            start,
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Google Videos: {e}")))?;

        let text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Google Videos body: {e}")))?;

        let doc = Html::parse_document(&text);
        let container_sel = Selector::parse("div.MjjYud").unwrap();
        let h3_sel = Selector::parse("h3").unwrap();
        let a_sel = Selector::parse("a[href]").unwrap();
        let snippet_sel = Selector::parse("div.ITZIwc").unwrap();

        let mut results = Vec::new();
        for (i, el) in doc.select(&container_sel).enumerate() {
            let title = match el.select(&h3_sel).next() {
                Some(t) => t.text().collect::<String>(),
                None => continue,
            };
            if title.is_empty() {
                continue;
            }

            let href = match el
                .select(&a_sel)
                .next()
                .and_then(|a| a.value().attr("href"))
            {
                Some(h) if h.starts_with("http") => h.to_string(),
                _ => continue,
            };

            let content = el
                .select(&snippet_sel)
                .next()
                .map(|s| s.text().collect::<String>())
                .unwrap_or_default();

            results.push(SearchResult {
                title,
                url: href,
                content,
                engine: "Google Videos".to_string(),
                engine_rank: (i + 1) as u32,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });
        }
        Ok(results)
    }
}
