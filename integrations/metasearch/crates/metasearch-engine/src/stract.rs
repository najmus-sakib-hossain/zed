//! Stract — independent web search engine via JSON POST API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Stract {
    metadata: EngineMetadata,
    client: Client,
}

impl Stract {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "stract".to_string().into(),
                display_name: "Stract".to_string().into(),
                homepage: "https://stract.com".to_string().into(),
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
impl SearchEngine for Stract {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // URL and selectors ported directly from metasearch2/src/engines/search/stract.rs
        let url = reqwest::Url::parse_with_params(
            "https://stract.com/search",
            &[
                ("ss", "false"),
                // Not a tracking token — this is Stract's default search-rankings parameter
                ("sr", "N4IgNglg1gpgJiAXAbQLoBoRwgZ0rBFDEAIzAHsBjApNAXyA"),
                ("q", query.query.as_str()),
                ("optic", ""),
            ],
        )
        .map_err(|e| MetasearchError::Engine(format!("Stract URL error: {e}")))?;

        let resp = self
            .client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Stract request failed: {e}")))?;

        let body = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Stract read failed: {e}")))?;

        let document = scraper::Html::parse_document(&body);
        let mut results = Vec::new();

        // metasearch2 selectors
        let result_sel = scraper::Selector::parse(
            "div.grid.w-full.grid-cols-1.space-y-10.place-self-start > div > div.flex.min-w-0.grow.flex-col"
        ).unwrap();
        let title_sel = scraper::Selector::parse("a[title]").unwrap();
        let link_sel  = scraper::Selector::parse("a[href]").unwrap();
        let desc_sel  = scraper::Selector::parse("#snippet-text").unwrap();

        for (i, element) in document.select(&result_sel).enumerate() {
            let title = element
                .select(&title_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let url = element
                .select(&link_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default()
                .to_string();

            let description = element
                .select(&desc_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if title.is_empty() || url.is_empty() {
                continue;
            }

            let mut r = SearchResult::new(&title, &url, &description, "stract");
            r.engine_rank = (i + 1) as u32;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        Ok(results)
    }
}
