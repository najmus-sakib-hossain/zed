//! RightDao search engine implementation.
//!
//! Scrapes RightDao HTML results.  RightDao is a small, privacy-friendly
//! general-web search engine at https://rightdao.com.
//! Ported from integrations/metasearch2/src/engines/search/rightdao.rs.

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
use smallvec::smallvec;
use std::borrow::Cow;
use tracing::info;

pub struct RightDao {
    metadata: EngineMetadata,
    client: Client,
}

impl RightDao {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: Cow::Borrowed("rightdao"),
                display_name: Cow::Borrowed("RightDao"),
                homepage: Cow::Borrowed("https://rightdao.com"),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for RightDao {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://rightdao.com/search?q={}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/120.0.0.0 Safari/537.36",
            )
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
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

        let result_sel = Selector::parse("div.item").unwrap();
        let title_sel = Selector::parse("div.title").unwrap();
        let link_sel = Selector::parse("a[href]").unwrap();
        let desc_sel = Selector::parse("div.description").unwrap();

        for (i, element) in document.select(&result_sel).enumerate() {
            let title = element
                .select(&title_sel)
                .next()
                .map(|n| n.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let url = element
                .select(&link_sel)
                .next()
                .and_then(|n| n.value().attr("href"))
                .map(|s| s.to_string())
                .unwrap_or_default();

            let description = element
                .select(&desc_sel)
                .next()
                .map(|n| n.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if url.is_empty() || title.is_empty() {
                continue;
            }

            let mut r = SearchResult::new(&title, &url, &description, "rightdao");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(engine = "rightdao", count = results.len(), "Search complete");
        Ok(results)
    }
}
