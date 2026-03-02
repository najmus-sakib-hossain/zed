//! DeviantArt — image search on DeviantArt.
//!
//! Scrapes HTML search results from deviantart.com.
//!
//! Reference: <https://www.deviantart.com>

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct DeviantArt {
    metadata: EngineMetadata,
    client: Client,
}

impl DeviantArt {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "deviantart".to_string().into(),
                display_name: "DeviantArt".to_string().into(),
                homepage: "https://www.deviantart.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for DeviantArt {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://www.deviantart.com/search?q={}",
            urlencoding::encode(&query.query),
        );

        let resp = match self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(7))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);
        let result_sel = match Selector::parse("a[data-hook='deviation_link']") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let img_sel = match Selector::parse("img") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (rank, item) in document.select(&result_sel).enumerate() {
            let href = item.value().attr("href").unwrap_or_default();
            if href.is_empty() || !href.starts_with("http") {
                continue;
            }

            let title = item
                .value()
                .attr("aria-label")
                .unwrap_or("DeviantArt Image")
                .to_string();

            let img_el = item.select(&img_sel).next();
            let thumbnail_src = img_el
                .and_then(|el| el.value().attr("src"))
                .unwrap_or_default()
                .to_string();

            if thumbnail_src.is_empty() {
                continue;
            }

            let mut result = SearchResult::new(
                title,
                href.to_string(),
                String::new(),
                self.metadata.name.clone(),
            );
            result.engine_rank = (rank + 1) as u32;
            result.thumbnail = Some(thumbnail_src);
            results.push(result);
        }

        Ok(results)
    }
}
