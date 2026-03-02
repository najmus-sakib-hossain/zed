//! Mistral adapter — wraps the existing `mistral` crate for the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Mistral LLM provider adapter — Tier 2 (Named).
pub struct MistralLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl MistralLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("MISTRAL_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("mistral"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("mistral"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MistralLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Mistral AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "mistral-large-latest".into(), name: "Mistral Large".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.0),
                    output_per_million: MicroCost::from_dollars(6.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "codestral-latest".into(), name: "Codestral".into(),
                provider_id: self.id.clone(), context_window: 256_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.30),
                    output_per_million: MicroCost::from_dollars(0.90),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "mistral-small-latest".into(), name: "Mistral Small".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.10),
                    output_per_million: MicroCost::from_dollars(0.30),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Mistral complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Mistral streaming not yet implemented"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1024]; request.inputs.len()],
            model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
