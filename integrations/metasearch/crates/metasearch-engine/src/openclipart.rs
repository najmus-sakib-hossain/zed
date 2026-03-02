//! OpenClipart — search for clipart images via HTML scraping.
//!
//! Reference: <https://openclipart.org>

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};

pub struct Openclipart {
    metadata: EngineMetadata,
    client: Client,
}

impl Openclipart {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "openclipart".to_string().into(),
                display_name: "OpenClipart".to_string().into(),
                homepage: "https://openclipart.org".to_string().into(),
                categories: smallvec![SearchCategory::Images],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Openclipart {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://openclipart.org/search/?query={}&p={}",
            urlencoding::encode(&query.query),
            query.page,
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .send()
            .await
        {
            Ok(r) => r,
            // Site may be slow or down
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let gallery_sel = Selector::parse("div.gallery div.artwork")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let a_sel = Selector::parse("a[href]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let img_sel = Selector::parse("a img")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (i, item) in document.select(&gallery_sel).enumerate() {
            let anchor = match item.select(&a_sel).next() {
                Some(a) => a,
                None => continue,
            };

            let href = anchor.value().attr("href").unwrap_or_default();
            if href.is_empty() {
                continue;
            }

            let item_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://openclipart.org{}", href)
            };

            let img = match item.select(&img_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title = img
                .value()
                .attr("alt")
                .unwrap_or("Clipart")
                .to_string();

            let thumbnail = img.value().attr("src").unwrap_or_default().to_string();

            let mut result = SearchResult::new(
                title,
                item_url,
                String::new(),
                self.metadata.name.clone(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            if !thumbnail.is_empty() {
                result.thumbnail = Some(thumbnail);
            }
            results.push(result);
        }

        Ok(results)
    }
}
