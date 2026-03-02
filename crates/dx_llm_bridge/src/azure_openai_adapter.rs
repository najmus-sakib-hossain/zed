//! Azure OpenAI adapter — Tier 1 native adapter with versioned endpoints.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct AzureOpenAiAdapter {
    id: LlmProviderId, api_key: String, endpoint: String, api_version: String,
    http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool>,
}
impl AzureOpenAiAdapter {
    pub fn new(api_key: String, endpoint: String, api_version: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("azure-openai"), api_key, endpoint: endpoint.trim_end_matches('/').to_string(), api_version, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for AzureOpenAiAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Azure OpenAI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Native }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "gpt-4o".into(), name: "GPT-4o (Azure)".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(16_384), pricing: self.pricing("gpt-4o"), supports_streaming: true, supports_tools: true, supports_vision: true },
            LlmModelInfo { id: "gpt-4o-mini".into(), name: "GPT-4o Mini (Azure)".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(16_384), pricing: self.pricing("gpt-4o-mini"), supports_streaming: true, supports_tools: true, supports_vision: true },
        ])
    }
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" };
            json!({ "role": role, "content": m.content })
        }).collect();
        let mut body = json!({ "messages": messages });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }

        let url = format!("{}/openai/deployments/{}/chat/completions?api-version={}", self.endpoint, request.model, self.api_version);
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url)
            .header("Content-Type", "application/json").header("api-key", &self.api_key)
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Azure OpenAI error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({ "input": request.inputs });
        let url = format!("{}/openai/deployments/{}/embeddings?api-version={}", self.endpoint, request.model, self.api_version);
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url).header("Content-Type", "application/json").header("api-key", &self.api_key).body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        let v: serde_json::Value = serde_json::from_str(&s)?;
        let embeddings = v["data"].as_array().unwrap_or(&vec![]).iter().filter_map(|d| d["embedding"].as_array().map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())).collect();
        Ok(EmbeddingResponse { embeddings, model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO })
    }
    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            "gpt-4o" => Some(TokenPricing { input_per_million: MicroCost::from_dollars(2.50), output_per_million: MicroCost::from_dollars(10.00), cached_input_per_million: None }),
            "gpt-4o-mini" => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.15), output_per_million: MicroCost::from_dollars(0.60), cached_input_per_million: None }),
            _ => None,
        }
    }
}
