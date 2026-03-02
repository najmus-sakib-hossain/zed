//! Ollama LLM API — self-hosted large language model.
//!
//! POST JSON to `{base_url}/api/generate`.
//! Website: https://ollama.com
//! Features: No pagination, configurable model

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct Ollama {
    metadata: EngineMetadata,
    client: Client,
    base_url: String,
    model: String,
}

impl Ollama {
    pub fn new(client: Client, base_url: &str, model: Option<String>) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        let model = model.unwrap_or_else(|| "llama3".to_string());
        Self {
            metadata: EngineMetadata {
                name: "ollama".to_string().into(),
                display_name: "Ollama".to_string().into(),
                homepage: "https://ollama.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: !base.is_empty(),
                timeout_ms: 30000,
                weight: 1.0,
            },
            client,
            base_url: base,
            model,
        }
    }
}

#[async_trait]
impl SearchEngine for Ollama {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if self.base_url.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/api/generate", self.base_url);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": query.query,
            "stream": false
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("Ollama JSON error: {}", e)))?;

        let mut results = Vec::new();

        if let Some(content) = data.get("response").and_then(|r| r.as_str()) {
            if !content.is_empty() {
                let title = format!("Ollama: {}", self.model);
                let mut r = SearchResult::new(&title, &self.base_url, content, "ollama");
                r.engine_rank = 0;
                r.category = SearchCategory::General.to_string();
                results.push(r);
            }
        }

        info!(
            engine = "ollama",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
