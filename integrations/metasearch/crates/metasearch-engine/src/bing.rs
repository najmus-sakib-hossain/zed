//! Bing search engine implementation.
//!
//! Translated from SearXNG's `bing.py` (279 lines, HTML scraping).
//! Bing is Microsoft's web search engine supporting general, images, news, and videos.
//! Website: https://www.bing.com
//! Features: Paging, SafeSearch, Time Range

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;
use smallvec::smallvec;
use base64::{Engine as _, engine::general_purpose};

pub struct Bing {
    metadata: EngineMetadata,
    client: Client,
}

impl Bing {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "bing".to_string().into(),
                display_name: "Bing".to_string().into(),
                homepage: "https://www.bing.com".to_string().into(),
                categories: smallvec![
                    SearchCategory::General,
                    SearchCategory::News,
                    SearchCategory::Images,
                ],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.4,
            },
            client,
        }
    }

    /// Map time_range to Bing's `filters` query parameter.
    fn time_range_param(time_range: Option<&str>) -> Option<String> {
        match time_range {
            Some("day") => Some("ex1%3a%22ez1%22".to_string()),
            Some("week") => Some("ex1%3a%22ez2%22".to_string()),
            Some("month") => Some("ex1%3a%22ez3%22".to_string()),
            Some("year") => Some("ex1%3a%22ez5_19868_20133%22".to_string()),
            _ => None,
        }
    }

    /// Map safe_search level to Bing's cookie.
    fn safesearch_cookie(level: u8) -> &'static str {
        match level {
            2 => "STRICT",
            1 => "DEMOTE",
            _ => "OFF",
        }
    }
}

#[async_trait]
impl SearchEngine for Bing {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let page = query.page.max(1);
        let offset = (page - 1) * 10 + 1;
        let encoded_query = urlencoding::encode(&query.query);

        let mut url = format!(
            "https://www.bing.com/search?q={}&pq={}&first={}&setlang=en&setmkt=en-US",
            encoded_query,
            encoded_query,  // pq parameter needed for correct pagination
            offset
        );

        // Add FORM parameter for pagination (like SearXNG)
        if page == 2 {
            url.push_str("&FORM=PERE");
        } else if page > 2 {
            url.push_str(&format!("&FORM=PERE{}", page - 2));
        }

        if let Some(filter) = Self::time_range_param(query.time_range.as_deref()) {
            url.push_str(&format!("&filters={}", filter));
        }

        let lang = query.language.as_deref().unwrap_or("en");
        let region = "US";  // Could be made configurable
        let safesearch = Self::safesearch_cookie(query.safe_search);

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", format!("{},en;q=0.9", lang))
            .header("Accept-Encoding", "gzip, deflate, br")
            // SearXNG uses _EDGE_CD and _EDGE_S cookies for region/language
            .header("Cookie", format!("SRCHHPGUSR=ADLT={}; _EDGE_CD=m={}&u={}; _EDGE_S=mkt={}&ui={}", 
                safesearch, region, lang, region, lang))
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&body);

        // Bing organic results are in <li class="b_algo">
        let result_selector = Selector::parse("li.b_algo").unwrap();
        let title_selector = Selector::parse("h2 a").unwrap();
        let snippet_selector = Selector::parse("p, .b_caption p, .b_lineclamp2, .b_lineclamp4").unwrap();

        let mut results = Vec::new();

        for (i, element) in document.select(&result_selector).enumerate() {
            let title_el = match element.select(&title_selector).next() {
                Some(el) => el,
                None => continue,
            };

            let title: String = title_el.text().collect::<String>().trim().to_string();
            let mut result_url = match title_el.value().attr("href") {
                Some(href) => href.to_string(),
                None => continue,
            };

            // Handle Bing redirect URLs (like SearXNG)
            if result_url.starts_with("https://www.bing.com/ck/a?") {
                // Decode base64 URL from u parameter
                if let Some(u_param) = result_url.split("&u=").nth(1) {
                    if let Some(encoded) = u_param.split('&').next() {
                        // Remove "a1" prefix and decode
                        if encoded.len() > 2 {
                            let encoded_url = &encoded[2..];
                            // Add padding for base64
                            let padding = (4 - (encoded_url.len() % 4)) % 4;
                            let padded = format!("{}{}", encoded_url, "=".repeat(padding));
                            if let Ok(decoded_bytes) = general_purpose::URL_SAFE.decode(&padded) {
                                if let Ok(decoded_url) = String::from_utf8(decoded_bytes) {
                                    result_url = decoded_url;
                                }
                            }
                        }
                    }
                }
            }

            if !result_url.starts_with("http") {
                continue;
            }

            let snippet: String = element
                .select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let mut r = SearchResult::new(&title, &result_url, &snippet, "bing");
            r.engine_rank = i as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(engine = "bing", count = results.len(), "Search complete");
        Ok(results)
    }
}
