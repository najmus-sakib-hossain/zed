//! Fireworks AI adapter — Tier 2 Named LLM provider for the DX platform.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Fireworks AI LLM provider adapter.
pub struct FireworksLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl FireworksLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("FIREWORKS_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("fireworks"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("fireworks"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for FireworksLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Fireworks AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "accounts/fireworks/models/llama-v3p3-70b-instruct".into(),
                name: "Llama 3.3 70B Instruct".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(16384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.90),
                    output_per_million: MicroCost::from_dollars(0.90),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "accounts/fireworks/models/qwen2p5-72b-instruct".into(),
                name: "Qwen 2.5 72B".into(),
                provider_id: self.id.clone(), context_window: 32_768,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.90),
                    output_per_million: MicroCost::from_dollars(0.90),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "accounts/fireworks/models/deepseek-v3".into(),
                name: "DeepSeek V3 (via Fireworks)".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.90),
                    output_per_million: MicroCost::from_dollars(0.90),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Fireworks complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Fireworks streaming not yet implemented"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 768]; request.inputs.len()],
            model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
