//! Sogou search engine implementation.
//!
//! Translated from SearXNG's `sogou.py` (HTML scraping).
//! Sogou is a major Chinese search engine.
//! Website: https://www.sogou.com
//! Features: Paging, Time Range

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{info, warn};
use smallvec::smallvec;

pub struct Sogou {
    metadata: EngineMetadata,
    client: Client,
}

impl Sogou {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "sogou".to_string().into(),
                display_name: "Sogou".to_string().into(),
                homepage: "https://www.sogou.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map time_range to Sogou's s_from parameter.
    fn time_range_param(time_range: Option<&str>) -> Option<&'static str> {
        match time_range {
            Some("day") => Some("inttime_day"),
            Some("week") => Some("inttime_week"),
            Some("month") => Some("inttime_month"),
            Some("year") => Some("inttime_year"),
            _ => None,
        }
    }
}

#[async_trait]
impl SearchEngine for Sogou {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let encoded = urlencoding::encode(&query.query);

        let mut url = format!("https://www.sogou.com/web?query={}&page={}", encoded, page);

        if let Some(tr) = Self::time_range_param(query.time_range.as_deref()) {
            url.push_str(&format!("&s_from={}&tsn=1", tr));
        }

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        // Check for captcha/anti-spider redirects
        if resp.status().as_u16() == 302 {
            if let Some(location) = resp.headers().get("location") {
                if let Ok(loc_str) = location.to_str() {
                    if loc_str.contains("antispider") {
                        warn!(engine = "sogou", "Anti-spider captcha detected");
                        return Err(MetasearchError::EngineError {
                            engine: "sogou".to_string(),
                            message: "Captcha required".to_string(),
                        });
                    }
                }
            }
        }

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);
        let mut results = Vec::new();

        // Standard results: div.rb with h3.pt > a
        let rb_sel = Selector::parse("div.rb").unwrap();
        let title_sel = Selector::parse("h3.pt a").unwrap();
        let content_sel = Selector::parse("div.ft").unwrap();
        let cite_sel = Selector::parse("cite").unwrap();

        // VR-style results: div.vrwrap with h3.vr-title > a
        let vr_sel = Selector::parse("div.vrwrap").unwrap();
        let vr_title_sel = Selector::parse("h3.vr-title a, h3[class*=\"vr-title\"] a").unwrap();
        let vr_content_sel =
            Selector::parse("div[class*=\"attribute-centent\"], div[class*=\"fz-mid\"]").unwrap();

        let date_re = Regex::new(r"(\d{4}-\d{1,2}-\d{1,2})").unwrap();
        let data_url_re = Regex::new(r#"data-url="([^"]+)"#).unwrap();

        let mut rank: u32 = 0;

        // Parse standard results
        for item in document.select(&rb_sel) {
            if let Some(title_el) = item.select(&title_sel).next() {
                let title: String = title_el.text().collect::<String>().trim().to_string();
                let mut item_url = title_el
                    .value()
                    .attr("href")
                    .unwrap_or_default()
                    .to_string();

                // Resolve /link?url= redirects via data-url attribute
                if item_url.starts_with("/link?url=") {
                    let item_html = item.html();
                    if let Some(caps) = data_url_re.captures(&item_html) {
                        item_url = caps[1].to_string();
                    } else {
                        item_url = format!("https://www.sogou.com{}", item_url);
                    }
                }

                let content: String = item
                    .select(&content_sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                if title.is_empty() || item_url.is_empty() {
                    continue;
                }

                let mut r = SearchResult::new(&title, &item_url, &content, "sogou");
                r.engine_rank = rank;
                r.category = SearchCategory::General.to_string();

                // Try to parse date from cite element
                if let Some(cite_el) = item.select(&cite_sel).next() {
                    let cite_text: String = cite_el.text().collect();
                    if let Some(caps) = date_re.captures(&cite_text) {
                        if let Ok(date) = chrono::NaiveDate::parse_from_str(&caps[1], "%Y-%m-%d") {
                            let dt = date.and_hms_opt(0, 0, 0).unwrap();
                            r.published_date =
                                chrono::DateTime::from_timestamp(dt.and_utc().timestamp(), 0);
                        }
                    }
                }

                results.push(r);
                rank += 1;
            }
        }

        // Parse VR-style results
        for item in document.select(&vr_sel) {
            // Skip special wraps
            if item.html().contains("special-wrap") {
                continue;
            }

            if let Some(title_el) = item.select(&vr_title_sel).next() {
                let title: String = title_el.text().collect::<String>().trim().to_string();
                let mut item_url = title_el
                    .value()
                    .attr("href")
                    .unwrap_or_default()
                    .to_string();

                if item_url.starts_with("/link?url=") {
                    let item_html = item.html();
                    if let Some(caps) = data_url_re.captures(&item_html) {
                        item_url = caps[1].to_string();
                    } else {
                        item_url = format!("https://www.sogou.com{}", item_url);
                    }
                }

                let content: String = item
                    .select(&vr_content_sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                if title.is_empty() || item_url.is_empty() {
                    continue;
                }

                let mut r = SearchResult::new(&title, &item_url, &content, "sogou");
                r.engine_rank = rank;
                r.category = SearchCategory::General.to_string();
                results.push(r);
                rank += 1;
            }
        }

        info!(engine = "sogou", count = results.len(), "Search complete");
        Ok(results)
    }
}
