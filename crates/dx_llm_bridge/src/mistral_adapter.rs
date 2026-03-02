//! Mistral adapter — Tier 2 named adapter.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct MistralAdapter {
    id: LlmProviderId,
    api_key: String,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl MistralAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("mistral"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
    fn base_url(&self) -> &str { "https://api.mistral.ai/v1" }
}

#[async_trait::async_trait]
impl LlmProvider for MistralAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Mistral AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "mistral-large-latest".into(), name: "Mistral Large".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(8_192), pricing: self.pricing("mistral-large-latest"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "mistral-medium-latest".into(), name: "Mistral Medium".into(), provider_id: self.id.clone(), context_window: 32_000, max_output_tokens: Some(8_192), pricing: self.pricing("mistral-medium-latest"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "mistral-small-latest".into(), name: "Mistral Small".into(), provider_id: self.id.clone(), context_window: 32_000, max_output_tokens: Some(8_192), pricing: self.pricing("mistral-small-latest"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "codestral-latest".into(), name: "Codestral".into(), provider_id: self.id.clone(), context_window: 32_000, max_output_tokens: Some(8_192), pricing: self.pricing("codestral-latest"), supports_streaming: true, supports_tools: false, supports_vision: false },
            LlmModelInfo { id: "pixtral-large-latest".into(), name: "Pixtral Large".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(8_192), pricing: self.pricing("pixtral-large-latest"), supports_streaming: true, supports_tools: true, supports_vision: true },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" };
            json!({ "role": role, "content": m.content })
        }).collect();
        let mut body = json!({ "model": request.model, "messages": messages });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }
        if let Some(tp) = request.top_p { body["top_p"] = json!(tp); }

        let url = format!("{}/chat/completions", self.base_url());
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url)
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let body_str = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Mistral error {}: {}", resp.status(), body_str); }
        let v: serde_json::Value = serde_json::from_str(&body_str)?;
        Ok(LlmResponse {
            content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string(),
            model: request.model.clone(),
            input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize,
            cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let resp = self.complete(request).await?;
        Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: resp.content, finish_reason: resp.finish_reason })]).boxed())
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({ "model": request.model, "input": request.inputs });
        let url = format!("{}/embeddings", self.base_url());
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url)
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let body_str = http_client::read_body_to_string(&mut resp).await?;
        let v: serde_json::Value = serde_json::from_str(&body_str)?;
        let embeddings = v["data"].as_array().unwrap_or(&vec![]).iter().filter_map(|d| {
            d["embedding"].as_array().map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
        }).collect();
        Ok(EmbeddingResponse { embeddings, model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("large") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(2.00), output_per_million: MicroCost::from_dollars(6.00), cached_input_per_million: None }),
            m if m.contains("small") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.20), output_per_million: MicroCost::from_dollars(0.60), cached_input_per_million: None }),
            m if m.contains("codestral") => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.30), output_per_million: MicroCost::from_dollars(0.90), cached_input_per_million: None }),
            _ => None,
        }
    }
}
