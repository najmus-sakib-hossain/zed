//! Ollama adapter — local LLM server (Tier 5: Local).

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct OllamaAdapter {
    id: LlmProviderId,
    host: String,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl OllamaAdapter {
    pub fn new(host: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            id: LlmProviderId::new("ollama"),
            host,
            http_client,
            available: parking_lot::RwLock::new(true),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Ollama" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Local }
    fn is_available(&self) -> bool { *self.available.read() }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        let url = format!("{}/api/tags", self.host);
        let request = http_client::Request::builder()
            .method(http_client::Method::GET)
            .uri(&url)
            .body(http_client::Body::empty())?;

        match self.http_client.send(request).await {
            Ok(mut response) => {
                let body_str = http_client::read_body_to_string(&mut response).await?;
                let resp: serde_json::Value = serde_json::from_str(&body_str)?;
                let models = resp["models"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| {
                        let name = m["name"].as_str().unwrap_or("unknown").to_string();
                        LlmModelInfo {
                            id: name.clone(),
                            name: name.clone(),
                            provider_id: self.id.clone(),
                            context_window: 4096,
                            max_output_tokens: None,
                            pricing: Some(TokenPricing {
                                input_per_million: MicroCost::ZERO,
                                output_per_million: MicroCost::ZERO,
                                cached_input_per_million: None,
                            }),
                            supports_streaming: true,
                            supports_tools: false,
                            supports_vision: name.contains("llava") || name.contains("vision"),
                        }
                    })
                    .collect();
                Ok(models)
            }
            Err(_) => {
                *self.available.write() = false;
                Ok(vec![])
            }
        }
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role {
                LlmRole::System => "system",
                LlmRole::User => "user",
                LlmRole::Assistant => "assistant",
                LlmRole::Tool => "tool",
            };
            let mut msg = json!({ "role": role, "content": m.content });
            if !m.images.is_empty() {
                msg["images"] = json!(m.images);
            }
            msg
        }).collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            body["options"] = json!({"temperature": temp});
        }

        let url = format!("{}/api/chat", self.host);
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        if !response.status().is_success() {
            *self.available.write() = false;
            anyhow::bail!("Ollama error {}: {}", response.status(), body_str);
        }

        let resp: serde_json::Value = serde_json::from_str(&body_str)?;
        let content = resp["message"]["content"].as_str().unwrap_or("").to_string();
        let input_tokens = resp["prompt_eval_count"].as_u64().unwrap_or(0) as usize;
        let output_tokens = resp["eval_count"].as_u64().unwrap_or(0) as usize;

        Ok(LlmResponse {
            content, model: request.model.clone(), input_tokens, output_tokens,
            cost: MicroCost::ZERO, finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role {
                LlmRole::System => "system", LlmRole::User => "user",
                LlmRole::Assistant => "assistant", LlmRole::Tool => "tool",
            };
            json!({ "role": role, "content": m.content })
        }).collect();

        let body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        let url = format!("{}/api/chat", self.host);
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        let chunks: Vec<Result<LlmStreamChunk>> = body_str
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .map(|v| {
                let delta = v["message"]["content"].as_str().unwrap_or("").to_string();
                let done = v["done"].as_bool().unwrap_or(false);
                Ok(LlmStreamChunk {
                    delta,
                    finish_reason: if done { Some("stop".into()) } else { None },
                })
            })
            .collect();

        Ok(futures::stream::iter(chunks).boxed())
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let mut all_embeddings = Vec::new();
        for input in &request.inputs {
            let body = json!({
                "model": request.model,
                "input": input,
            });
            let url = format!("{}/api/embed", self.host);
            let http_request = http_client::Request::builder()
                .method(http_client::Method::POST)
                .uri(&url)
                .header("Content-Type", "application/json")
                .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

            let mut response = self.http_client.send(http_request).await?;
            let body_str = http_client::read_body_to_string(&mut response).await?;
            let resp: serde_json::Value = serde_json::from_str(&body_str)?;

            let embedding: Vec<f32> = resp["embeddings"][0]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            all_embeddings.push(embedding);
        }

        Ok(EmbeddingResponse {
            embeddings: all_embeddings, model: request.model.clone(),
            input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        Some(TokenPricing {
            input_per_million: MicroCost::ZERO,
            output_per_million: MicroCost::ZERO,
            cached_input_per_million: None,
        })
    }
}
