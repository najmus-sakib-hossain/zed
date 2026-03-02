//! Cloudflare Workers AI — text generation via Cloudflare's AI platform.
//!
//! POST JSON to `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/run/@cf/{model}`.
//! Website: https://ai.cloudflare.com
//! Features: No pagination, requires account_id and api_token

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

pub struct CloudflareAi {
    metadata: EngineMetadata,
    client: Client,
    account_id: String,
    api_token: String,
    model: String,
}

impl CloudflareAi {
    pub fn new(
        client: Client,
        account_id: Option<String>,
        api_token: Option<String>,
        model: Option<String>,
    ) -> Self {
        let acct = account_id.unwrap_or_default();
        let token = api_token.unwrap_or_default();
        let model = model.unwrap_or_else(|| "meta/llama-3-8b-instruct".to_string());
        let enabled = !acct.is_empty() && !token.is_empty();
        Self {
            metadata: EngineMetadata {
                name: "cloudflareai".to_string().into(),
                display_name: "Cloudflare AI".to_string().into(),
                homepage: "https://ai.cloudflare.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 30000,
                weight: 1.0,
            },
            client,
            account_id: acct,
            api_token: token,
            model,
        }
    }
}

#[async_trait]
impl SearchEngine for CloudflareAi {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if self.account_id.is_empty() || self.api_token.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/{}",
            self.account_id, self.model
        );

        let body = serde_json::json!({
            "prompt": query.query
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                MetasearchError::ParseError(format!("Cloudflare AI JSON error: {}", e))
            })?;

        let mut results = Vec::new();

        let content = data
            .get("result")
            .and_then(|r| r.get("response"))
            .and_then(|r| r.as_str())
            .unwrap_or_default();

        if !content.is_empty() {
            let mut r = SearchResult::new(
                "Cloudflare AI",
                "https://ai.cloudflare.com",
                content,
                "cloudflareai",
            );
            r.engine_rank = 0;
            r.category = SearchCategory::General.to_string();
            results.push(r);
        }

        info!(
            engine = "cloudflareai",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
