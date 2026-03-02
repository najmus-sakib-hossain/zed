//! DokuWiki search — configurable instance URL, HTML scraping.
//! SearXNG equivalent: `doku.py`
//!
//! DokuWiki is an open-source wiki that uses plain text files.
//! This engine scrapes the built-in search page.

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct Doku {
    client: Client,
    base_url: String,
}

impl Doku {
    pub fn new(client: Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for Doku {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "dokuwiki".to_string().into(),
            display_name: "DokuWiki".to_string().into(),
            homepage: "https://www.dokuwiki.org".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.base_url.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/?do=search&id={}",
            self.base_url,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("DokuWiki: {e}")))?;

        let text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("DokuWiki body: {e}")))?;

        let doc = Html::parse_document(&text);

        // Quick hits
        let quickhit_sel = Selector::parse("div.search_quickresult ul li").unwrap();
        let link_sel = Selector::parse("a.wikilink1").unwrap();

        // Full search results (dt/dd pairs)
        let dt_sel = Selector::parse("dl.search_results dt").unwrap();
        let dd_sel = Selector::parse("dl.search_results dd").unwrap();

        let mut results = Vec::new();
        let mut rank = 1u32;

        // Parse quick hits
        for el in doc.select(&quickhit_sel) {
            if let Some(a) = el.select(&link_sel).next() {
                let title = a.value().attr("title").unwrap_or("").to_string();
                let href = a.value().attr("href").unwrap_or("").to_string();
                let full_url = if href.starts_with("http") {
                    href
                } else {
                    format!("{}{}", self.base_url, href)
                };
                results.push(SearchResult {
                    title,
                    url: full_url,
                    content: String::new(),
                    engine: "DokuWiki".to_string(),
                    engine_rank: rank,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });
                rank += 1;
            }
        }

        // Parse full search results
        let dts: Vec<_> = doc.select(&dt_sel).collect();
        let dds: Vec<_> = doc.select(&dd_sel).collect();

        for (dt, dd) in dts.iter().zip(dds.iter()) {
            if let Some(a) = dt.select(&link_sel).next() {
                let title = a.value().attr("title").unwrap_or("").to_string();
                let href = a.value().attr("href").unwrap_or("").to_string();
                let full_url = if href.starts_with("http") {
                    href
                } else {
                    format!("{}{}", self.base_url, href)
                };
                let content = dd.text().collect::<String>();

                results.push(SearchResult {
                    title,
                    url: full_url,
                    content,
                    engine: "DokuWiki".to_string(),
                    engine_rank: rank,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });
                rank += 1;
            }
        }

        Ok(results)
    }
}
