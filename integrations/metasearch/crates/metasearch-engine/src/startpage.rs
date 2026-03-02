//! Startpage search engine implementation.
//!
//! HTML scraping via POST: <https://www.startpage.com/sp/search>
//! Website: https://www.startpage.com
//! Features: Web search, pagination, time range filtering

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

pub struct Startpage {
    metadata: EngineMetadata,
    client: Client,
}

impl Startpage {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "startpage".to_string().into(),
                display_name: "Startpage".to_string().into(),
                homepage: "https://www.startpage.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map time_range string to Startpage's `with_date` parameter.
    fn map_time_range(time_range: Option<&str>) -> Option<&'static str> {
        match time_range {
            Some("day") => Some("d"),
            Some("week") => Some("w"),
            Some("month") => Some("m"),
            Some("year") => Some("y"),
            _ => None,
        }
    }
}

#[async_trait]
impl SearchEngine for Startpage {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);

        // Build GET URL — POST causes 307 redirect that strips results
        let mut url = format!(
            "https://www.startpage.com/sp/search?query={}&page={}",
            urlencoding::encode(&query.query),
            page
        );

        if let Some(date_param) = Self::map_time_range(query.time_range.as_deref()) {
            url.push_str(&format!("&with_date={}", date_param));
        }

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .header("Accept", "text/html,application/xhtml+xml")
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

        let html_text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&html_text);

        let mut results = Vec::new();

        // Updated selectors for current Startpage HTML structure
        // Result containers use class "result css-..."
        let result_sel = Selector::parse(".result")
            .expect("selector should parse");
        let title_sel = Selector::parse(".wgl-title, h2, h3")
            .expect("selector should parse");
        let link_sel = Selector::parse("a[href]")
            .expect("link selector should parse");
        let desc_sel = Selector::parse(".result-snippet, p.description, p")
            .expect("selector should parse");

        for (i, container) in document.select(&result_sel).enumerate() {
            // Extract title
            let title_text = container
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if title_text.is_empty() {
                continue;
            }

            // Extract URL — look for href in links pointing to external sites
            let href = container
                .select(&link_sel)
                .find_map(|a| {
                    a.value()
                        .attr("data-url")
                        .or_else(|| a.value().attr("href"))
                        .filter(|h| h.starts_with("http") && !h.contains("startpage.com"))
                        .map(|h| h.to_string())
                })
                .unwrap_or_default();

            if href.is_empty() {
                continue;
            }

            // Extract description
            let content: String = container
                .select(&desc_sel)
                .next()
                .map(|el| {
                    el.text()
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            let mut r = SearchResult::new(&title_text, &href, &content, "startpage");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        Ok(results)
    }
}
