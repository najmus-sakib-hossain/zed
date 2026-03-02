//! Seznam (Czech search engine) implementation.
//!
//! HTML scraping: <https://search.seznam.cz/>
//! Website: https://www.seznam.cz
//! Features: Web search, no pagination (first page only)

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

pub struct Seznam {
    metadata: EngineMetadata,
    client: Client,
}

impl Seznam {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "seznam".to_string().into(),
                display_name: "Seznam".to_string().into(),
                homepage: "https://www.seznam.cz".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Seznam {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://search.seznam.cz/?q={}",
            urlencoding::encode(&query.query)
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(6))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "cs,en;q=0.9")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let html_text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&html_text);

        let mut results = Vec::new();

        // Use stable data-e-a="heading" attribute on result anchors
        // Falls back to generic h3 a if that fails
        let result_sel = Selector::parse("a[data-e-a='heading']")
            .or_else(|_| Selector::parse("[data-dot-data] h3 a"))
            .or_else(|_| Selector::parse("h3 a"))
            .unwrap_or_else(|_| Selector::parse("a").expect("basic selector should parse"));

        for (i, link) in document.select(&result_sel).enumerate() {
            let href = link.value().attr("href").unwrap_or_default();
            if href.is_empty()
                || href.starts_with('#')
                || href.starts_with("javascript:")
                || href.starts_with("https://search.seznam.cz")
                || href.starts_with("//search.seznam.cz")
                || href.starts_with("https://napoveda.seznam.cz")
                || href.starts_with("https://o-seznam.cz")
            {
                continue;
            }

            let title: String = link.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }

            let result_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://search.seznam.cz{}", href)
            };

            // Try to find description text near the link
            // Walk up to the parent and look for description text
            let content = String::new();

            let mut r = SearchResult::new(&title, &result_url, &content, "seznam");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);

            if results.len() >= 10 {
                break;
            }
        }

        // If strategy above didn't yield results, try a broader approach
        if results.is_empty() {
            let broad_sel = Selector::parse("div.Result")
                .or_else(|_| Selector::parse("[class*='Result']"))
                .unwrap_or_else(|_| Selector::parse("div").expect("div selector should parse"));
            let link_sel =
                Selector::parse("h3 a, a[href]").unwrap_or_else(|_| {
                    Selector::parse("a").expect("basic selector should parse")
                });
            let desc_sel = Selector::parse("p, .description, [class*='desc'], span")
                .unwrap_or_else(|_| {
                    Selector::parse("span").expect("span selector should parse")
                });

            for (i, container) in document.select(&broad_sel).enumerate() {
                let link = match container.select(&link_sel).next() {
                    Some(l) => l,
                    None => continue,
                };

                let href = link.value().attr("href").unwrap_or_default();
                if href.is_empty()
                    || !href.starts_with("http")
                    || href.starts_with("https://search.seznam.cz")
                    || href.starts_with("https://napoveda.seznam.cz")
                    || href.starts_with("https://o-seznam.cz")
                {
                    continue;
                }

                let title: String = link.text().collect::<String>().trim().to_string();
                if title.is_empty() || title.len() < 3 {
                    continue;
                }

                let content: String = container
                    .select(&desc_sel)
                    .map(|el| el.text().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");

                let mut r = SearchResult::new(&title, href, &content, "seznam");
                r.engine_rank = (i + 1) as u32;
                r.category = SearchCategory::General.to_string();
                results.push(r);

                if results.len() >= 10 {
                    break;
                }
            }
        }

        Ok(results)
    }
}
