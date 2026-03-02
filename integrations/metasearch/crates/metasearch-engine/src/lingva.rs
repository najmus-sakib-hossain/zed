//! Lingva Translate — alternative Google Translate frontend
//!
//! Queries the Lingva API at `{url}/api/v1/{source}/{target}/{query}`.
//! Configurable `url` (instance URL). Disabled by default (empty url).
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/lingva.py>

use async_trait::async_trait;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct Lingva {
    client: Client,
    url: String,
}

impl Lingva {
    pub fn new(client: Client, url: &str) -> Self {
        Self {
            client,
            url: url.to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for Lingva {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "lingva".to_string().into(),
            display_name: "Lingva".to_string().into(),
            homepage: if self.url.is_empty() {
                "https://lingva.thedaviddelta.com".to_string()
            } else {
                self.url.clone()
            }.into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.url.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.url.is_empty() {
            return Ok(vec![]);
        }

        // Default: auto-detect source, translate to English
        let api_url = format!(
            "{}/api/v1/auto/en/{}",
            self.url,
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&api_url)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Lingva request error: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Lingva parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(translation) = json.get("translation").and_then(|v| v.as_str()) {
            let page_url = format!("{}/auto/en/{}", self.url, urlencoding::encode(&query.query));

            let mut content = format!("Translation: {}", translation);

            // Extract definitions from info.definitions
            if let Some(info) = json.get("info") {
                if let Some(defs) = info.get("definitions").and_then(|d| d.as_array()) {
                    let mut def_texts = Vec::new();
                    for def in defs.iter().take(3) {
                        if let Some(list) = def.get("list").and_then(|l| l.as_array()) {
                            for item in list.iter().take(2) {
                                if let Some(d) = item.get("definition").and_then(|d| d.as_str()) {
                                    if !d.is_empty() {
                                        def_texts.push(d.to_string());
                                    }
                                }
                            }
                        }
                    }
                    if !def_texts.is_empty() {
                        content.push_str(&format!(" | Definitions: {}", def_texts.join("; ")));
                    }
                }
            }

            let mut result = SearchResult::new(
                format!("Lingva: {}", translation),
                page_url,
                content,
                "lingva",
            );
            result.engine_rank = 1;
            results.push(result);
        }

        Ok(results)
    }
}
