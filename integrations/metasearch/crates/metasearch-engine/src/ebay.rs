//! eBay engine — search product listings via HTML scraping.
//! Translated from SearXNG `searx/engines/ebay.py`.

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

pub struct Ebay {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl Ebay {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "ebay".to_string().into(),
                display_name: "eBay".to_string().into(),
                homepage: "https://www.ebay.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: "https://www.ebay.com".to_string(),
        }
    }

    pub fn with_base_url(client: Client, base_url: &str) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "ebay".to_string().into(),
                display_name: "eBay".to_string().into(),
                homepage: base_url.to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.8,
            },
            client,
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for Ebay {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page;
        let url = format!(
            "{}/sch/i.html?_nkw={}&_sacat={}",
            self.base_url,
            urlencoding::encode(&query.query),
            page,
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

        let html_text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&html_text);
        let item_sel = Selector::parse("li.s-item").unwrap();
        let link_sel = Selector::parse("a.s-item__link").unwrap();
        let title_sel = Selector::parse("h3.s-item__title").unwrap();
        let price_sel = Selector::parse("span.s-item__price").unwrap();
        let thumb_sel = Selector::parse("img.s-item__image-img").unwrap();

        let mut results = Vec::new();

        for (i, item) in document.select(&item_sel).enumerate() {
            let title = item
                .select(&title_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            let item_url = item
                .select(&link_sel)
                .next()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or_default()
                .to_string();

            let price = item
                .select(&price_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            let thumbnail = item
                .select(&thumb_sel)
                .next()
                .and_then(|e| e.value().attr("src"))
                .map(|s| s.to_string());

            let snippet = if price.is_empty() {
                "eBay listing".to_string()
            } else {
                format!("Price: {}", price.trim())
            };

            let mut result = SearchResult::new(
                title.trim().to_string(),
                item_url,
                snippet,
                "ebay".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::General.to_string();
            result.thumbnail = thumbnail;
            results.push(result);
        }

        Ok(results)
    }
}
