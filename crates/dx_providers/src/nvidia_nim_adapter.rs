//! NVIDIA NIM adapter — Tier 2 Named LLM provider for the DX platform.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// NVIDIA NIM provider adapter — GPU-optimized inference microservices.
pub struct NvidiaNimLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl NvidiaNimLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("NVIDIA_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("nvidia-nim"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("nvidia-nim"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for NvidiaNimLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "NVIDIA NIM" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "nvidia/llama-3.1-nemotron-70b-instruct".into(),
                name: "Nemotron 70B".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.0),
                    output_per_million: MicroCost::from_dollars(0.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "nvidia/llama-3.3-70b-instruct".into(),
                name: "Llama 3.3 70B (NVIDIA)".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(16384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.0),
                    output_per_million: MicroCost::from_dollars(0.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "nvidia/nv-embedqa-e5-v5".into(),
                name: "NV-EmbedQA E5 v5".into(),
                provider_id: self.id.clone(), context_window: 512,
                max_output_tokens: None,
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.0),
                    output_per_million: MicroCost::ZERO,
                    cached_input_per_million: None,
                }),
                supports_streaming: false, supports_tools: false, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("NVIDIA NIM complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("NVIDIA NIM streaming not yet implemented"))
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
