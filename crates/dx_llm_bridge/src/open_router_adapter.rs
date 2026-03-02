//! OpenRouter adapter — Tier 4 aggregator (100+ models through one API).

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct OpenRouterAdapter { id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl OpenRouterAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("openrouter"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for OpenRouterAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "OpenRouter" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Aggregator }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        let req = http_client::Request::builder().method(http_client::Method::GET).uri("https://openrouter.ai/api/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key)).body(http_client::Body::empty())?;
        match self.http_client.send(req).await {
            Ok(mut resp) => {
                let s = http_client::read_body_to_string(&mut resp).await?;
                let v: serde_json::Value = serde_json::from_str(&s)?;
                Ok(v["data"].as_array().unwrap_or(&vec![]).iter().take(200).map(|m| {
                    let id_str = m["id"].as_str().unwrap_or("unknown");
                    let pricing = m["pricing"].as_object().and_then(|p| {
                        let input = p.get("prompt")?.as_str()?.parse::<f64>().ok()?;
                        let output = p.get("completion")?.as_str()?.parse::<f64>().ok()?;
                        Some(TokenPricing { input_per_million: MicroCost::from_dollars(input * 1_000_000.0), output_per_million: MicroCost::from_dollars(output * 1_000_000.0), cached_input_per_million: None })
                    });
                    LlmModelInfo {
                        id: id_str.to_string(), name: m["name"].as_str().unwrap_or(id_str).to_string(),
                        provider_id: self.id.clone(), context_window: m["context_length"].as_u64().unwrap_or(4096) as usize,
                        max_output_tokens: m["top_provider"]["max_completion_tokens"].as_u64().map(|v| v as usize),
                        pricing, supports_streaming: true, supports_tools: false, supports_vision: false,
                    }
                }).collect())
            }
            Err(_) => { *self.available.write() = false; Ok(vec![]) }
        }
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" };
            json!({ "role": role, "content": m.content })
        }).collect();
        let mut body = json!({ "model": request.model, "messages": messages, "stream": false });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://openrouter.ai/api/v1/chat/completions")
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://zed.dev").header("X-Title", "DX by Zed")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("OpenRouter error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> { anyhow::bail!("OpenRouter does not support embeddings") }
    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
