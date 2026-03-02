//! dx_llm_adapters — Bridges existing Zed LLM provider crates to the DX `LlmProvider` trait.
//!
//! Each adapter wraps an existing Zed provider crate and implements the unified
//! `LlmProvider` interface from `dx_core`, enabling transparent provider switching,
//! fallback chains, and cost tracking across 100+ LLM providers.
//!
//! # Tier 1 — Native Adapters (full SDK-level)
//! - OpenAI (`crates/open_ai`)
//! - Anthropic (`crates/anthropic`)
//! - Google AI (`crates/google_ai`)
//! - AWS Bedrock (`crates/bedrock`)
//! - Ollama (`crates/ollama`)
//! - Azure OpenAI
//!
//! # Tier 2 — Named Adapters (provider-specific quirks)
//! - Mistral, DeepSeek, xAI, Cohere, Groq, Fireworks AI, Together AI, etc.
//!
//! # Tier 3 — OpenAI-Compatible Generic Adapter (40+ providers)
//!
//! # Tier 4 — Aggregator Multipliers
//! - OpenRouter, Vercel AI, Cloudflare AI Gateway, etc.
//!
//! # Tier 5 — Local Models
//! - Ollama, LM Studio, llama.cpp, Candle-native

mod anthropic_adapter;
mod azure_adapter;
mod bedrock_adapter;
mod deepseek_adapter;
mod google_adapter;
mod local_adapter;
mod mistral_adapter;
mod ollama_adapter;
mod openai_adapter;
mod openai_compat_adapter;
mod openrouter_adapter;
mod xai_adapter;

pub use anthropic_adapter::AnthropicLlmAdapter;
pub use azure_adapter::AzureOpenAiAdapter;
pub use bedrock_adapter::BedrockLlmAdapter;
pub use deepseek_adapter::DeepSeekLlmAdapter;
pub use google_adapter::GoogleAiLlmAdapter;
pub use local_adapter::LocalLlmAdapter;
pub use mistral_adapter::MistralLlmAdapter;
pub use ollama_adapter::OllamaLlmAdapter;
pub use openai_adapter::OpenAiLlmAdapter;
pub use openai_compat_adapter::OpenAiCompatLlmAdapter;
pub use openrouter_adapter::OpenRouterLlmAdapter;
pub use xai_adapter::XAiLlmAdapter;

use dx_core::{DxProviderRegistry, LlmFallbackChain, LlmProvider};
use std::sync::Arc;

/// Register all available LLM providers into the DX provider registry.
///
/// This scans for configured API keys and registers providers that are ready.
/// Providers without keys are registered but marked as unavailable.
pub fn register_all_providers(registry: &DxProviderRegistry, config: &dx_core::DxConfig) {
    // Tier 1: Native adapters
    register_if_key(registry, config, "openai", |key| {
        Arc::new(OpenAiLlmAdapter::new(key)) as Arc<dyn LlmProvider>
    });
    register_if_key(registry, config, "anthropic", |key| {
        Arc::new(AnthropicLlmAdapter::new(key))
    });
    register_if_key(registry, config, "google-ai", |key| {
        Arc::new(GoogleAiLlmAdapter::new(key))
    });
    register_if_key(registry, config, "bedrock", |key| {
        Arc::new(BedrockLlmAdapter::new(key))
    });

    // Ollama is always available (local)
    registry.register_llm_provider(Arc::new(OllamaLlmAdapter::new(None)));

    register_if_key(registry, config, "azure-openai", |key| {
        Arc::new(AzureOpenAiAdapter::new(
            key,
            "https://YOUR-RESOURCE.openai.azure.com".into(),
            "2024-06-01".into(),
        ))
    });

    // Tier 2: Named adapters
    register_if_key(registry, config, "mistral", |key| {
        Arc::new(MistralLlmAdapter::new(key))
    });
    register_if_key(registry, config, "deepseek", |key| {
        Arc::new(DeepSeekLlmAdapter::new(key))
    });
    register_if_key(registry, config, "xai", |key| {
        Arc::new(XAiLlmAdapter::new(key))
    });

    // Tier 3: OpenAI-compatible providers
    for compat_config in dx_core::known_openai_compatible_providers() {
        let key = config
            .resolve_provider_key(&compat_config.provider_name.to_lowercase().replace(' ', "-"));
        registry.register_llm_provider(Arc::new(OpenAiCompatLlmAdapter::new(
            compat_config, key,
        )));
    }

    // Tier 4: Aggregators
    register_if_key(registry, config, "openrouter", |key| {
        Arc::new(OpenRouterLlmAdapter::new(key))
    });

    // Tier 5: Local
    registry.register_llm_provider(Arc::new(LocalLlmAdapter::new()));

    log::info!(
        "Registered {} LLM providers",
        registry.list_llm_providers().len()
    );
}

/// Build a default fallback chain: cloud primary → cloud backup → local.
pub fn build_default_fallback_chain(
    registry: &DxProviderRegistry,
) -> LlmFallbackChain {
    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();

    // Prefer configured cloud providers first
    for provider in registry.available_llm_providers() {
        providers.push(provider);
    }

    LlmFallbackChain::new(providers)
}

fn register_if_key<F>(
    registry: &DxProviderRegistry,
    config: &dx_core::DxConfig,
    provider_id: &str,
    factory: F,
) where
    F: FnOnce(String) -> Arc<dyn LlmProvider>,
{
    if let Some(key) = config.resolve_provider_key(provider_id) {
        registry.register_llm_provider(factory(key));
    }
}
