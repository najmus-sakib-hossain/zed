//! ANSA (ansa.it) news search engine.
//!
//! ANSA is Italy's oldest and largest news agency. This engine scrapes
//! search results from their public search page.

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::MetasearchError;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const BASE_URL: &str = "https://www.ansa.it";
const SEARCH_URL: &str = "https://www.ansa.it/ricerca/ansait/search.shtml";
const PAGE_SIZE: u32 = 12;

pub struct Ansa {
    metadata: EngineMetadata,
    client: Client,
}

impl Ansa {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "ansa".to_string().into(),
                display_name: "ANSA".to_string().into(),
                homepage: BASE_URL.to_string().into(),
                categories: smallvec![SearchCategory::News],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Ansa {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let start = (page.saturating_sub(1)) * PAGE_SIZE;

        let mut url =
            Url::parse(SEARCH_URL).map_err(|e| MetasearchError::ParseError(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("any", &query.query)
            .append_pair("start", &start.to_string())
            .append_pair("sort", "data:desc");

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&resp);
        let article_sel = Selector::parse("div.article").unwrap();
        let title_sel = Selector::parse("div.content h2.title a").unwrap();
        let text_sel = Selector::parse("div.content div.text").unwrap();

        let mut results = Vec::new();

        for article in document.select(&article_sel) {
            let title_el = match article.select(&title_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title: String = title_el
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let href = title_el.value().attr("href").unwrap_or_default();
            let article_url = format!("{}{}", BASE_URL, href);

            let content: String = article
                .select(&text_sel)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            results.push(SearchResult::new(title, article_url, content, "ANSA"));
        }

        Ok(results)
    }
}
