//! DuckDuckGo search engine implementation.
//! Uses the HTML-lite POST endpoint at html.duckduckgo.com.

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
use smallvec::smallvec;

pub struct DuckDuckGo {
    metadata: EngineMetadata,
    client: Client,
}

impl DuckDuckGo {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "duckduckgo".to_string().into(),
                display_name: "DuckDuckGo".to_string().into(),
                homepage: "https://duckduckgo.com".to_string().into(),
                categories: smallvec![
                    SearchCategory::General,
                    SearchCategory::Images,
                    SearchCategory::News,
                ],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.2,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGo {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Use DuckDuckGo's HTML lite POST endpoint which doesn't require JS.
        // Pagination: s=0 (page 1), s=30 (page 2), s=60 (page 3), ...
        let offset = (query.page.saturating_sub(1)) * 30;
        let body = if offset == 0 {
            format!("q={}", urlencoding::encode(&query.query))
        } else {
            format!(
                "q={}&s={}&dc={}&v=l&o=json&api=/d.js",
                urlencoding::encode(&query.query),
                offset,
                offset + 1
            )
        };

        let resp = self
            .client
            .post("https://html.duckduckgo.com/html/")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Referer", "https://html.duckduckgo.com/")
            .header("Origin", "https://html.duckduckgo.com")
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "same-origin")
            .body(body)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text: String = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&html_text);

        // DDG-lite uses simpler class names: div.web-result > div.links_main
        let result_sel = Selector::parse("div.web-result").unwrap();
        let title_sel = Selector::parse("a.result__a").unwrap();
        let snippet_sel = Selector::parse("a.result__snippet").unwrap();

        let mut results = Vec::new();

        for (i, item) in document.select(&result_sel).enumerate() {
            // Get title and URL from the same <a> element
            let title_el = match item.select(&title_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title: String = title_el.text().collect();
            let title = title.trim().to_string();
            if title.is_empty() {
                continue;
            }

            // Get the href attribute directly
            let href = title_el.value().attr("href").unwrap_or_default();
            if href.is_empty() || href.starts_with("//duckduckgo.com") {
                continue;
            }

            // DDG-lite uses uddg parameter for the actual URL
            let canonical_url = if href.contains("uddg=") {
                // Extract URL from uddg parameter
                href.split("uddg=")
                    .nth(1)
                    .and_then(|s| urlencoding::decode(s).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| href.to_string())
            } else {
                href.to_string()
            };

            if canonical_url.is_empty() || !canonical_url.starts_with("http") {
                continue;
            }

            let snippet: String = item
                .select(&snippet_sel)
                .next()
                .map(|e| {
                    e.text()
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            let mut r = SearchResult::new(&title, &canonical_url, &snippet, "duckduckgo");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        Ok(results)
    }
}
