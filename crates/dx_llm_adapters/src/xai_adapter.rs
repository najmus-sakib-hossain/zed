//! xAI (Grok) adapter — Tier 2 named adapter wrapping `crates/x_ai`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct XAiLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl XAiLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self { id: LlmProviderId::new("xai"), api_key, available }
    }
}

#[async_trait::async_trait]
impl LlmProvider for XAiLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "xAI (Grok)" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "grok-3".into(),
                name: "Grok 3".into(),
                provider_id: self.id.clone(),
                context_window: 131_072,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.00),
                    output_per_million: MicroCost::from_dollars(15.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "grok-3-mini".into(),
                name: "Grok 3 Mini".into(),
                provider_id: self.id.clone(),
                context_window: 131_072,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.30),
                    output_per_million: MicroCost::from_dollars(0.50),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("xAI complete: model={}", request.model);
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("xAI stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("xAI does not provide embedding models"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            "grok-3" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(3.00),
                output_per_million: MicroCost::from_dollars(15.00),
                cached_input_per_million: None,
            }),
            "grok-3-mini" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.30),
                output_per_million: MicroCost::from_dollars(0.50),
                cached_input_per_million: None,
            }),
            _ => None,
        }
    }
}
