//! Cohere adapter — Tier 2 Named LLM provider for the DX platform.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Cohere LLM provider adapter — strong embeddings and RAG capabilities.
pub struct CohereLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl CohereLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("COHERE_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("cohere"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("cohere"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for CohereLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Cohere" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "command-r-plus".into(), name: "Command R+".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "command-r".into(), name: "Command R".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "embed-english-v3.0".into(), name: "Embed English v3".into(),
                provider_id: self.id.clone(), context_window: 512,
                max_output_tokens: None,
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.10),
                    output_per_million: MicroCost::ZERO,
                    cached_input_per_million: None,
                }),
                supports_streaming: false, supports_tools: false, supports_vision: false,
            },
            LlmModelInfo {
                id: "embed-multilingual-v3.0".into(), name: "Embed Multilingual v3".into(),
                provider_id: self.id.clone(), context_window: 512,
                max_output_tokens: None,
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.10),
                    output_per_million: MicroCost::ZERO,
                    cached_input_per_million: None,
                }),
                supports_streaming: false, supports_tools: false, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Cohere complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Cohere streaming not yet implemented"))
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
