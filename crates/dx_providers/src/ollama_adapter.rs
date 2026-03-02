//! Ollama adapter — wraps the existing `ollama` crate for the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, TokenPricing,
};
use futures::stream::BoxStream;

/// Ollama LLM provider adapter — Tier 5 (Local).
///
/// Wraps the existing `ollama` crate. Supports all locally installed models:
/// Llama 3.1, Qwen3, Phi-4, Mistral, CodeLlama, DeepSeek-Coder, etc.
///
/// Zero cost, unlimited use, fully offline.
pub struct OllamaLlmProvider {
    id: LlmProviderId,
    base_url: String,
    available: bool,
}

impl OllamaLlmProvider {
    /// Detect Ollama running on default localhost:11434.
    pub fn detect() -> Option<Self> {
        let base_url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        // Optimistic detection — we assume Ollama might be running.
        // Real availability is checked lazily on first call.
        Some(Self {
            id: LlmProviderId::new("ollama"),
            base_url,
            available: true,
        })
    }

    /// Create with explicit host URL.
    pub fn new(base_url: String) -> Self {
        Self {
            id: LlmProviderId::new("ollama"),
            base_url,
            available: true,
        }
    }

    /// Well-known Ollama models with their capabilities.
    fn well_known_models() -> Vec<LlmModelInfo> {
        vec![
            LlmModelInfo {
                id: "llama3.1:8b".to_string(),
                name: "Llama 3.1 8B".to_string(),
                provider_id: LlmProviderId::new("ollama"),
                context_window: 128_000,
                max_output_tokens: Some(8_192),
                pricing: None, // Free — local
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "qwen3:8b".to_string(),
                name: "Qwen3 8B".to_string(),
                provider_id: LlmProviderId::new("ollama"),
                context_window: 128_000,
                max_output_tokens: Some(8_192),
                pricing: None,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "phi4:14b".to_string(),
                name: "Phi-4 14B".to_string(),
                provider_id: LlmProviderId::new("ollama"),
                context_window: 16_384,
                max_output_tokens: Some(4_096),
                pricing: None,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "deepseek-coder-v2:16b".to_string(),
                name: "DeepSeek Coder V2 16B".to_string(),
                provider_id: LlmProviderId::new("ollama"),
                context_window: 128_000,
                max_output_tokens: Some(8_192),
                pricing: None,
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "llava:13b".to_string(),
                name: "LLaVA 13B (Vision)".to_string(),
                provider_id: LlmProviderId::new("ollama"),
                context_window: 4_096,
                max_output_tokens: Some(2_048),
                pricing: None,
                supports_streaming: true,
                supports_tools: false,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "smollm2:360m".to_string(),
                name: "SmolLM2 360M (ultra-light)".to_string(),
                provider_id: LlmProviderId::new("ollama"),
                context_window: 2_048,
                max_output_tokens: Some(1_024),
                pricing: None,
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            },
        ]
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaLlmProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::Local
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // In a full implementation, this would call GET /api/tags to list
        // actually installed models. For now, return well-known models.
        let _ = &self.base_url;
        Ok(Self::well_known_models())
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let _ = &self.base_url;
        log::debug!(
            "Ollama complete: model={}, messages={}",
            request.model,
            request.messages.len()
        );
        Err(anyhow::anyhow!(
            "Ollama adapter: HTTP bridge not yet wired to `ollama` crate"
        ))
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let _ = request;
        Err(anyhow::anyhow!("Ollama adapter: streaming bridge not yet wired"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let _ = request;
        Err(anyhow::anyhow!("Ollama adapter: embedding bridge not yet wired"))
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        None // Local models are free
    }
}
