//! Groq adapter — Tier 2 named adapter (fastest inference).

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct GroqAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }

impl GroqAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("groq"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}

#[async_trait::async_trait]
impl LlmProvider for GroqAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Groq" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "llama-3.3-70b-versatile".into(), name: "Llama 3.3 70B".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(32_768), pricing: self.pricing("llama-3.3-70b-versatile"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "llama-3.1-8b-instant".into(), name: "Llama 3.1 8B".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(8_192), pricing: self.pricing("llama-3.1-8b-instant"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "mixtral-8x7b-32768".into(), name: "Mixtral 8x7B".into(), provider_id: self.id.clone(), context_window: 32_768, max_output_tokens: Some(8_192), pricing: self.pricing("mixtral-8x7b-32768"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "deepseek-r1-distill-llama-70b".into(), name: "DeepSeek R1 Distill 70B".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(16_384), pricing: self.pricing("deepseek-r1-distill-llama-70b"), supports_streaming: true, supports_tools: false, supports_vision: false },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" };
            json!({ "role": role, "content": m.content })
        }).collect();
        let mut body = json!({ "model": request.model, "messages": messages, "stream": false });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }

        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.groq.com/openai/v1/chat/completions")
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let body_str = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Groq error {}: {}", resp.status(), body_str); }
        let v: serde_json::Value = serde_json::from_str(&body_str)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let resp = self.complete(request).await?;
        Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: resp.content, finish_reason: resp.finish_reason })]).boxed())
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        anyhow::bail!("Groq does not support embeddings")
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("70b") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.59), output_per_million: MicroCost::from_dollars(0.79), cached_input_per_million: None }),
            m if m.contains("8b") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.05), output_per_million: MicroCost::from_dollars(0.08), cached_input_per_million: None }),
            m if m.contains("mixtral") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.24), output_per_million: MicroCost::from_dollars(0.24), cached_input_per_million: None }),
            _ => None,
        }
    }
}
