//! DX LLM Bridge — connects existing Zed provider crates to dx_core's LlmProvider trait.
//!
//! This crate provides adapter implementations that bridge the gap between Zed's existing
//! LLM provider crates (open_ai, anthropic, google_ai, etc.) and the unified `LlmProvider`
//! trait defined in `dx_core`. Each adapter wraps an HTTP client and provider-specific
//! configuration to implement the common interface.

pub mod anthropic_adapter;
pub mod azure_openai_adapter;
pub mod bedrock_adapter;
pub mod cohere_adapter;
pub mod deepseek_adapter;
pub mod fireworks_adapter;
pub mod google_ai_adapter;
pub mod groq_adapter;
pub mod huggingface_adapter;
pub mod lm_studio_adapter;
pub mod mistral_adapter;
pub mod nvidia_nim_adapter;
pub mod ollama_adapter;
pub mod open_router_adapter;
pub mod openai_adapter;
pub mod openai_compat_adapter;
pub mod replicate_adapter;
pub mod together_adapter;
pub mod vercel_adapter;
pub mod x_ai_adapter;

pub use anthropic_adapter::AnthropicAdapter;
pub use azure_openai_adapter::AzureOpenAiAdapter;
pub use bedrock_adapter::BedrockAdapter;
pub use cohere_adapter::CohereAdapter;
pub use deepseek_adapter::DeepSeekAdapter;
pub use fireworks_adapter::FireworksAdapter;
pub use google_ai_adapter::GoogleAiAdapter;
pub use groq_adapter::GroqAdapter;
pub use huggingface_adapter::HuggingFaceAdapter;
pub use lm_studio_adapter::LmStudioAdapter;
pub use mistral_adapter::MistralAdapter;
pub use nvidia_nim_adapter::NvidiaNimAdapter;
pub use ollama_adapter::OllamaAdapter;
pub use open_router_adapter::OpenRouterAdapter;
pub use openai_adapter::OpenAiAdapter;
pub use openai_compat_adapter::OpenAiCompatAdapter;
pub use replicate_adapter::ReplicateAdapter;
pub use together_adapter::TogetherAdapter;
pub use vercel_adapter::VercelAdapter;
pub use x_ai_adapter::XAiAdapter;

use dx_core::llm_provider::LlmProvider;
use std::sync::Arc;

/// Create all configured LLM provider adapters based on available API keys and settings.
pub fn create_all_adapters(
    http_client: Arc<dyn http_client::HttpClient>,
) -> Vec<Arc<dyn LlmProvider>> {
    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();

    // Tier 1: Native adapters
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        providers.push(Arc::new(OpenAiAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        providers.push(Arc::new(AnthropicAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("GOOGLE_AI_API_KEY") {
        providers.push(Arc::new(GoogleAiAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("AZURE_OPENAI_API_KEY") {
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").unwrap_or_default();
        let api_version = std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-06-01".to_string());
        providers.push(Arc::new(AzureOpenAiAdapter::new(
            key,
            endpoint,
            api_version,
            http_client.clone(),
        )));
    }

    // Ollama is always available if running locally
    providers.push(Arc::new(OllamaAdapter::new(
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string()),
        http_client.clone(),
    )));

    // Tier 2: Named adapters
    if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
        providers.push(Arc::new(MistralAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        providers.push(Arc::new(DeepSeekAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("XAI_API_KEY") {
        providers.push(Arc::new(XAiAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("COHERE_API_KEY") {
        providers.push(Arc::new(CohereAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("GROQ_API_KEY") {
        providers.push(Arc::new(GroqAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("FIREWORKS_API_KEY") {
        providers.push(Arc::new(FireworksAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("TOGETHER_API_KEY") {
        providers.push(Arc::new(TogetherAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("HUGGINGFACE_API_KEY") {
        providers.push(Arc::new(HuggingFaceAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("NVIDIA_NIM_API_KEY") {
        providers.push(Arc::new(NvidiaNimAdapter::new(key, http_client.clone())));
    }
    if let Ok(key) = std::env::var("REPLICATE_API_KEY") {
        providers.push(Arc::new(ReplicateAdapter::new(key, http_client.clone())));
    }

    // LM Studio local
    providers.push(Arc::new(LmStudioAdapter::new(
        std::env::var("LM_STUDIO_HOST")
            .unwrap_or_else(|_| "http://localhost:1234".to_string()),
        http_client.clone(),
    )));

    // Tier 4: Aggregators
    if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
        providers.push(Arc::new(OpenRouterAdapter::new(key, http_client.clone())));
    }

    // Tier 3: OpenAI-compatible generic — add all known providers with env keys
    for config in dx_core::llm_provider::known_openai_compatible_providers() {
        let env_key = format!(
            "{}_API_KEY",
            config
                .provider_name
                .to_uppercase()
                .replace(' ', "_")
                .replace('.', "_")
        );
        if let Ok(key) = std::env::var(&env_key) {
            let mut cfg = config.clone();
            cfg.api_key = Some(key);
            providers.push(Arc::new(OpenAiCompatAdapter::new(cfg, http_client.clone())));
        }
    }

    providers
}
