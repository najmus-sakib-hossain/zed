//! SourceHut — search projects on sr.ht.
//!
//! Scrapes HTML search results from sr.ht/projects.
//! No authentication required.
//!
//! Reference: <https://sr.ht>

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

pub struct Sourcehut {
    metadata: EngineMetadata,
    client: Client,
}

impl Sourcehut {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "sourcehut".to_string().into(),
                display_name: "SourceHut".to_string().into(),
                homepage: "https://sr.ht".to_string().into(),
                categories: smallvec![SearchCategory::IT],
                enabled: true,
                timeout_ms: 8000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Sourcehut {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://sr.ht/projects?search={}&page={}&sort=recently-updated",
            urlencoding::encode(&query.query),
            query.page,
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(7))
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
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
            Ok(b) => b,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&body);
        let event_sel = Selector::parse("div.event-list div.event").unwrap();
        let h4_sel = Selector::parse("h4").unwrap();
        let link_sel = Selector::parse("h4 a").unwrap();
        let desc_sel = Selector::parse("p").unwrap();

        let mut results = Vec::new();

        for (rank, event) in document.select(&event_sel).enumerate() {
            let links: Vec<_> = event.select(&link_sel).collect();
            let project_link = if links.len() >= 2 {
                links[1]
            } else if !links.is_empty() {
                links[0]
            } else {
                continue;
            };

            let href = project_link.value().attr("href").unwrap_or_default();
            if href.is_empty() {
                continue;
            }

            let page_url = format!("https://sr.ht{}", href);

            let title = event
                .select(&h4_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            let content = event
                .select(&desc_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            if title.is_empty() {
                continue;
            }

            let mut result =
                SearchResult::new(title, page_url, content, self.metadata.name.clone());
            result.engine_rank = (rank + 1) as u32;
            results.push(result);
        }

        Ok(results)
    }
}
