//! Vercel AI SDK adapter — Tier 4 aggregator.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct VercelAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl VercelAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("vercel"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for VercelAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Vercel AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Aggregator }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "v0-1.0-md".into(), name: "v0 by Vercel".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(16_384), pricing: None, supports_streaming: true, supports_tools: true, supports_vision: false },
        ])
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Vercel AI SDK gateway — OpenAI-compatible endpoint routing
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" };
            json!({ "role": role, "content": m.content })
        }).collect();
        let body = json!({ "model": request.model, "messages": messages, "stream": false });
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.vercel.ai/v1/chat/completions")
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Vercel error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO, finish_reason: None })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> { anyhow::bail!("Vercel does not support embeddings directly") }
    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
