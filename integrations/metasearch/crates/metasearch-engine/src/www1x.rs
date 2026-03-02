//! 1x.com — photography search via XML/HTML backend.
//!
//! Reference: <https://1x.com>

use async_trait::async_trait;
use regex::Regex;
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

pub struct Www1x {
    metadata: EngineMetadata,
    client: Client,
}

impl Www1x {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "www1x".to_string().into(),
                display_name: "1x".to_string().into(),
                homepage: "https://1x.com".to_string().into(),
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
impl SearchEngine for Www1x {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://1x.com/backend/search.php?q={}",
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

        // Response is XML with a <data> element containing HTML.
        // Extract the text content between <data> tags.
        let data_re = Regex::new(r"(?s)<data>(.*?)</data>")
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let html_fragment = match data_re.captures(&body) {
            Some(caps) => caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
            None => return Ok(Vec::new()),
        };

        if html_fragment.is_empty() {
            return Ok(Vec::new());
        }

        let document = Html::parse_fragment(&html_fragment);

        let a_sel = Selector::parse("a[href]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;
        let img_sel = Selector::parse("img[src]")
            .map_err(|e| MetasearchError::ParseError(format!("{e:?}")))?;

        let mut results = Vec::new();

        for (i, anchor) in document.select(&a_sel).enumerate() {
            let href = anchor.value().attr("href").unwrap_or_default();
            if href.is_empty() {
                continue;
            }

            let item_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://1x.com{}", href)
            };

            let img = match anchor.select(&img_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let img_src = img.value().attr("src").unwrap_or_default();
            let thumbnail = if img_src.is_empty() {
                String::new()
            } else if img_src.starts_with("http") {
                img_src.to_string()
            } else {
                format!("https://gallery.1x.com/{}", img_src.trim_start_matches('/'))
            };

            let title = img
                .value()
                .attr("alt")
                .map(|s| s.to_string())
                .or_else(|| {
                    let text = anchor.text().collect::<String>();
                    let trimmed = text.trim().to_string();
                    if trimmed.is_empty() { None } else { Some(trimmed) }
                })
                .unwrap_or_else(|| "1x Photo".to_string());

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
