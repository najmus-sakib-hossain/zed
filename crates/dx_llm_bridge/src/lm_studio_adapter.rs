//! LM Studio adapter — Tier 5 local models.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct LmStudioAdapter { id: LlmProviderId, host: String, http_client: Arc<dyn HttpClient>, available: parking_lot::RwLock<bool> }
impl LmStudioAdapter {
    pub fn new(host: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self { id: LlmProviderId::new("lm-studio"), host, http_client, available: parking_lot::RwLock::new(true) }
    }
}
#[async_trait::async_trait]
impl LlmProvider for LmStudioAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "LM Studio" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Local }
    fn is_available(&self) -> bool { *self.available.read() }
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        let url = format!("{}/v1/models", self.host);
        let req = http_client::Request::builder().method(http_client::Method::GET).uri(&url).body(http_client::Body::empty())?;
        match self.http_client.send(req).await {
            Ok(mut resp) => {
                let s = http_client::read_body_to_string(&mut resp).await?;
                let v: serde_json::Value = serde_json::from_str(&s)?;
                Ok(v["data"].as_array().unwrap_or(&vec![]).iter().map(|m| LlmModelInfo {
                    id: m["id"].as_str().unwrap_or("unknown").to_string(), name: m["id"].as_str().unwrap_or("unknown").to_string(),
                    provider_id: self.id.clone(), context_window: 4096, max_output_tokens: None,
                    pricing: Some(TokenPricing { input_per_million: MicroCost::ZERO, output_per_million: MicroCost::ZERO, cached_input_per_million: None }),
                    supports_streaming: true, supports_tools: false, supports_vision: false,
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
        let url = format!("{}/v1/chat/completions", self.host);
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url).header("Content-Type", "application/json").body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        if !resp.status().is_success() { anyhow::bail!("LM Studio error: {}", s); }
        let v: serde_json::Value = serde_json::from_str(&s)?;
        Ok(LlmResponse { content: v["choices"][0]["message"]["content"].as_str().unwrap_or("").into(), model: request.model.clone(), input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize, output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize, cost: MicroCost::ZERO, finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from) })
    }
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> { let r = self.complete(request).await?; Ok(futures::stream::iter(vec![Ok(LlmStreamChunk { delta: r.content, finish_reason: r.finish_reason })]).boxed()) }
    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({ "model": request.model, "input": request.inputs });
        let url = format!("{}/v1/embeddings", self.host);
        let req = http_client::Request::builder().method(http_client::Method::POST).uri(&url).header("Content-Type", "application/json").body(http_client::Body::from(serde_json::to_vec(&body)?))?;
        let mut resp = self.http_client.send(req).await?;
        let s = http_client::read_body_to_string(&mut resp).await?;
        let v: serde_json::Value = serde_json::from_str(&s)?;
        let embeddings = v["data"].as_array().unwrap_or(&vec![]).iter().filter_map(|d| d["embedding"].as_array().map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())).collect();
        Ok(EmbeddingResponse { embeddings, model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO })
    }
    fn pricing(&self, _model: &str) -> Option<TokenPricing> { Some(TokenPricing { input_per_million: MicroCost::ZERO, output_per_million: MicroCost::ZERO, cached_input_per_million: None }) }
}
