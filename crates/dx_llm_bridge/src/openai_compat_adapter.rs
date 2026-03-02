//! Generic OpenAI-compatible adapter — Tier 3 single adapter for 40+ providers.
//!
//! Cerebras, Perplexity, Venice AI, Deep Infra, SiliconFlow, Nebius, Baseten,
//! IO.NET, Moonshot AI, MiniMax, OVHcloud, Scaleway, vLLM, GPUStack, llamafile, etc.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

/// A generic adapter that works with any provider exposing an OpenAI-compatible
/// `/v1/chat/completions` endpoint.
pub struct OpenAiCompatAdapter {
    id: LlmProviderId,
    config: OpenAiCompatibleConfig,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl OpenAiCompatAdapter {
    pub fn new(config: OpenAiCompatibleConfig, http_client: Arc<dyn HttpClient>) -> Self {
        let id_str = config
            .provider_name
            .to_lowercase()
            .replace(' ', "-")
            .replace('.', "-");
        Self {
            id: LlmProviderId::new(format!("openai-compat-{}", id_str)),
            config,
            http_client,
            available: parking_lot::RwLock::new(true),
        }
    }

    fn chat_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/chat/completions", base)
    }

    fn embeddings_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/embeddings", base)
    }

    fn build_messages(&self, messages: &[LlmMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    LlmRole::System => "system",
                    LlmRole::User => "user",
                    LlmRole::Assistant => "assistant",
                    LlmRole::Tool => "tool",
                };
                json!({ "role": role, "content": m.content })
            })
            .collect()
    }

    fn build_headers(
        &self,
        builder: http_client::http::request::Builder,
    ) -> http_client::http::request::Builder {
        let mut b = builder.header("Content-Type", "application/json");
        if let Some(ref key) = self.config.api_key {
            b = b.header("Authorization", format!("Bearer {}", key));
        }
        for (k, v) in &self.config.custom_headers {
            b = b.header(k.as_str(), v.as_str());
        }
        b
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiCompatAdapter {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.config.provider_name
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::OpenAiCompatible
    }

    fn is_available(&self) -> bool {
        *self.available.read()
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        let url = {
            let base = self.config.base_url.trim_end_matches('/');
            format!("{}/models", base)
        };

        let builder = self.build_headers(
            http_client::Request::builder()
                .method(http_client::Method::GET)
                .uri(&url),
        );
        let request = builder.body(http_client::Body::empty())?;

        match self.http_client.send(request).await {
            Ok(mut response) => {
                let body_str = http_client::read_body_to_string(&mut response).await?;
                let resp: serde_json::Value = serde_json::from_str(&body_str)?;

                let models: Vec<LlmModelInfo> = resp["data"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| LlmModelInfo {
                        id: m["id"].as_str().unwrap_or("unknown").to_string(),
                        name: m["id"].as_str().unwrap_or("unknown").to_string(),
                        provider_id: self.id.clone(),
                        context_window: m["context_length"].as_u64().unwrap_or(4096) as usize,
                        max_output_tokens: None,
                        pricing: None,
                        supports_streaming: true,
                        supports_tools: false,
                        supports_vision: false,
                    })
                    .collect();

                Ok(models)
            }
            Err(_) => {
                // If model listing fails, return default model if configured
                if let Some(ref default_model) = self.config.default_model {
                    Ok(vec![LlmModelInfo {
                        id: default_model.clone(),
                        name: default_model.clone(),
                        provider_id: self.id.clone(),
                        context_window: 4096,
                        max_output_tokens: None,
                        pricing: None,
                        supports_streaming: true,
                        supports_tools: false,
                        supports_vision: false,
                    }])
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages = self.build_messages(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        if !request.stop_sequences.is_empty() {
            body["stop"] = json!(request.stop_sequences);
        }

        let builder = self.build_headers(
            http_client::Request::builder()
                .method(http_client::Method::POST)
                .uri(&self.chat_url()),
        );
        let http_request = builder.body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        if !response.status().is_success() {
            *self.available.write() = false;
            anyhow::bail!(
                "{} API error {}: {}",
                self.config.provider_name,
                response.status(),
                body_str
            );
        }

        let resp: serde_json::Value = serde_json::from_str(&body_str)?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let input_tokens = resp["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = resp["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;
        let finish_reason = resp["choices"][0]["finish_reason"]
            .as_str()
            .map(String::from);

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            input_tokens,
            output_tokens,
            cost: MicroCost::ZERO,
            finish_reason,
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let messages = self.build_messages(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let builder = self.build_headers(
            http_client::Request::builder()
                .method(http_client::Method::POST)
                .uri(&self.chat_url()),
        );
        let http_request = builder.body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        let chunks: Vec<Result<LlmStreamChunk>> = body_str
            .lines()
            .filter(|line| line.starts_with("data: "))
            .filter(|line| *line != "data: [DONE]")
            .filter_map(|line| {
                let json_str = &line["data: ".len()..];
                serde_json::from_str::<serde_json::Value>(json_str).ok()
            })
            .map(|v| {
                let delta = v["choices"][0]["delta"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let finish_reason = v["choices"][0]["finish_reason"]
                    .as_str()
                    .map(String::from);
                Ok(LlmStreamChunk {
                    delta,
                    finish_reason,
                })
            })
            .collect();

        Ok(futures::stream::iter(chunks).boxed())
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({
            "model": request.model,
            "input": request.inputs,
        });

        let builder = self.build_headers(
            http_client::Request::builder()
                .method(http_client::Method::POST)
                .uri(&self.embeddings_url()),
        );
        let http_request = builder.body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;
        let resp: serde_json::Value = serde_json::from_str(&body_str)?;

        let embeddings: Vec<Vec<f32>> = resp["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|d| {
                d["embedding"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            })
            .collect();

        let input_tokens = resp["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(EmbeddingResponse {
            embeddings,
            model: request.model.clone(),
            input_tokens,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        // Generic adapter doesn't know pricing for arbitrary providers
        None
    }
}
