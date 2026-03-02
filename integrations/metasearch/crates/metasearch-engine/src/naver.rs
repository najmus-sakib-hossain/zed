//! Naver — Korean search engine.
//!
//! Scrapes HTML search results from search.naver.com.
//! The page uses NAVER Soro Design System (sds-comps) classes.
//!
//! Reference: <https://search.naver.com>

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

use metasearch_core::category::SearchCategory;
use metasearch_core::engine::{EngineMetadata, SearchEngine};
use metasearch_core::error::Result;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use smallvec::smallvec;

const BASE_URL: &str = "https://search.naver.com";
const RESULTS_PER_PAGE: u32 = 15;

pub struct Naver {
    metadata: EngineMetadata,
    client: Client,
}

impl Naver {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "naver".to_string().into(),
                display_name: "Naver".to_string().into(),
                homepage: BASE_URL.to_string().into(),
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
impl SearchEngine for Naver {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let start = (query.page.saturating_sub(1)) * RESULTS_PER_PAGE + 1;

        let url = format!(
            "{}/search.naver?query={}&where=web&start={}",
            BASE_URL,
            urlencoding::encode(&query.query),
            start,
        );

        let resp = match self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .header("Accept-Language", "ko-KR,ko;q=0.9,en;q=0.8")
            .timeout(std::time::Duration::from_secs(7))
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
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);

        // Naver now uses NAVER SDS (Soro Design System):
        // - Result links: a[nocr="1"][target="_blank"] with external URLs
        // - Title text: child element with class containing "sds-comps-profile-info-title"
        // - Description: text in child with class containing "fds-web-desc" or similar
        let link_sel = match Selector::parse("a[nocr='1'][target='_blank']") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let title_sel = Selector::parse("[class*='sds-comps-profile-info-title']").ok();
        let desc_sel = Selector::parse("[class*='fds-web-desc'], [class*='sds-comps-text-type-body3']").ok();

        let mut results = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        for (i, link) in document.select(&link_sel).enumerate() {
            let href = link.value().attr("href").unwrap_or_default();

            // Only include external (non-naver) URLs
            if href.is_empty()
                || !href.starts_with("http")
                || href.contains("naver.com")
                || href.contains("naver.net")
            {
                continue;
            }

            // Deduplicate (same result may appear with anchor variants)
            let base_url = href.split('#').next().unwrap_or(href);
            if seen_urls.contains(base_url) {
                continue;
            }
            seen_urls.insert(base_url.to_string());

            // Title: prefer text from the title div, fall back to full link text
            let title = if let Some(ref sel) = title_sel {
                link.select(sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| link.text().collect::<String>().trim().to_string())
            } else {
                link.text().collect::<String>().trim().to_string()
            };

            if title.is_empty() {
                continue;
            }

            // Description: look for nearby description text
            let content = if let Some(ref sel) = desc_sel {
                link.select(sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let mut result = SearchResult::new(title, href.to_string(), content, "naver");
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::General.to_string();
            results.push(result);

            if results.len() >= 10 {
                break;
            }
        }

        Ok(results)
    }
}
