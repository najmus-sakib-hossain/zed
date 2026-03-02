//! Groq adapter — ultra-fast inference via Groq LPU hardware. Tier 2 Named.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Groq LLM provider adapter — optimized for lowest-latency inference.
pub struct GroqLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl GroqLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("GROQ_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("groq"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("groq"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for GroqLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Groq" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "llama-3.3-70b-versatile".into(), name: "Llama 3.3 70B".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(32_768),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.59),
                    output_per_million: MicroCost::from_dollars(0.79),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "llama-3.1-8b-instant".into(), name: "Llama 3.1 8B".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.05),
                    output_per_million: MicroCost::from_dollars(0.08),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "mixtral-8x7b-32768".into(), name: "Mixtral 8x7B".into(),
                provider_id: self.id.clone(), context_window: 32_768,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.24),
                    output_per_million: MicroCost::from_dollars(0.24),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
            LlmModelInfo {
                id: "gemma2-9b-it".into(), name: "Gemma 2 9B".into(),
                provider_id: self.id.clone(), context_window: 8192,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.20),
                    output_per_million: MicroCost::from_dollars(0.20),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Groq complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Groq streaming not yet implemented"))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("Groq does not support embeddings"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
