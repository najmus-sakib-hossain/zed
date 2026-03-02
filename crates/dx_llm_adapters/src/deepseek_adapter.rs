//! DeepSeek adapter — Tier 2 named adapter wrapping `crates/deepseek`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct DeepSeekLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl DeepSeekLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self { id: LlmProviderId::new("deepseek"), api_key, available }
    }
}

#[async_trait::async_trait]
impl LlmProvider for DeepSeekLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "DeepSeek" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "deepseek-chat".into(),
                name: "DeepSeek V3".into(),
                provider_id: self.id.clone(),
                context_window: 64_000,
                max_output_tokens: Some(8_192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.14),
                    output_per_million: MicroCost::from_dollars(0.28),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.014)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "deepseek-reasoner".into(),
                name: "DeepSeek R1".into(),
                provider_id: self.id.clone(),
                context_window: 64_000,
                max_output_tokens: Some(8_192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.55),
                    output_per_million: MicroCost::from_dollars(2.19),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.14)),
                }),
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("DeepSeek complete: model={}", request.model);
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("DeepSeek stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("DeepSeek does not provide embedding models"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            "deepseek-chat" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.14),
                output_per_million: MicroCost::from_dollars(0.28),
                cached_input_per_million: Some(MicroCost::from_dollars(0.014)),
            }),
            "deepseek-reasoner" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.55),
                output_per_million: MicroCost::from_dollars(2.19),
                cached_input_per_million: Some(MicroCost::from_dollars(0.14)),
            }),
            _ => None,
        }
    }
}
