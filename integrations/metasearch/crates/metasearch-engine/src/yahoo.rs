//! Yahoo engine — search the web via Yahoo HTML scraping.
//! Translated from SearXNG `searx/engines/yahoo.py`.
//!
//! Yahoo wraps result URLs in a tracking redirect. This engine
//! extracts the real destination from the `/RU=…/RK` wrapper.

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

pub struct Yahoo {
    metadata: EngineMetadata,
    client: Client,
}

impl Yahoo {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "yahoo".to_string().into(),
                display_name: "Yahoo".to_string().into(),
                homepage: "https://search.yahoo.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

/// Remove Yahoo tracking wrapper from a URL.
///
/// Yahoo wraps URLs like:
///   `https://r.search.yahoo.com/…/RU=https%3a%2f%2fexample.com/RK=…/RS=…`
///
/// We extract the part between `/RU=` and the next `/RK` or `/RS`.
fn parse_url(url_string: &str) -> String {
    let ru_marker = "/RU=";
    let ru_pos = match url_string.find(ru_marker) {
        Some(pos) => pos + ru_marker.len(),
        None => return url_string.to_string(),
    };

    let remaining = &url_string[ru_pos..];

    // Find the earliest ending marker: /RS or /RK
    let end_markers = ["/RS", "/RK"];
    let end_pos = end_markers
        .iter()
        .filter_map(|marker| remaining.rfind(marker))
        .min();

    match end_pos {
        Some(pos) => {
            let encoded = &remaining[..pos];
            urlencoding::decode(encoded)
                .unwrap_or_else(|_| encoded.into())
                .to_string()
        }
        None => url_string.to_string(),
    }
}

#[async_trait]
impl SearchEngine for Yahoo {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;

        let mut url = format!(
            "https://search.yahoo.com/search?p={}",
            urlencoding::encode(&query.query),
        );

        // Pagination: page 1 uses iscqry='', page 2+ uses b offset
        if page == 1 {
            url.push_str("&iscqry=");
        } else {
            let offset = page * 7 + 1; // Yahoo uses 7 results per page: 8, 15, 22…
            url.push_str(&format!("&b={}&pz=7&bct=0&xargs=0", offset));
        }

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&html_text);

        // Yahoo result containers have class "algo-sr"
        let result_sel = Selector::parse("div.algo-sr").unwrap();
        // Title link — try two patterns Yahoo uses
        let title_link_sel = Selector::parse("div.compTitle h3 a").unwrap();
        let title_link_alt_sel = Selector::parse("div.compTitle a").unwrap();
        let content_sel = Selector::parse("div.compText").unwrap();

        let mut results = Vec::new();

        for (i, result) in document.select(&result_sel).enumerate() {
            // Try primary selector, fall back to alternate
            let link_el = result
                .select(&title_link_sel)
                .next()
                .or_else(|| result.select(&title_link_alt_sel).next());

            let link_el = match link_el {
                Some(el) => el,
                None => continue,
            };

            let raw_url = link_el.value().attr("href").unwrap_or_default();
            let clean_url = parse_url(raw_url);
            if clean_url.is_empty() {
                continue;
            }

            // Title: prefer aria-label, then inner text
            let title = link_el
                .value()
                .attr("aria-label")
                .map(|s| s.to_string())
                .unwrap_or_else(|| link_el.text().collect::<String>().trim().to_string());

            // Clean up title — remove extra whitespace
            let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");

            let content = result
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();
            let content: String = content.split_whitespace().collect::<Vec<_>>().join(" ");

            let mut sr = SearchResult::new(title, clean_url, content, "yahoo".to_string());
            sr.engine_rank = (i + 1) as u32;
            sr.category = SearchCategory::General.to_string();
            results.push(sr);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_with_tracking() {
        let tracked =
            "https://r.search.yahoo.com/_ylt=Awr.z/RU=https%3a%2f%2fexample.com%2fpage/RK=2/RS=abc";
        assert_eq!(parse_url(tracked), "https://example.com/page");
    }

    #[test]
    fn test_parse_url_without_tracking() {
        let plain = "https://example.com/page";
        assert_eq!(parse_url(plain), "https://example.com/page");
    }
}
