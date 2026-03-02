//! LibreTranslate — Free and Open Source Machine Translation API
//!
//! Configurable `base_url` (can be a list in SearXNG; we use a single URL)
//! and optional `api_key`.  Disabled by default (empty base_url).
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/libretranslate.py>

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct LibreTranslate {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl LibreTranslate {
    pub fn new(client: Client, base_url: &str, api_key: Option<String>) -> Self {
        Self {
            client,
            base_url: base_url.to_string(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for LibreTranslate {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "libretranslate".to_string().into(),
            display_name: "LibreTranslate".to_string().into(),
            homepage: if self.base_url.is_empty() {
                "https://libretranslate.com".to_string()
            } else {
                self.base_url.clone()
            }.into(),
            categories: smallvec![SearchCategory::General],
            enabled: !self.base_url.is_empty(),
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.base_url.is_empty() {
            return Ok(vec![]);
        }

        let url = format!("{}/translate", self.base_url);

        let mut body = json!({
            "q": query.query,
            "source": "auto",
            "target": "en",
            "alternatives": 3
        });

        if let Some(ref key) = self.api_key {
            body["api_key"] = serde_json::Value::String(key.clone());
        }

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("LibreTranslate request error: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("LibreTranslate parse error: {e}")))?;

        let mut results = Vec::new();

        if let Some(text) = json.get("translatedText").and_then(|v| v.as_str()) {
            let mut content = format!("Translation: {}", text);
            if let Some(alts) = json.get("alternatives").and_then(|v| v.as_array()) {
                let alt_strs: Vec<&str> = alts.iter().filter_map(|a| a.as_str()).collect();
                if !alt_strs.is_empty() {
                    content.push_str(&format!(" | Alternatives: {}", alt_strs.join(", ")));
                }
            }
            let mut result = SearchResult::new(
                format!("LibreTranslate: {}", text),
                self.base_url.clone(),
                content,
                "libretranslate",
            );
            result.engine_rank = 1;
            results.push(result);
        }

        Ok(results)
    }
}
