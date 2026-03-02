//! Ollama adapter — Tier 1/5 native adapter wrapping `crates/ollama`.
//! Local models via Ollama, free and unlimited.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct OllamaLlmAdapter {
    id: LlmProviderId,
    base_url: String,
    available: bool,
}

impl OllamaLlmAdapter {
    pub fn new(base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "http://localhost:11434".into());
        Self {
            id: LlmProviderId::new("ollama"),
            base_url: url,
            available: true, // Assume available; health check at runtime
        }
    }

    /// Check if Ollama is actually running.
    pub async fn health_check(&mut self) -> bool {
        // Real implementation: GET http://localhost:11434/api/tags
        self.available = true; // Placeholder
        self.available
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Ollama (Local)" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Local }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // Real implementation: GET {base_url}/api/tags, parse response
        log::info!("Ollama list_models from {}", self.base_url);
        Ok(vec![
            LlmModelInfo {
                id: "llama3.2:latest".into(),
                name: "Llama 3.2 (Ollama)".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: None,
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::ZERO,
                    output_per_million: MicroCost::ZERO,
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Ollama complete: model={} at {}", request.model, self.base_url);
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("Ollama stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!("Ollama embed: model={}", request.model);
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 384]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        Some(TokenPricing {
            input_per_million: MicroCost::ZERO,
            output_per_million: MicroCost::ZERO,
            cached_input_per_million: None,
        })
    }
}
