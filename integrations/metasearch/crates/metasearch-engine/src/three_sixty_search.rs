//! 360 Search — Chinese web search engine.
//! Scrapes HTML results from so.com.
//!
//! Reference: <https://www.so.com>

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

pub struct ThreeSixtySearch {
    metadata: EngineMetadata,
    client: Client,
}

impl ThreeSixtySearch {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "360search".to_string().into(),
                display_name: "360 Search".to_string().into(),
                homepage: "https://www.so.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for ThreeSixtySearch {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page.max(1);
        let mut url = format!(
            "https://www.so.com/s?pn={}&q={}",
            page,
            urlencoding::encode(&query.query),
        );

        // Time range support
        if let Some(ref range) = query.time_range {
            let adv_t = match range.as_str() {
                "day" => Some("d"),
                "week" => Some("w"),
                "month" => Some("m"),
                "year" => Some("y"),
                _ => None,
            };
            if let Some(t) = adv_t {
                url.push_str(&format!("&adv_t={}", t));
            }
        }

        let resp = match self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", "https://www.so.com/")
            .send()
            .await
        {
            Ok(r) => r,
            // Geo-blocking or redirect loop → return empty gracefully
            Err(_) => return Ok(Vec::new()),
        };

        // Handle redirect detection (e.g., CAPTCHA or geo-block)
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&html_text);

        let item_sel =
            Selector::parse("li.res-list").unwrap_or_else(|_| Selector::parse("li").unwrap());
        let title_sel = Selector::parse("h3.res-title a")
            .unwrap_or_else(|_| Selector::parse("h3 a").unwrap());
        let content_sel = Selector::parse("p.res-desc, span.res-list-summary")
            .unwrap_or_else(|_| Selector::parse("p").unwrap());

        let mut results = Vec::new();

        for (i, item) in document.select(&item_sel).enumerate() {
            let title_el = match item.select(&title_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title: String = title_el.text().collect();
            let title = title.trim().to_string();
            if title.is_empty() {
                continue;
            }

            // Try data-mdurl first, then plain href
            let link = title_el
                .value()
                .attr("data-mdurl")
                .or_else(|| title_el.value().attr("href"))
                .unwrap_or_default()
                .to_string();

            if link.is_empty() {
                continue;
            }

            let content: String = item
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let mut result = SearchResult::new(&title, &link, &content, "360search");
            result.engine_rank = (i + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
