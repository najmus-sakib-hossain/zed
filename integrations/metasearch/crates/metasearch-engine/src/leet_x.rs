//! 1337x engine — search torrents via HTML scraping.
//! Translated from SearXNG `searx/engines/1337x.py`.

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

pub struct LeetX {
    metadata: EngineMetadata,
    client: Client,
}

impl LeetX {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "1337x".to_string().into(),
                display_name: "1337x".to_string().into(),
                homepage: "https://1337x.to".to_string().into(),
                categories: smallvec![SearchCategory::Files],
                enabled: true,
                timeout_ms: 5000,
                weight: 0.7,
            },
            client,
        }
    }
}

#[async_trait]
impl SearchEngine for LeetX {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let page = query.page;
        let url = format!(
            "https://1337x.to/search/{}/{}/",
            urlencoding::encode(&query.query),
            page,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let html_text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::ParseError(e.to_string()))?;

        let document = Html::parse_document(&html_text);
        let row_sel = Selector::parse("table.table-list tbody tr").unwrap();
        let name_sel = Selector::parse("td.name a:nth-child(2)").unwrap();
        let seeds_sel = Selector::parse("td.seeds").unwrap();
        let leeches_sel = Selector::parse("td.leeches").unwrap();
        let size_sel = Selector::parse("td.size").unwrap();

        let mut results = Vec::new();

        for (i, row) in document.select(&row_sel).enumerate() {
            let title = row
                .select(&name_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            let href = row
                .select(&name_sel)
                .next()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or_default();

            let item_url = format!("https://1337x.to{}", href);

            let seeds = row
                .select(&seeds_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            let leeches = row
                .select(&leeches_sel)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            let filesize = row
                .select(&size_sel)
                .next()
                .map(|e| e.text().next().unwrap_or_default().to_string())
                .unwrap_or_default();

            let snippet = format!(
                "Seeds: {} | Leeches: {} | Size: {}",
                seeds.trim(),
                leeches.trim(),
                filesize.trim(),
            );

            let mut result = SearchResult::new(
                title.trim().to_string(),
                item_url,
                snippet,
                "1337x".to_string(),
            );
            result.engine_rank = (i + 1) as u32;
            result.category = SearchCategory::Files.to_string();
            results.push(result);
        }

        Ok(results)
    }
}
