//! BTDigg torrent search engine.
//!
//! Scrapes btdig.com for torrent metadata via HTML parsing.

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

pub struct Btdigg {
    metadata: EngineMetadata,
    client: Client,
}

impl Btdigg {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "btdigg".to_string().into(),
                display_name: "BTDigg".to_string().into(),
                homepage: "https://btdig.com".to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Btdigg {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let search_term = urlencoding::encode(&query.query);
        let page = query.page.saturating_sub(1); // 0-indexed
        let url = format!("https://btdig.com/search?q={}&p={}", search_term, page);

        let resp = match self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
            .timeout(std::time::Duration::from_secs(7))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let body = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);
        let result_sel = Selector::parse("div.one_result").unwrap();
        let name_sel = Selector::parse("div.torrent_name a").unwrap();
        let excerpt_sel = Selector::parse("div.torrent_excerpt").unwrap();
        let size_sel = Selector::parse("span.torrent_size").unwrap();
        let magnet_sel = Selector::parse("div.torrent_magnet a").unwrap();

        let mut results = Vec::new();

        for element in document.select(&result_sel) {
            let title = element
                .select(&name_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let href = element
                .select(&name_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|h| {
                    if h.starts_with("http") {
                        h.to_string()
                    } else {
                        format!("https://btdig.com{}", h)
                    }
                })
                .unwrap_or_default();

            let content = element
                .select(&excerpt_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().replace('\n', " | "))
                .unwrap_or_default();

            let filesize = element
                .select(&size_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let _magnet = element
                .select(&magnet_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default();

            let snippet = if filesize.is_empty() {
                content.clone()
            } else {
                format!("Size: {} | {}", filesize, content)
            };

            if !title.is_empty() && !href.is_empty() {
                let mut result = SearchResult::new(&title, &href, &snippet, "btdigg");
                result.category = SearchCategory::Files.to_string();
                results.push(result);
            }
        }

        Ok(results)
    }
}
