//! Rumble — video search via HTML scraping.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct Rumble {
    metadata: EngineMetadata,
    client: Client,
}

impl Rumble {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "rumble".to_string().into(),
                display_name: "Rumble".to_string().into(),
                homepage: "https://rumble.com".to_string().into(),
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
impl SearchEngine for Rumble {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://rumble.com/search/video?q={}&page={}",
            urlencoding::encode(&query.query),
            query.page
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Rumble request failed: {e}")))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Rumble read failed: {e}")))?;

        let document = Html::parse_document(&html_text);
        let item_sel = Selector::parse("li.video-listing-entry").unwrap();
        let link_sel = Selector::parse("a.video-item--a").unwrap();
        let title_sel = Selector::parse("h3.video-item--title").unwrap();
        let desc_sel = Selector::parse(".video-item--meta").unwrap();
        let thumb_sel = Selector::parse("img.video-item--img").unwrap();

        let mut results = Vec::new();
        for (i, el) in document.select(&item_sel).enumerate() {
            let title = el
                .select(&title_sel)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let href = el
                .select(&link_sel)
                .next()
                .and_then(|a| a.value().attr("href"))
                .unwrap_or_default();

            let result_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://rumble.com{}", href)
            };

            let snippet = el
                .select(&desc_sel)
                .next()
                .map(|d| d.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let thumbnail_url = el
                .select(&thumb_sel)
                .next()
                .and_then(|img| img.value().attr("src").map(|s| s.to_string()));

            if !title.is_empty() && !href.is_empty() {
                let mut result = SearchResult::new(
                    title,
                    result_url,
                    snippet,
                    "rumble",
                );
                result.engine_rank = (i + 1) as u32;
                result.thumbnail = thumbnail_url;
                results.push(result);
            }
        }

        Ok(results)
    }
}
