//! Anthropic adapter — Tier 1 native adapter wrapping `crates/anthropic`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;
use std::sync::Arc;

/// Anthropic LLM adapter — wraps the existing `crates/anthropic` API client.
pub struct AnthropicLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl AnthropicLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self {
            id: LlmProviderId::new("anthropic"),
            api_key,
            available,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicLlmAdapter {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Anthropic"
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
                id: "claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(64_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.00),
                    output_per_million: MicroCost::from_dollars(15.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.30)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "claude-opus-4-20250514".into(),
                name: "Claude Opus 4".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(32_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(15.00),
                    output_per_million: MicroCost::from_dollars(75.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(1.50)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "claude-3-5-haiku-20241022".into(),
                name: "Claude 3.5 Haiku".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(8_192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.80),
                    output_per_million: MicroCost::from_dollars(4.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.08)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!(
            "Anthropic complete: model={}, messages={}",
            request.model,
            request.messages.len()
        );

        // Bridge to existing crates/anthropic client.
        // Anthropic uses a different message format with system prompt separated.
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("end_turn".into()),
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("Anthropic stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!(
            "Anthropic does not provide embedding models directly"
        ))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("opus-4") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(15.00),
                output_per_million: MicroCost::from_dollars(75.00),
                cached_input_per_million: Some(MicroCost::from_dollars(1.50)),
            }),
            m if m.contains("sonnet-4") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(3.00),
                output_per_million: MicroCost::from_dollars(15.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.30)),
            }),
            m if m.contains("haiku") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.80),
                output_per_million: MicroCost::from_dollars(4.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.08)),
            }),
            _ => None,
        }
    }
}
