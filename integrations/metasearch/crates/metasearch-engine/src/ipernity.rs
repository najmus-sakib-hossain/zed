//! Ipernity engine — search photos on Ipernity.
//! Translated from SearXNG `searx/engines/ipernity.py`.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

const BASE_URL: &str = "https://www.ipernity.com";
const PAGE_SIZE: u32 = 10;

pub struct Ipernity {
    metadata: EngineMetadata,
    client: Client,
}

impl Ipernity {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "ipernity".to_string().into(),
                display_name: "Ipernity".to_string().into(),
                homepage: "https://www.ipernity.com".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Ipernity {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "{}/search/photo/@/page:{}:{}?q={}",
            BASE_URL,
            page,
            PAGE_SIZE,
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let img_sel = Selector::parse(r#"a[href^="/doc"] img"#)
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let link_sel = Selector::parse(r#"a[href^="/doc"]"#)
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (i, link_el) in document.select(&link_sel).enumerate() {
            let href = link_el.value().attr("href").unwrap_or_default();
            let result_url = format!("{}{}", BASE_URL, href);

            // Try to get the img inside this link
            let img_el = link_el.select(&img_sel).next();
            let thumbnail = img_el
                .and_then(|img| img.value().attr("src"))
                .map(|s| s.to_string());
            let img_src = thumbnail.as_ref().map(|t| t.replace("240.jpg", "640.jpg"));

            let title = img_el
                .and_then(|img| img.value().attr("alt"))
                .unwrap_or("Ipernity Photo")
                .to_string();

            let mut result =
                SearchResult::new(title, result_url, String::new(), "ipernity".to_string());
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            result.thumbnail = thumbnail;
            if let Some(src) = img_src {
                result.metadata = serde_json::json!({ "img_src": src });
            }
            results.push(result);
        }

        Ok(results)
    }
}
