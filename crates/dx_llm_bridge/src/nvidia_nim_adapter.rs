//! NVIDIA NIM adapter — Tier 2 named adapter.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct NvidiaNimAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl NvidiaNimAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("nvidia-nim"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for NvidiaNimAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "NVIDIA NIM" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "meta/llama-3.1-405b-instruct".into(), name: "Llama 3.1 405B (NIM)".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(4_096), pricing: None, supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "nvidia/nemotron-4-340b-instruct".into(), name: "Nemotron 4 340B".into(), provider_id: self.id.clone(), context_window: 4_096, max_output_tokens: Some(4_096), pricing: None, supports_streaming: true, supports_tools: false, supports_vision: false },
        ])
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| { let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" }; json!({ "role": role, "content": m.content }) }).collect();
        let mut body = json!({ "model": request.model, "messages": messages });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://integrate.api.nvidia.com/v1/chat/completions").header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key)).body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("NVIDIA NIM error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> { anyhow::bail!("Use NVIDIA embedding NIM separately") }
    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
