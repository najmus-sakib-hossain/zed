//! Mojeek — independent web search engine (HTML scraping)
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/mojeek.py>

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};

pub struct Mojeek {
    metadata: EngineMetadata,
    client: Client,
}

impl Mojeek {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "mojeek".to_string().into(),
                display_name: "Mojeek".to_string().into(),
                homepage: "https://mojeek.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.6,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for Mojeek {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let mut url = format!(
            "https://www.mojeek.com/search?q={}",
            urlencoding::encode(&query.query)
        );
        // Setting s=0 on the first page triggers a rate-limit, so only add offset for page > 1
        if query.page > 1 {
            url.push_str(&format!("&s={}", 10 * (query.page - 1)));
        }

        let resp = match self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
            .timeout(std::time::Duration::from_secs(7))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let html_text = match resp.text().await {
            Ok(t) => t,
            Err(_) => return Ok(Vec::new()),
        };

        let document = Html::parse_document(&html_text);

        let li_sel = Selector::parse("ul.results-standard > li").unwrap();
        let title_sel = Selector::parse("h2 a").unwrap();
        let url_sel = Selector::parse("a.ob").unwrap();
        let content_sel = Selector::parse("p.s").unwrap();

        let mut results = Vec::new();

        for (i, li) in document.select(&li_sel).enumerate() {
            let title = li
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let link = li
                .select(&url_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default()
                .to_string();

            let content = li
                .select(&content_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() && !link.is_empty() {
                let mut result = SearchResult::new(
                    title.trim().to_string(),
                    link,
                    content.trim().to_string(),
                    self.metadata.name.clone(),
                );
                result.engine_rank = (i + 1) as u32;
                results.push(result);
            }
        }

        Ok(results)
    }
}
