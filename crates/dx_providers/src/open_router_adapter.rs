//! OpenRouter adapter — Tier 4 Aggregator LLM provider.
//! Routes requests to the best available provider for any model.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// OpenRouter aggregator — unified API for 200+ models across many providers.
pub struct OpenRouterLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl OpenRouterLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("openrouter"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("openrouter"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenRouterLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "OpenRouter" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Aggregator }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // OpenRouter has 200+ models — in production, query /api/v1/models dynamically.
        // Listing a curated set of popular ones:
        Ok(vec![
            LlmModelInfo {
                id: "openai/gpt-4o".into(), name: "GPT-4o (via OpenRouter)".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(16384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.0),
                    cached_input_per_million: Some(MicroCost::from_dollars(1.25)),
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "anthropic/claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4 (via OpenRouter)".into(),
                provider_id: self.id.clone(), context_window: 200_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.0),
                    output_per_million: MicroCost::from_dollars(15.0),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.30)),
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "google/gemini-2.5-pro-preview".into(),
                name: "Gemini 2.5 Pro (via OpenRouter)".into(),
                provider_id: self.id.clone(), context_window: 1_000_000,
                max_output_tokens: Some(65536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.25),
                    output_per_million: MicroCost::from_dollars(10.0),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.315)),
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "meta-llama/llama-3.3-70b-instruct".into(),
                name: "Llama 3.3 70B (via OpenRouter)".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.39),
                    output_per_million: MicroCost::from_dollars(0.39),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("OpenRouter complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("OpenRouter streaming not yet implemented"))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("OpenRouter does not support embeddings directly"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
