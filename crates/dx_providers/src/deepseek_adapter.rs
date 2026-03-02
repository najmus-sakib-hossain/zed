//! DeepSeek adapter — Tier 2 Named LLM provider for the DX platform.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// DeepSeek LLM provider adapter.
pub struct DeepSeekLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl DeepSeekLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("DEEPSEEK_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("deepseek"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("deepseek"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for DeepSeekLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "DeepSeek" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "deepseek-chat".into(), name: "DeepSeek V3".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.27),
                    output_per_million: MicroCost::from_dollars(1.10),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.07)),
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "deepseek-reasoner".into(), name: "DeepSeek R1".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.55),
                    output_per_million: MicroCost::from_dollars(2.19),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.14)),
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
            LlmModelInfo {
                id: "deepseek-coder".into(), name: "DeepSeek Coder V2".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.14),
                    output_per_million: MicroCost::from_dollars(0.28),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("DeepSeek complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("DeepSeek streaming not yet implemented"))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("DeepSeek does not support embeddings"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
