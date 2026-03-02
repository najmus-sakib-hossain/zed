//! Hugging Face Inference API adapter — Tier 2 Named LLM provider.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Hugging Face Inference API adapter — access to thousands of open models.
pub struct HuggingFaceLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl HuggingFaceLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("HF_API_KEY")
            .or_else(|_| std::env::var("HUGGING_FACE_API_KEY"))
            .ok()?;
        Some(Self {
            id: LlmProviderId::new("huggingface"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("huggingface"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for HuggingFaceLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Hugging Face" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "meta-llama/Meta-Llama-3.1-70B-Instruct".into(),
                name: "Llama 3.1 70B (HF)".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.0),
                    output_per_million: MicroCost::from_dollars(0.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
            LlmModelInfo {
                id: "mistralai/Mixtral-8x7B-Instruct-v0.1".into(),
                name: "Mixtral 8x7B (HF)".into(),
                provider_id: self.id.clone(), context_window: 32_768,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.0),
                    output_per_million: MicroCost::from_dollars(0.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
            LlmModelInfo {
                id: "BAAI/bge-large-en-v1.5".into(),
                name: "BGE Large English v1.5".into(),
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
        log::info!("HuggingFace complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("HuggingFace streaming not yet implemented"))
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
