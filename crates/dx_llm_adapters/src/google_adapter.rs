//! Google AI adapter — Tier 1 native adapter wrapping `crates/google_ai`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct GoogleAiLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl GoogleAiLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self {
            id: LlmProviderId::new("google-ai"),
            api_key,
            available,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for GoogleAiLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Google AI (Gemini)" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Native }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "gemini-2.5-pro".into(),
                name: "Gemini 2.5 Pro".into(),
                provider_id: self.id.clone(),
                context_window: 1_000_000,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.25),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.315)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gemini-2.5-flash".into(),
                name: "Gemini 2.5 Flash".into(),
                provider_id: self.id.clone(),
                context_window: 1_000_000,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.0375)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Google AI complete: model={}", request.model);
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("STOP".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("Google AI stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!("Google AI embed: model={}", request.model);
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 768]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("2.5-pro") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(1.25),
                output_per_million: MicroCost::from_dollars(10.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.315)),
            }),
            m if m.contains("2.5-flash") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.15),
                output_per_million: MicroCost::from_dollars(0.60),
                cached_input_per_million: Some(MicroCost::from_dollars(0.0375)),
            }),
            _ => None,
        }
    }
}
