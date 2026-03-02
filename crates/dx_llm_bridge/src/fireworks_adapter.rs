//! Fireworks AI adapter — Tier 2 named adapter.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct FireworksAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl FireworksAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("fireworks"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for FireworksAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Fireworks AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "accounts/fireworks/models/llama-v3p1-405b-instruct".into(), name: "Llama 3.1 405B".into(), provider_id: self.id.clone(), context_window: 131_072, max_output_tokens: Some(16_384), pricing: self.pricing("llama-405b"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "accounts/fireworks/models/qwen2p5-72b-instruct".into(), name: "Qwen 2.5 72B".into(), provider_id: self.id.clone(), context_window: 32_768, max_output_tokens: Some(8_192), pricing: self.pricing("qwen-72b"), supports_streaming: true, supports_tools: true, supports_vision: false },
        ])
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| { let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" }; json!({ "role": role, "content": m.content }) }).collect();
        let mut body = json!({ "model": request.model, "messages": messages });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.fireworks.ai/inference/v1/chat/completions").header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key)).body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Fireworks error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> { anyhow::bail!("Use Fireworks embedding endpoint separately") }
    fn pricing(&self, _model: &str) -> Option<TokenPricing> { Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.90), output_per_million: MicroCost::from_dollars(0.90), cached_input_per_million: None }) }
}
