//! Replicate adapter — Tier 2 Named LLM provider for the DX platform.
//! Replicate runs open-source models via cloud GPUs with a pay-per-use model.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Replicate LLM provider adapter.
pub struct ReplicateLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl ReplicateLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("REPLICATE_API_TOKEN").ok()?;
        Some(Self {
            id: LlmProviderId::new("replicate-llm"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("replicate-llm"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for ReplicateLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Replicate" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "meta/meta-llama-3.1-405b-instruct".into(),
                name: "Llama 3.1 405B (Replicate)".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(9.50),
                    output_per_million: MicroCost::from_dollars(9.50),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
            LlmModelInfo {
                id: "meta/meta-llama-3-70b-instruct".into(),
                name: "Llama 3 70B (Replicate)".into(),
                provider_id: self.id.clone(), context_window: 8192,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.65),
                    output_per_million: MicroCost::from_dollars(2.75),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Replicate complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Replicate LLM streaming not yet implemented"))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("Replicate does not have a dedicated embeddings endpoint"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
