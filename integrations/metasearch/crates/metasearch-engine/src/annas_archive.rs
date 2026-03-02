//! Anna's Archive — free shadow library metasearch for books and files.
//!
//! Reference: <https://annas-archive.org/>

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

pub struct AnnasArchive {
    metadata: EngineMetadata,
    client: Client,
}

impl AnnasArchive {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "annas_archive".to_string().into(),
                display_name: "Anna's Archive".to_string().into(),
                homepage: "https://annas-archive.li".to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for AnnasArchive {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = format!(
            "https://annas-archive.li/search?q={}&page={}",
            urlencoding::encode(&query.query),
            page
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
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

        let body: String = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);

        // Anna's Archive wraps each result in a div inside js-aarecord-list-outer
        let item_sel = Selector::parse("main div.js-aarecord-list-outer > div").unwrap();
        let link_sel = Selector::parse("a[href^='/md5']").unwrap();
        let desc_sel = Selector::parse("div.relative").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut results = Vec::new();

        for item in document.select(&item_sel) {
            let link_el = match item.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title = link_el.text().collect::<String>().trim().to_string();
            let href = link_el.value().attr("href").unwrap_or_default();

            if title.is_empty() || href.is_empty() {
                continue;
            }

            let content = item
                .select(&desc_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let thumbnail = item
                .select(&img_sel)
                .next()
                .and_then(|el| el.value().attr("src"))
                .map(|s| s.to_string());

            let full_url = format!("https://annas-archive.li{}", href);

            let mut result =
                SearchResult::new(title, full_url, content, "Anna's Archive".to_string());
            result.category = SearchCategory::Files.to_string();
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
