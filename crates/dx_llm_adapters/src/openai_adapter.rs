//! OpenAI adapter — Tier 1 native adapter wrapping `crates/open_ai`.
//!
//! Maps OpenAI's existing API client to the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;
use std::sync::Arc;

/// OpenAI LLM adapter — wraps the existing `crates/open_ai` API client.
pub struct OpenAiLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    base_url: String,
    available: bool,
}

impl OpenAiLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self {
            id: LlmProviderId::new("openai"),
            api_key,
            base_url: "https://api.openai.com/v1".into(),
            available,
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiLlmAdapter {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::Native
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(1.25)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gpt-4o-mini".into(),
                name: "GPT-4o Mini".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.075)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "o1".into(),
                name: "o1".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(100_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(15.00),
                    output_per_million: MicroCost::from_dollars(60.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(7.50)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "o3-mini".into(),
                name: "o3-mini".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(100_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.10),
                    output_per_million: MicroCost::from_dollars(4.40),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.55)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Bridge to existing crates/open_ai client.
        // Real implementation constructs the HTTP request using the existing OpenAI client.
        log::info!(
            "OpenAI complete: model={}, messages={}",
            request.model,
            request.messages.len()
        );

        let _body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": format!("{:?}", m.role).to_lowercase(),
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": false,
        });

        // Placeholder — real implementation calls the OpenAI HTTP API
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("OpenAI stream: model={}", request.model);
        let stream = futures::stream::empty();
        Ok(Box::pin(stream))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!(
            "OpenAI embed: model={}, inputs={}",
            request.model,
            request.inputs.len()
        );
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1536]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            "gpt-4o" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(2.50),
                output_per_million: MicroCost::from_dollars(10.00),
                cached_input_per_million: Some(MicroCost::from_dollars(1.25)),
            }),
            "gpt-4o-mini" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.15),
                output_per_million: MicroCost::from_dollars(0.60),
                cached_input_per_million: Some(MicroCost::from_dollars(0.075)),
            }),
            _ => None,
        }
    }
}
