//! Brave Search engine implementation.
//! Selectors ported from metasearch2/src/engines/search/brave.rs.

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
use std::borrow::Cow;
use tracing::info;
use smallvec::smallvec;

#[allow(dead_code)]
pub struct Brave {
    metadata: EngineMetadata,
    client: Client,
    api_key: Option<String>,
}

impl Brave {
    pub fn new(client: Client, api_key: Option<String>) -> Self {
        Self {
            metadata: EngineMetadata {
                name: Cow::Borrowed("brave"),
                display_name: Cow::Borrowed("Brave Search"),
                homepage: Cow::Borrowed("https://search.brave.com"),
                categories: smallvec![SearchCategory::General, SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.3,
            },
            client,
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for Brave {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // URL and selectors ported directly from metasearch2/src/engines/search/brave.rs
        let url = reqwest::Url::parse_with_params(
            "https://search.brave.com/search",
            &[("q", query.query.as_str())],
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
        let result_sel = Selector::parse("#results > .snippet[data-pos]:not(.standalone)").unwrap();
        let title_sel  = Selector::parse(".title").unwrap();
        let link_sel   = Selector::parse("a").unwrap();
        let desc_sel   = Selector::parse(".generic-snippet, .video-snippet > .snippet-description").unwrap();

        for (i, element) in document.select(&result_sel).enumerate() {
            let title = element
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let url = element
                .select(&link_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default()
                .to_string();

            let description = element
                .select(&desc_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if title.is_empty() || url.is_empty() {
                continue;
            }

            let mut r = SearchResult::new(&title, &url, &description, "brave");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(engine = "brave", count = results.len(), "Search complete");
        Ok(results)
    }
}
