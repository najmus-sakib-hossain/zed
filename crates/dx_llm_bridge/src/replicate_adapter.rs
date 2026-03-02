//! Replicate adapter — Tier 2 named adapter.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct ReplicateAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl ReplicateAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("replicate"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for ReplicateAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Replicate" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "meta/meta-llama-3.1-405b-instruct".into(), name: "Llama 3.1 405B (Replicate)".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(4_096), pricing: None, supports_streaming: true, supports_tools: false, supports_vision: false },
        ])
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Replicate uses a prediction API — create prediction, then poll for result.
        let prompt = request.messages.iter().map(|m| {
            let prefix = match m.role { LlmRole::System => "[SYSTEM]", LlmRole::User => "[USER]", LlmRole::Assistant => "[ASSISTANT]", LlmRole::Tool => "[TOOL]" };
            format!("{} {}", prefix, m.content)
        }).collect::<Vec<_>>().join("\n");

        let body = json!({
            "input": {
                "prompt": prompt,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
            }
        });
        let url = format!("https://api.replicate.com/v1/models/{}/predictions", request.model);
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url)
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .header("Prefer", "wait")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Replicate error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;

        let content = match &v["output"] {
            serde_json::Value::Array(arr) => arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(""),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        };

        Ok(LlmResponse { content, model: request.model.clone(), input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO, finish_reason: Some("stop".into()) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> { anyhow::bail!("Replicate embedding via separate model endpoint") }
    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
