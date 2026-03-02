//! Wordnik — English dictionary (HTML scraping)
//!
//! Scrapes word definitions from `https://www.wordnik.com/words/{query}`.
//! No API key required.
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/wordnik.py>

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

pub struct Wordnik {
    client: Client,
}

impl Wordnik {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for Wordnik {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "Wordnik".to_string().into(),
            display_name: "Wordnik".to_string().into(),
            homepage: "https://www.wordnik.com".to_string().into(),
            categories: smallvec![SearchCategory::General, SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://www.wordnik.com/words/{}",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (compatible; metasearch/1.0)")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Wordnik request error: {e}")))?;

        let html = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Wordnik read error: {e}")))?;

        let document = Html::parse_document(&html);

        // Select definition list items under #define
        let define_sel = Selector::parse("#define li").unwrap();
        let abbr_sel = Selector::parse("abbr").unwrap();

        let mut results = Vec::new();

        for (i, li) in document.select(&define_sel).enumerate() {
            let full_text: String = li.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if full_text.is_empty() {
                continue;
            }

            // Extract part of speech abbreviation
            let pos = li
                .select(&abbr_sel)
                .next()
                .map(|a| a.text().collect::<String>())
                .unwrap_or_default();

            let def = if !pos.is_empty() && full_text.starts_with(&pos) {
                full_text[pos.len()..].trim().to_string()
            } else {
                full_text
            };

            let title = if pos.is_empty() {
                def.clone()
            } else {
                format!("({}) {}", pos.trim(), def)
            };

            results.push(SearchResult {
                title,
                url: url.clone(),
                content: def,
                engine: "wordnik".to_string(),
                engine_rank: (i + 1) as u32,
                    score: 0.0,
                    thumbnail: None,
                    published_date: None,
                    category: String::new(),
                    metadata: serde_json::Value::Null,
                });

            if results.len() >= 10 {
                break;
            }
        }

        Ok(results)
    }
}
