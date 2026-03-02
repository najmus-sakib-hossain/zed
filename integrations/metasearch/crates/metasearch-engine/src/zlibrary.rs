//! Z-Library book search engine implementation.
//!
//! HTML scraping: <https://zlibrary-global.se/s/>
//! Website: https://zlibrary-global.se
//! Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;
use smallvec::smallvec;

const BASE_URL: &str = "https://z-lib.fm";

pub struct Zlibrary {
    metadata: EngineMetadata,
    client: Client,
}

impl Zlibrary {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "zlibrary".to_string().into(),
                display_name: "Z-Library".to_string().into(),
                homepage: BASE_URL.to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Zlibrary {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let page = query.page.max(1);

        let url = format!("{}/s/{}?page={}", BASE_URL, encoded, page);

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        if body.trim_start().starts_with('<') && body.contains("captcha") {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(&body);

        // Z-Library now uses <z-bookcard> web components with slot-based content
        let card_selector = Selector::parse("z-bookcard")
            .map_err(|e| MetasearchError::ParseError(format!("selector: {:?}", e)))?;
        let title_selector = Selector::parse("div[slot='title']")
            .map_err(|e| MetasearchError::ParseError(format!("selector: {:?}", e)))?;
        let author_selector = Selector::parse("div[slot='author']")
            .map_err(|e| MetasearchError::ParseError(format!("selector: {:?}", e)))?;

        let mut results = Vec::new();

        for (i, card) in document.select(&card_selector).enumerate() {
            let href = card.value().attr("href").unwrap_or_default();
            if href.is_empty() {
                continue;
            }

            let result_url = format!("{}{}", BASE_URL, href);

            let title = match card.select(&title_selector).next() {
                Some(el) => el.text().collect::<String>().trim().to_string(),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            let author = card
                .select(&author_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Supplement with metadata attributes
            let ext = card.value().attr("extension").unwrap_or_default();
            let year = card.value().attr("year").unwrap_or_default();
            let lang = card.value().attr("language").unwrap_or_default();

            let mut parts = Vec::new();
            if !author.is_empty() {
                parts.push(format!("by {author}"));
            }
            if !ext.is_empty() {
                parts.push(format!("[{ext}]"));
            }
            if !year.is_empty() && year != "0" {
                parts.push(year.to_string());
            }
            if !lang.is_empty() && lang.to_lowercase() != "english" {
                parts.push(lang.to_string());
            }
            let content = parts.join(" ");

            let mut r = SearchResult::new(&title, &result_url, &content, "zlibrary");
            r.engine_rank = i as u32;
            r.category = SearchCategory::Files.to_string();
            results.push(r);
        }

        info!(
            engine = "zlibrary",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
