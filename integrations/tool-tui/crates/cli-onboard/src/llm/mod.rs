pub mod aggregator;
pub mod config;
pub mod copilot;
pub mod copilot_accounts;
pub mod copilot_bootstrap;
pub mod custom;
pub mod discovery;
pub mod enterprise;
pub mod error;
pub mod genai;
pub mod generic;
pub mod models_dev;
pub mod openai_compatible;
pub mod opencode;
pub mod presets;
pub mod provider;
pub mod registry;
pub mod types;

pub use aggregator::{
    AggregatorEndpoint, AggregatorFormat, AggregatorModelIndex, fetch_aggregator_model_lists,
};
pub use config::{ProviderConfigEntry, ProviderConfigFile, ProviderProfileEntry};
pub use copilot::{
    CopilotApiSurface, CopilotOAuthConfig, CopilotOAuthFlow, CopilotTier,
    detect_copilot_tier_from_env, endpoint_for_model, model_api_surface,
};
pub use copilot_accounts::{CopilotAccountId, CopilotAccountManager, CopilotAccountProfile};
pub use copilot_bootstrap::{CopilotBootstrapResult, ensure_github_copilot_ready_interactive};
pub use custom::{
    AzureOpenAiProvider, GitHubCopilotProvider, VertexAiProvider,
    register_enterprise_custom_from_env,
};
pub use discovery::{
    CatalogFilter, DiscoveredProvider, DiscoveryCatalog, fetch_litellm_providers,
    fetch_models_dev_providers, fetch_openrouter_providers, refresh_discovery_catalog,
    search_catalog, search_catalog_advanced, start_catalog_auto_refresh,
};
pub use enterprise::{
    ibm_watsonx_provider, register_enterprise_openai_compatible_from_env, sap_ai_core_provider,
    snowflake_cortex_provider,
};
pub use error::{ModelNotFoundError, ProviderError, RateLimitError};
pub use genai::GenAiProvider;
pub use generic::GenericProvider;
pub use models_dev::{ModelsDevProvider, map_models_dev_to_generic};
pub use openai_compatible::OpenAiCompatibleProvider;
pub use opencode::{FREE_MODELS as OPENCODE_FREE_MODELS, OpenCodeProvider};
pub use presets::{ProviderPreset, openai_compatible_provider_presets};
pub use provider::LlmProvider;
pub use registry::ProviderRegistry;
pub use types::{
    AuthRequirement, ChatChunk, ChatMessage, ChatRequest, ChatResponse, MessageContent, ModelInfo,
    ProviderCapabilities, ProviderMetadata, RateLimitInfo,
};
