//! Geizhals — European price-comparison search engine.
//!
//! Scrapes HTML results from geizhals.de.

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

pub struct Geizhals {
    metadata: EngineMetadata,
    client: Client,
}

impl Geizhals {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "geizhals".to_string().into(),
                display_name: "Geizhals".to_string().into(),
                homepage: "https://geizhals.de".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Geizhals {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://geizhals.de/?fs={}&hloc=at&hloc=de&in=",
            urlencoding::encode(&query.query)
        );

        let resp = match self
            .client
            .get(&url)
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

        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);
        let item_sel = Selector::parse("div.listview__item, article.listview__item").unwrap();
        let link_sel = Selector::parse("a.listview__name-link").unwrap();
        let price_sel = Selector::parse("span.price").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut results = Vec::new();

        for (rank, item) in document.select(&item_sel).enumerate() {
            let link_el = match item.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title = link_el.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }

            let href = match link_el.value().attr("href") {
                Some(h) => {
                    if h.starts_with("http") {
                        h.to_string()
                    } else {
                        format!("https://geizhals.de{h}")
                    }
                }
                None => continue,
            };

            let price = item
                .select(&price_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let snippet = if price.is_empty() {
                String::new()
            } else {
                format!("Price: {price}")
            };

            let thumbnail = item
                .select(&img_sel)
                .next()
                .and_then(|el| el.value().attr("src"))
                .map(|s| s.to_string());

            let mut result = SearchResult::new(title, href, snippet, self.metadata.name.clone());
            result.engine_rank = (rank + 1) as u32;
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
