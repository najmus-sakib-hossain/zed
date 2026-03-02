//! DictZone — online dictionary (HTML scraping)
//!
//! Scrapes translation results from dictzone.com.
//! URL pattern: `https://dictzone.com/{from}-{to}-dictionary/{query}`
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/dictzone.py>

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct DictZone {
    client: Client,
}

impl DictZone {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for DictZone {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "DictZone".to_string().into(),
            display_name: "DictZone".to_string().into(),
            homepage: "https://dictzone.com".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // Default: English to German (matching SearXNG defaults)
        let url = format!(
            "https://dictzone.com/english-german-dictionary/{}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            )
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await;

        // dictzone.com may be unreachable — return empty gracefully
        let resp = match resp {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        let html = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("DictZone read error: {e}")))?;

        let document = Html::parse_document(&html);
        let row_sel = Selector::parse("table#r tr").unwrap();
        let td_sel = Selector::parse("td").unwrap();

        let mut results = Vec::new();

        for (i, row) in document.select(&row_sel).enumerate() {
            let tds: Vec<_> = row.select(&td_sel).collect();
            if tds.len() != 2 {
                continue;
            }

            let from_text: String = tds[0]
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let to_text: String = tds[1]
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            if from_text.is_empty() && to_text.is_empty() {
                continue;
            }

            let mut result = SearchResult::new(
                format!("{} → {}", from_text, to_text),
                url.clone(),
                format!("{}: {}", from_text, to_text),
                "dictzone",
            );
            result.engine_rank = (i + 1) as u32;
            results.push(result);

            if results.len() >= 10 {
                break;
            }
        }

        Ok(results)
    }
}
