//! OpenAI-Compatible generic adapter — Tier 3.
//!
//! A single adapter that works with 40+ providers that expose an OpenAI-compatible
//! API. Configured via `OpenAiCompatibleConfig` from `dx_core`.
//!
//! Supported providers include: Cerebras, Perplexity, Venice AI, Baseten, Deep Infra,
//! IO.NET, Moonshot AI, MiniMax, Nebius, OVHcloud, Scaleway, SiliconFlow,
//! Inference.net, vLLM, GPUStack, llamafile, Groq, Fireworks AI, Together AI,
//! Mistral, DeepSeek, xAI, Cohere, LM Studio, etc.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, OpenAiCompatibleConfig,
    TokenPricing,
};
use futures::stream::BoxStream;
use std::collections::HashMap;

/// OpenAI-Compatible generic LLM provider — Tier 3.
///
/// One adapter, 40+ providers. Any service that exposes `/v1/chat/completions`
/// following the OpenAI API spec can be used through this adapter.
pub struct OpenAiCompatibleProvider {
    id: LlmProviderId,
    config: OpenAiCompatibleConfig,
    available: bool,
}

impl OpenAiCompatibleProvider {
    /// Create from an explicit config.
    pub fn from_config(config: OpenAiCompatibleConfig) -> Self {
        let id_str = config.provider_name.to_lowercase().replace(' ', "-");
        Self {
            id: LlmProviderId::new(id_str),
            config,
            available: true,
        }
    }

    /// Discover providers from well-known environment variables.
    ///
    /// Checks for `GROQ_API_KEY`, `FIREWORKS_API_KEY`, `TOGETHER_API_KEY`,
    /// `DEEPSEEK_API_KEY`, `MISTRAL_API_KEY`, `XAI_API_KEY`, `PERPLEXITY_API_KEY`,
    /// `CEREBRAS_API_KEY`, `DEEPINFRA_API_KEY`, `SILICONFLOW_API_KEY`, etc.
    pub fn discover_from_env() -> Vec<Self> {
        let known_providers: Vec<(&str, &str, &str)> = vec![
            ("GROQ_API_KEY", "Groq", "https://api.groq.com/openai/v1"),
            ("FIREWORKS_API_KEY", "Fireworks AI", "https://api.fireworks.ai/inference/v1"),
            ("TOGETHER_API_KEY", "Together AI", "https://api.together.xyz/v1"),
            ("DEEPSEEK_API_KEY", "DeepSeek", "https://api.deepseek.com"),
            ("MISTRAL_API_KEY", "Mistral", "https://api.mistral.ai/v1"),
            ("XAI_API_KEY", "xAI", "https://api.x.ai/v1"),
            ("PERPLEXITY_API_KEY", "Perplexity", "https://api.perplexity.ai"),
            ("CEREBRAS_API_KEY", "Cerebras", "https://api.cerebras.ai/v1"),
            ("DEEPINFRA_API_KEY", "Deep Infra", "https://api.deepinfra.com/v1/openai"),
            ("SILICONFLOW_API_KEY", "SiliconFlow", "https://api.siliconflow.cn/v1"),
            ("OPENROUTER_API_KEY", "OpenRouter", "https://openrouter.ai/api/v1"),
            ("NVIDIA_API_KEY", "NVIDIA NIM", "https://integrate.api.nvidia.com/v1"),
            ("NEBIUS_API_KEY", "Nebius", "https://api.studio.nebius.ai/v1"),
            ("COHERE_API_KEY", "Cohere", "https://api.cohere.ai/compatibility/v1"),
            ("VENICE_API_KEY", "Venice AI", "https://api.venice.ai/api/v1"),
        ];

        let mut providers = Vec::new();
        for (env_var, name, base_url) in known_providers {
            if let Ok(api_key) = std::env::var(env_var) {
                if !api_key.is_empty() {
                    providers.push(Self::from_config(OpenAiCompatibleConfig {
                        provider_name: name.to_string(),
                        base_url: base_url.to_string(),
                        api_key: Some(api_key),
                        default_model: None,
                        custom_headers: HashMap::new(),
                    }));
                }
            }
        }
        providers
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.config.provider_name
    }

    fn tier(&self) -> LlmProviderTier {
        // Some well-known providers get higher tier classification
        match self.config.provider_name.as_str() {
            "Mistral" | "DeepSeek" | "xAI" | "Cohere" => LlmProviderTier::Named,
            "OpenRouter" => LlmProviderTier::Aggregator,
            _ => LlmProviderTier::OpenAiCompatible,
        }
    }

    fn is_available(&self) -> bool {
        self.available && self.config.api_key.as_ref().map_or(false, |k| !k.is_empty())
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // OpenAI-compatible providers support GET /v1/models
        // For now return empty — real implementation would query the endpoint
        let _ = &self.config.base_url;
        Ok(Vec::new())
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let _ = &self.config;
        log::debug!(
            "{} complete: model={}, messages={}",
            self.config.provider_name,
            request.model,
            request.messages.len()
        );
        Err(anyhow::anyhow!(
            "{} adapter: HTTP bridge not yet wired",
            self.config.provider_name
        ))
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let _ = request;
        Err(anyhow::anyhow!(
            "{} adapter: streaming bridge not yet wired",
            self.config.provider_name
        ))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let _ = request;
        Err(anyhow::anyhow!(
            "{} adapter: embedding bridge not yet wired",
            self.config.provider_name
        ))
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        None // Pricing varies per provider — would need a pricing database
    }
}
