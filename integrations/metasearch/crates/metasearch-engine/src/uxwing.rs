//! UXWing — search for free icons via HTML scraping.
//!
//! Reference: <https://uxwing.com>

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

pub struct Uxwing {
    metadata: EngineMetadata,
    client: Client,
}

impl Uxwing {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "uxwing".to_string().into(),
                display_name: "UXWing".to_string().into(),
                homepage: "https://uxwing.com".to_string().into(),
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
impl SearchEngine for Uxwing {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://uxwing.com/?s={}",
            urlencoding::encode(&query.query),
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let document = Html::parse_document(&body);

        let article_sel = Selector::parse("article[id^='post-']")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let a_sel = Selector::parse("a[href]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let img_sel = Selector::parse("img[src]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (i, article) in document.select(&article_sel).enumerate() {
            let anchor = match article.select(&a_sel).next() {
                Some(a) => a,
                None => continue,
            };

            let href = anchor.value().attr("href").unwrap_or_default();
            if href.is_empty() {
                continue;
            }

            let img = match article.select(&img_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let img_src = img.value().attr("src").unwrap_or_default().to_string();
            let title = img
                .value()
                .attr("alt")
                .unwrap_or("Icon")
                .to_string();

            let mut result = SearchResult::new(
                title,
                href.to_string(),
                String::new(),
                self.metadata.name.clone(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Images.to_string();
            if !img_src.is_empty() {
                result.thumbnail = Some(img_src);
            }
            results.push(result);
        }

        Ok(results)
    }
}
