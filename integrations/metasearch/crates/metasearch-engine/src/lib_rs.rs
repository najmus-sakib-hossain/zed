//! lib.rs — search Rust crates on lib.rs.
//!
//! Scrapes HTML search results from lib.rs.
//!
//! Reference: <https://lib.rs>

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

pub struct LibRs {
    metadata: EngineMetadata,
    client: Client,
}

impl LibRs {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "lib_rs".to_string().into(),
                display_name: "lib.rs".to_string().into(),
                homepage: "https://lib.rs".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for LibRs {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://lib.rs/search?q={}",
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);
        let result_sel = Selector::parse("main div ol li a")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let title_sel = Selector::parse("div.h h4")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let content_sel = Selector::parse("div.h p")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let version_sel = Selector::parse("div.meta span.version, div.meta span.v")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let downloads_sel = Selector::parse("div.meta span.downloads")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (rank, item) in document.select(&result_sel).enumerate() {
            let href = item.value().attr("href").unwrap_or_default();
            let link = format!("https://lib.rs{}", href);

            let title = item
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let content = item
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let version = item
                .select(&version_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let downloads = item
                .select(&downloads_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let snippet = if version.is_empty() && downloads.is_empty() {
                content.clone()
            } else {
                format!("{} | v{} | {}", content, version.trim(), downloads.trim())
            };

            let mut result = SearchResult::new(title, link, snippet, self.metadata.name.clone());
            result.engine_rank = (rank + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
