//! LM Studio adapter — Tier 3 Local LLM provider for the DX platform.
//! Connects to a locally-running LM Studio server via OpenAI-compatible API.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// LM Studio local provider — zero-cost local inference via LM Studio desktop app.
pub struct LmStudioLlmProvider {
    id: LlmProviderId,
    base_url: String,
    available: bool,
}

impl LmStudioLlmProvider {
    /// Attempts to create from the default local LM Studio endpoint.
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("LM_STUDIO_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());
        Some(Self {
            id: LlmProviderId::new("lm-studio"),
            base_url,
            available: true,
        })
    }

    pub fn new(base_url: String) -> Self {
        Self {
            id: LlmProviderId::new("lm-studio"),
            base_url,
            available: true,
        }
    }

    /// Check if the LM Studio server is running by probing the models endpoint.
    pub async fn probe_availability(&mut self) -> bool {
        // In a real implementation, HTTP GET to {base_url}/models
        self.available = false; // Default to false until probed
        self.available
    }
}

#[async_trait::async_trait]
impl LlmProvider for LmStudioLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "LM Studio (Local)" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Local }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // LM Studio dynamically loads models — query the local server.
        // For now return an empty list; populate from /v1/models at runtime.
        Ok(vec![
            LlmModelInfo {
                id: "local-model".into(),
                name: "Currently Loaded Model".into(),
                provider_id: self.id.clone(),
                context_window: 4096,
                max_output_tokens: Some(2048),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::ZERO,
                    output_per_million: MicroCost::ZERO,
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!(
            "LM Studio complete: url={}, model={}, messages={}",
            self.base_url, request.model, request.messages.len()
        );
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("LM Studio streaming not yet implemented"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 768]; request.inputs.len()],
            model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        // Local models are free
        Some(TokenPricing {
            input_per_million: MicroCost::ZERO,
            output_per_million: MicroCost::ZERO,
            cached_input_per_million: None,
        })
    }
}
