//! Google search engine implementation.
//!
//! Scrapes Google's web search results page. Based on SearXNG's `google.py`.
//! Uses async API with arc_id for better bot detection avoidance.

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

pub struct Google {
    metadata: EngineMetadata,
    client: Client,
}

impl Google {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "google".to_string().into(),
                display_name: "Google".to_string().into(),
                homepage: "https://www.google.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.5,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Google {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // URL and selectors ported directly from metasearch2/src/engines/search/google.rs
        let url = reqwest::Url::parse_with_params(
            "https://www.google.com/search",
            &[
                ("q", query.query.as_str()),
                ("nfpr", "1"),   // no autocorrect
                ("filter", "0"),
                ("start", "0"),
            ],
        )
        .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let resp = self
            .client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);
        let mut results = Vec::new();

        // metasearch2 selectors
        let result_sel = Selector::parse("[jscontroller=SC7lYd]").unwrap();
        let title_sel  = Selector::parse("h3").unwrap();
        let link_sel   = Selector::parse("a[href]").unwrap();
        let desc_sel   = Selector::parse("div[data-sncf='2'], div[data-sncf='1,2'], div[style='-webkit-line-clamp:2']").unwrap();

        for (i, element) in document.select(&result_sel).enumerate() {
            let title = element
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let raw_href = element
                .select(&link_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default();

            // Handle /url?q= redirect URLs
            let url = if raw_href.starts_with("/url?q=") {
                reqwest::Url::parse(&format!("https://www.google.com{raw_href}"))
                    .ok()
                    .and_then(|u| {
                        u.query_pairs()
                            .find(|(k, _)| k == "q")
                            .map(|(_, v)| v.to_string())
                    })
                    .unwrap_or_else(|| raw_href.to_string())
            } else {
                raw_href.to_string()
            };

            let description = element
                .select(&desc_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if title.is_empty() || url.is_empty() {
                continue;
            }

            let mut r = SearchResult::new(&title, &url, &description, "google");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(engine = "google", count = results.len(), "Search complete");
        Ok(results)
    }
}
