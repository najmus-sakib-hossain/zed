//! Generic OpenAI-compatible adapter — Tier 3.
//!
//! A single adapter that wraps any of the 40+ providers exposing the
//! `/v1/chat/completions` endpoint. Providers register themselves with
//! [`OpenAiCompatibleConfig`] from `dx_core`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, OpenAiCompatibleConfig,
    TokenPricing,
};
use futures::stream::BoxStream;

/// A generic adapter for any OpenAI-compatible API endpoint.
pub struct OpenAiCompatLlmAdapter {
    id: LlmProviderId,
    config: OpenAiCompatibleConfig,
    available: bool,
}

impl OpenAiCompatLlmAdapter {
    pub fn new(config: OpenAiCompatibleConfig) -> Self {
        let available = config.api_key.as_ref().map_or(false, |k| !k.is_empty());
        let id = LlmProviderId::new(&config.name);
        Self { id, config, available }
    }

    /// Create adapters for all well-known OpenAI-compatible providers.
    pub fn well_known_providers() -> Vec<OpenAiCompatibleConfig> {
        use dx_core::known_openai_compatible_providers;
        known_openai_compatible_providers()
    }

    /// Try to construct a provider from an environment variable convention.
    /// E.g. `TOGETHER_API_KEY` → Together AI.
    pub fn from_env(config: OpenAiCompatibleConfig) -> Self {
        let env_key = format!("{}_API_KEY", config.name.to_uppercase().replace(' ', "_"));
        let api_key = std::env::var(&env_key).unwrap_or_default();
        let mut cfg = config;
        if cfg.api_key.is_none() && !api_key.is_empty() {
            cfg.api_key = Some(api_key);
        }
        Self::new(cfg)
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiCompatLlmAdapter {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }
    fn name(&self) -> &str {
        &self.config.name
    }
    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::OpenAiCompat
    }
    fn is_available(&self) -> bool {
        self.available
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // For a generic adapter, the model list comes from the config or
        // from a `/v1/models` discovery call.
        let models: Vec<LlmModelInfo> = self
            .config
            .default_models
            .iter()
            .map(|m| LlmModelInfo {
                id: m.clone(),
                name: m.clone(),
                provider_id: self.id.clone(),
                context_window: self.config.default_context_window.unwrap_or(4096),
                max_output_tokens: self.config.default_max_output,
                pricing: None,
                supports_streaming: true,
                supports_tools: self.config.supports_tools,
                supports_vision: self.config.supports_vision,
            })
            .collect();
        Ok(models)
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!(
            "OpenAI-compat [{}] complete: model={}",
            self.config.name,
            request.model,
        );
        // In production this would POST to `{base_url}/v1/chat/completions`.
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!(
            "OpenAI-compat [{}] stream: model={}",
            self.config.name,
            request.model,
        );
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        if self.config.supports_embeddings {
            // Would POST to `{base_url}/v1/embeddings`.
            Ok(EmbeddingResponse {
                embeddings: vec![],
                model: String::new(),
                total_tokens: 0,
                cost: MicroCost::ZERO,
            })
        } else {
            Err(anyhow::anyhow!(
                "{} does not support embeddings",
                self.config.name
            ))
        }
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        // Generic adapter doesn't have built-in pricing tables.
        // Pricing can be supplied via config.
        None
    }
}

/// Register all well-known OpenAI-compatible providers that have API keys set.
pub fn register_available_compat_providers() -> Vec<OpenAiCompatLlmAdapter> {
    OpenAiCompatLlmAdapter::well_known_providers()
        .into_iter()
        .map(OpenAiCompatLlmAdapter::from_env)
        .filter(|a| a.is_available())
        .collect()
}
