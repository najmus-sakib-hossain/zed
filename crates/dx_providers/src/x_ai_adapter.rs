//! xAI (Grok) adapter — Tier 2 Named LLM provider for the DX platform.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// xAI Grok LLM provider adapter.
pub struct XAiLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl XAiLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("XAI_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("x-ai"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("x-ai"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for XAiLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "xAI (Grok)" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "grok-3".into(), name: "Grok 3".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(16384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.0),
                    output_per_million: MicroCost::from_dollars(15.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "grok-3-mini".into(), name: "Grok 3 Mini".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(16384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.30),
                    output_per_million: MicroCost::from_dollars(0.50),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "grok-2-vision".into(), name: "Grok 2 Vision".into(),
                provider_id: self.id.clone(), context_window: 32_768,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.0),
                    output_per_million: MicroCost::from_dollars(10.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("xAI complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("xAI streaming not yet implemented"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 3072]; request.inputs.len()],
            model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
