//! Together AI adapter — Tier 2 named adapter.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct TogetherAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl TogetherAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("together"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for TogetherAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Together AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".into(), name: "Llama 3.1 405B Turbo".into(), provider_id: self.id.clone(), context_window: 130_815, max_output_tokens: Some(4_096), pricing: self.pricing("405b"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "Qwen/Qwen2.5-72B-Instruct-Turbo".into(), name: "Qwen 2.5 72B Turbo".into(), provider_id: self.id.clone(), context_window: 32_768, max_output_tokens: Some(4_096), pricing: self.pricing("72b"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "deepseek-ai/DeepSeek-R1".into(), name: "DeepSeek R1".into(), provider_id: self.id.clone(), context_window: 163_840, max_output_tokens: Some(16_384), pricing: self.pricing("deepseek-r1"), supports_streaming: true, supports_tools: false, supports_vision: false },
        ])
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| { let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" }; json!({ "role": role, "content": m.content }) }).collect();
        let mut body = json!({ "model": request.model, "messages": messages });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.together.xyz/v1/chat/completions").header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key)).body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Together error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({ "model": "togethercomputer/m2-bert-80M-8k-retrieval", "input": request.inputs });
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.together.xyz/v1/embeddings").header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key)).body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        let v: serde_json::Value = serde_json::from_str(&s)?;
        let embeddings = v["data"].as_array().unwrap_or(&vec![]).iter().filter_map(|d| d["embedding"].as_array().map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())).collect();
        Ok(EmbeddingResponse { embeddings, model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO })
    }
    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("405b") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(3.50), output_per_million: MicroCost::from_dollars(3.50), cached_input_per_million: None }),
            m if m.contains("72b") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.60), output_per_million: MicroCost::from_dollars(0.60), cached_input_per_million: None }),
            m if m.contains("deepseek") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(3.50), output_per_million: MicroCost::from_dollars(3.50), cached_input_per_million: None }),
            _ => None,
        }
    }
}
