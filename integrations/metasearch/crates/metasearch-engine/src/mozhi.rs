//! Mozhi Translation engine implementation (self-hosted).
//!
//! JSON API: `{base_url}/api/translate`
//! Website: https://codeberg.org/aryak/mozhi
//! Features: Translation via self-hosted Mozhi instance, no pagination

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct Mozhi {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
}

impl Mozhi {
    pub fn new(client: Client, base_url: &str) -> Self {
        let enabled = !base_url.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "mozhi".to_string().into(),
                display_name: "Mozhi".to_string().into(),
                homepage: "https://codeberg.org/aryak/mozhi".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl SearchEngine for Mozhi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/api/translate?from=auto&to=en&text={}&engine=google",
            self.base_url,
            urlencoding::encode(&query.query)
        );

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

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("JSON error: {}", e)))?;

        let mut results = Vec::new();

        // Parse translated-text from response
        let translated_text = data
            .get("translated-text")
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        if !translated_text.is_empty() {
            let mut r = SearchResult::new(
                format!("Translation: {}", translated_text),
                format!("{}/", self.base_url),
                translated_text,
                "mozhi",
            );
            r.engine_rank = 1;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        // Parse additional word choices / definitions
        if let Some(choices) = data.get("word_choices").and_then(|w| w.as_array()) {
            for (i, choice) in choices.iter().take(5).enumerate() {
                let definition = choice
                    .get("definition")
                    .and_then(|d| d.as_str())
                    .unwrap_or_default();
                let word = choice
                    .get("word")
                    .and_then(|w| w.as_str())
                    .unwrap_or_default();

                if definition.is_empty() && word.is_empty() {
                    continue;
                }

                let title = if !word.is_empty() {
                    format!("Definition: {}", word)
                } else {
                    "Definition".to_string()
                };

                let content = if !definition.is_empty() {
                    definition.to_string()
                } else {
                    word.to_string()
                };

                let mut r = SearchResult::new(
                    &title,
                    format!("{}/", self.base_url),
                    &content,
                    "mozhi",
                );
                r.engine_rank = (i + 2) as u32;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        Ok(results)
    }
}
