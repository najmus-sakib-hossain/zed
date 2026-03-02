//! Emojipedia search engine.
//!
//! Emojipedia is an emoji reference website documenting the meaning
//! and common usage of emoji characters in the Unicode Standard.

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::Result;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const BASE_URL: &str = "https://emojipedia.org";

pub struct Emojipedia {
    metadata: EngineMetadata,
    client: Client,
}

impl Emojipedia {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "emojipedia".to_string().into(),
                display_name: "Emojipedia".to_string().into(),
                homepage: BASE_URL.to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Emojipedia {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let mut url = Url::parse(&format!("{}/search", BASE_URL)).unwrap();
        url.query_pairs_mut().append_pair("q", &query.query);

        let resp = match self
            .client
            .get(url.as_str())
            .timeout(std::time::Duration::from_secs(7))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let resp = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&resp);

        // Emojipedia uses div class starting with "EmojisList" containing <a> links
        let container_sel = Selector::parse("div[class^='EmojisList'] a").unwrap();

        let mut results = Vec::new();

        for element in document.select(&container_sel) {
            let href = element.value().attr("href").unwrap_or_default();
            let title: String = element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            if title.is_empty() || href.is_empty() {
                continue;
            }

            let emoji_url = format!("{}{}", BASE_URL, href);

            results.push(SearchResult::new(title, emoji_url, "", "Emojipedia"));
        }

        Ok(results)
    }
}
