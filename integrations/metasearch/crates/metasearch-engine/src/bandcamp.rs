//! Bandcamp search engine implementation.
//!
//! Translated from SearXNG's `bandcamp.py` (81 lines, HTML scraping).
//! Bandcamp is an online music platform for artists to share and sell music.
//! Website: https://bandcamp.com/
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

pub struct Bandcamp {
    metadata: EngineMetadata,
    client: Client,
}

impl Bandcamp {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bandcamp".to_string().into(),
                display_name: "Bandcamp".to_string().into(),
                homepage: "https://bandcamp.com".to_string().into(),
                categories: smallvec![SearchCategory::Music],
                enabled: true,
                timeout_ms: 4000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Bandcamp {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let page = query.page.max(1);

        let url = format!("https://bandcamp.com/search?q={}&page={}", encoded, page);

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let result_selector = Selector::parse("li.searchresult").unwrap();
        let link_selector = Selector::parse(".itemurl a").unwrap();
        let title_selector = Selector::parse(".heading a").unwrap();
        let subhead_selector = Selector::parse(".subhead").unwrap();
        let art_selector = Selector::parse(".art img").unwrap();
        let itemtype_selector = Selector::parse(".itemtype").unwrap();

        let mut results = Vec::new();

        for (i, element) in document.select(&result_selector).enumerate() {
            let link_el = match element.select(&link_selector).next() {
                Some(el) => el,
                None => continue,
            };
            let result_url: String = link_el.text().collect::<String>().trim().to_string();

            let title: String = element
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let content: String = element
                .select(&subhead_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let thumbnail: Option<String> = element
                .select(&art_selector)
                .next()
                .and_then(|el| el.value().attr("src").map(|s| s.to_string()));

            let item_type: String = element
                .select(&itemtype_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_lowercase())
                .unwrap_or_default();

            let snippet = if item_type.is_empty() {
                content
            } else {
                format!("[{}] {}", item_type, content)
            };

            if !title.is_empty() && !result_url.is_empty() {
                let mut r = SearchResult::new(&title, &result_url, &snippet, "bandcamp");
                r.engine_rank = i as u32;
                r.category = SearchCategory::Music.to_string();
                r.thumbnail = thumbnail;
                results.push(r);
            }
        }

        info!(
            engine = "bandcamp",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
