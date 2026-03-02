//! Cohere adapter — Tier 2 named adapter.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct CohereAdapter {
    id: LlmProviderId, api_key: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool>,
}

impl CohereAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("cohere"), api_key, http_client, available: parking_lot::RwLock::new(true) }
    }
}

#[async_trait::async_trait]
impl LlmProvider for CohereAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Cohere" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { *self.available.read() }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo { id: "command-r-plus".into(), name: "Command R+".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(4_096), pricing: self.pricing("command-r-plus"), supports_streaming: true, supports_tools: true, supports_vision: false },
            LlmModelInfo { id: "command-r".into(), name: "Command R".into(), provider_id: self.id.clone(), context_window: 128_000, max_output_tokens: Some(4_096), pricing: self.pricing("command-r"), supports_streaming: true, supports_tools: true, supports_vision: false },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Cohere v2 uses OpenAI-compatible chat format
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role { LlmRole::System => "system", LlmRole::User => "user", LlmRole::Assistant => "assistant", LlmRole::Tool => "tool" };
            json!({ "role": role, "content": m.content })
        }).collect();
        let mut body = json!({ "model": request.model, "messages": messages, "stream": false });
        if let Some(mt) = request.max_tokens { body["max_tokens"] = json!(mt); }
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }

        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.cohere.com/v2/chat")
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let body_str = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("Cohere error {}: {}", resp.status(), body_str); }
        let v: serde_json::Value = serde_json::from_str(&body_str)?;
        let content = v["message"]["content"][0]["text"].as_str().unwrap_or("").to_string();
        let input_tokens = v["usage"]["tokens"]["input_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = v["usage"]["tokens"]["output_tokens"].as_u64().unwrap_or(0) as usize;
        Ok(LlmResponse { content, model: request.model.clone(), input_tokens, output_tokens, cost: MicroCost::ZERO, finish_reason: v["finish_reason"].as_str().map(String::from) })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let resp = self.complete(request).await?;
        Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: resp.content, finish_reason: resp.finish_reason })]).boxed())
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({ "model": "embed-english-v3.0", "texts": request.inputs, "input_type": "search_document" });
        let req = http_client::Request::builder().method(http_client::Method::POST).uri("https://api.cohere.com/v2/embed")
            .header("Content-Type", "application/json").header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let body_str = http_client::read_body_to_string(&mut resp).await?;
        let v: serde_json::Value = serde_json::from_str(&body_str)?;
        let embeddings: Vec<Vec<f32>> = v["embeddings"]["float"].as_array().unwrap_or(&vec![]).iter()
            .filter_map(|e| e.as_array().map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect()))
            .collect();
        Ok(EmbeddingResponse { embeddings, model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            "command-r-plus" => Some(TokenPricing { input_per_million: MicroCost::from_dollars(2.50), output_per_million: MicroCost::from_dollars(10.00), cached_input_per_million: None }),
            "command-r" => Some(TokenPricing { input_per_million: MicroCost::from_dollars(0.15), output_per_million: MicroCost::from_dollars(0.60), cached_input_per_million: None }),
            _ => None,
        }
    }
}
