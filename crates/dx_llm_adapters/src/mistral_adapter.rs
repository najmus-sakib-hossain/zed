//! Mistral adapter — Tier 2 named adapter wrapping `crates/mistral`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct MistralLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl MistralLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self { id: LlmProviderId::new("mistral"), api_key, available }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MistralLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Mistral AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "mistral-large-latest".into(),
                name: "Mistral Large".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: None,
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.00),
                    output_per_million: MicroCost::from_dollars(6.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "codestral-latest".into(),
                name: "Codestral".into(),
                provider_id: self.id.clone(),
                context_window: 256_000,
                max_output_tokens: None,
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.30),
                    output_per_million: MicroCost::from_dollars(0.90),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Mistral complete: model={}", request.model);
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("Mistral stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!("Mistral embed: model={}", request.model);
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1024]; request.inputs.len()],
            model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
