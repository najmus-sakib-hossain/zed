//! DX Providers — Bridges existing Zed provider crates to DX unified traits.
//!
//! This crate wires the existing Zed LLM provider crates (open_ai, anthropic,
//! google_ai, ollama, etc.) to the DX `LlmProvider` trait from `dx_core`.
//!
//! ## Architecture
//!
//! Each adapter wraps an existing Zed provider crate and implements `LlmProvider`.
//! This means all 100+ providers accessible through Zed's existing infra become
//! available to DX's unified fallback chains, cost tracking, and rate limiting.
//!
//! ## Provider Tiers
//!
//! - **Tier 1 (Native):** OpenAI, Anthropic, Google AI, Bedrock, Ollama, Azure OpenAI
//! - **Tier 2 (Named):** Mistral, DeepSeek, xAI, Cohere, Groq, Fireworks, Together, HuggingFace
//! - **Tier 3 (OpenAI-Compatible):** 40+ providers via generic adapter
//! - **Tier 4 (Aggregator):** OpenRouter, Vercel, Cloudflare AI Gateway
//! - **Tier 5 (Local):** Ollama, LM Studio, llama.cpp, Candle-native

mod anthropic_adapter;
mod azure_openai_adapter;
mod bedrock_adapter;
mod cohere_adapter;
mod deepseek_adapter;
mod fireworks_adapter;
mod google_ai_adapter;
mod groq_adapter;
mod huggingface_adapter;
mod lm_studio_adapter;
mod mistral_adapter;
mod nvidia_nim_adapter;
mod ollama_adapter;
mod open_router_adapter;
mod openai_adapter;
mod openai_compatible;
mod provider_bridge;
mod replicate_llm_adapter;
mod together_adapter;
mod vercel_adapter;
mod x_ai_adapter;

pub use anthropic_adapter::AnthropicLlmProvider;
pub use azure_openai_adapter::AzureOpenAiLlmProvider;
pub use bedrock_adapter::BedrockLlmProvider;
pub use cohere_adapter::CohereLlmProvider;
pub use deepseek_adapter::DeepSeekLlmProvider;
pub use fireworks_adapter::FireworksLlmProvider;
pub use google_ai_adapter::GoogleAiLlmProvider;
pub use groq_adapter::GroqLlmProvider;
pub use huggingface_adapter::HuggingFaceLlmProvider;
pub use lm_studio_adapter::LmStudioLlmProvider;
pub use mistral_adapter::MistralLlmProvider;
pub use nvidia_nim_adapter::NvidiaNimLlmProvider;
pub use ollama_adapter::OllamaLlmProvider;
pub use open_router_adapter::OpenRouterLlmProvider;
pub use openai_adapter::OpenAiLlmProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
pub use provider_bridge::ProviderBridge;
pub use replicate_llm_adapter::ReplicateLlmProvider;
pub use together_adapter::TogetherLlmProvider;
pub use vercel_adapter::VercelLlmProvider;
pub use x_ai_adapter::XAiLlmProvider;

use dx_core::{DxProviderRegistry, LlmProvider};
use std::sync::Arc;

/// Register all available provider adapters into the DX provider registry.
///
/// This scans for configured API keys and available local endpoints,
/// then creates adapter instances for each discovered provider.
pub fn register_all_providers(registry: &DxProviderRegistry) {
    log::info!("DX Providers: registering all available provider adapters...");

    // Tier 1 — Native adapters
    if let Some(provider) = OpenAiLlmProvider::from_env() {
        log::info!("  Registered: OpenAI (native)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = AnthropicLlmProvider::from_env() {
        log::info!("  Registered: Anthropic (native)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = GoogleAiLlmProvider::from_env() {
        log::info!("  Registered: Google AI (native)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = BedrockLlmProvider::from_env() {
        log::info!("  Registered: AWS Bedrock (native)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = AzureOpenAiLlmProvider::from_env() {
        log::info!("  Registered: Azure OpenAI (native)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = OllamaLlmProvider::detect() {
        log::info!("  Registered: Ollama (local)");
        registry.register_llm_provider(Arc::new(provider));
    }

    // Tier 2 — Named adapters
    if let Some(provider) = MistralLlmProvider::from_env() {
        log::info!("  Registered: Mistral AI");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = DeepSeekLlmProvider::from_env() {
        log::info!("  Registered: DeepSeek");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = XAiLlmProvider::from_env() {
        log::info!("  Registered: xAI (Grok)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = GroqLlmProvider::from_env() {
        log::info!("  Registered: Groq");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = FireworksLlmProvider::from_env() {
        log::info!("  Registered: Fireworks AI");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = TogetherLlmProvider::from_env() {
        log::info!("  Registered: Together AI");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = CohereLlmProvider::from_env() {
        log::info!("  Registered: Cohere");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = NvidiaNimLlmProvider::from_env() {
        log::info!("  Registered: NVIDIA NIM");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = HuggingFaceLlmProvider::from_env() {
        log::info!("  Registered: Hugging Face");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = ReplicateLlmProvider::from_env() {
        log::info!("  Registered: Replicate");
        registry.register_llm_provider(Arc::new(provider));
    }

    // Tier 3 — OpenAI-compatible providers (auto-detected from env vars)
    for compat in OpenAiCompatibleProvider::discover_from_env() {
        log::info!("  Registered: {} (OpenAI-compatible)", compat.name());
        registry.register_llm_provider(Arc::new(compat));
    }

    // Tier 4 — Aggregators
    if let Some(provider) = OpenRouterLlmProvider::from_env() {
        log::info!("  Registered: OpenRouter (aggregator)");
        registry.register_llm_provider(Arc::new(provider));
    }

    if let Some(provider) = VercelLlmProvider::from_env() {
        log::info!("  Registered: Vercel AI (aggregator)");
        registry.register_llm_provider(Arc::new(provider));
    }

    // Tier 5 — Local
    if let Some(provider) = LmStudioLlmProvider::from_env() {
        log::info!("  Registered: LM Studio (local)");
        registry.register_llm_provider(Arc::new(provider));
    }

    log::info!("DX Providers: registration complete");
}
