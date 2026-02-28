use crate::llm::aggregator::AggregatorModelIndex;
use crate::llm::custom::register_enterprise_custom_from_env;
use crate::llm::discovery::DiscoveryCatalog;
use crate::llm::enterprise::register_enterprise_openai_compatible_from_env;
use crate::llm::genai::GenAiProvider;
use crate::llm::generic::GenericProvider;
use crate::llm::models_dev::ModelsDevProvider;
use crate::llm::presets::openai_compatible_provider_presets;
use crate::llm::provider::LlmProvider;
use crate::llm::types::{AuthRequirement, ProviderCapabilities, ProviderMetadata};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<String, Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<P>(&mut self, provider: P)
    where
        P: LlmProvider + 'static,
    {
        self.providers.insert(provider.id().to_string(), Arc::new(provider));
    }

    pub fn insert(&mut self, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn LlmProvider>> {
        self.providers.get(id).cloned()
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn register_default_genai_providers(&mut self) {
        let defaults = [
            // github_copilot is registered via register_enterprise_custom_providers() using GitHubCopilotProvider
            ("openai", "gpt-4o-mini", "major-cloud"),
            ("anthropic", "claude-3-5-haiku-latest", "major-cloud"),
            ("google", "gemini-2.5-flash", "major-cloud"),
            ("groq", "groq::llama-3.1-8b-instant", "fast-inference"),
            ("openrouter", "openrouter::openai/gpt-oss-20b", "aggregator"),
            ("ollama", "ollama::llama3", "local-runner"),
            ("together", "together::openai/gpt-oss-20b", "fast-inference"),
            ("mistral", "mistral::mistral-small-latest", "open-source-host"),
            ("cohere", "cohere::command-r7b-12-2024", "open-source-host"),
            ("deepseek", "deepseek::deepseek-chat", "open-source-host"),
            ("xai", "xai::grok-3-mini", "open-source-host"),
        ];

        for (id, model, category) in defaults {
            let metadata = ProviderMetadata {
                id: id.to_string(),
                name: id.to_string(),
                category: category.to_string(),
                auth_requirement: if id == "ollama" {
                    AuthRequirement::None
                } else {
                    AuthRequirement::BearerToken
                },
                capabilities: ProviderCapabilities {
                    chat: true,
                    streaming: true,
                    tools: true,
                    vision: true,
                    audio_input: true,
                    audio_output: false,
                    model_listing: true,
                },
                rate_limits: None,
                docs_url: None,
                website: None,
            };
            self.register(GenAiProvider::new(id, model, metadata));
        }
    }

    pub fn register_openai_compatible_presets(&mut self) -> usize {
        let presets = openai_compatible_provider_presets();
        let mut registered = 0usize;

        for preset in presets {
            // Special handling for OpenCode - no API key required
            if preset.id == "opencode" {
                if let Ok(provider) = crate::llm::opencode::OpenCodeProvider::new() {
                    self.register(provider);
                    registered += 1;
                }
                continue;
            }

            if preset.api_key_env.is_empty() {
                continue;
            }

            let Ok(api_key) = std::env::var(preset.api_key_env) else {
                continue;
            };

            if api_key.trim().is_empty() {
                continue;
            }

            let provider = GenericProvider::new(preset.id, preset.base_url, api_key)
                .with_metadata(preset.metadata());
            self.register(provider);
            registered += 1;
        }

        registered
    }

    pub fn register_from_discovery_catalog(&mut self, catalog: &DiscoveryCatalog) -> usize {
        self.register_from_discovery_catalog_with_resolver(catalog, |env_name| {
            std::env::var(env_name).ok()
        })
    }

    pub fn register_from_discovery_catalog_with_resolver<F>(
        &mut self,
        catalog: &DiscoveryCatalog,
        mut api_key_resolver: F,
    ) -> usize
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut registered = 0usize;

        for provider in &catalog.providers {
            if !provider.openai_compatible {
                continue;
            }

            let Some(base_url) = provider.base_url.clone() else {
                continue;
            };

            let env_name =
                provider.default_api_key_env().unwrap_or_else(|| "OPENAI_API_KEY".to_string());
            let Some(api_key) = api_key_resolver(&env_name) else {
                continue;
            };

            if api_key.trim().is_empty() {
                continue;
            }

            let capabilities = ProviderCapabilities {
                chat: true,
                streaming: true,
                tools: provider.supports_tools,
                vision: provider.supports_vision,
                audio_input: provider.supports_audio,
                audio_output: false,
                model_listing: true,
            };

            let metadata = ProviderMetadata {
                id: provider.id.clone(),
                name: provider.name.clone(),
                category: provider.category.clone(),
                auth_requirement: AuthRequirement::BearerToken,
                capabilities,
                rate_limits: None,
                docs_url: provider.docs_url.clone(),
                website: provider.website.clone(),
            };

            let generic =
                GenericProvider::new(&provider.id, base_url, api_key).with_metadata(metadata);
            self.register(generic);
            registered += 1;
        }

        registered
    }

    pub fn register_models_dev_providers(&mut self, providers: &[ModelsDevProvider]) -> usize {
        self.register_models_dev_providers_with_resolver(providers, |env_name| {
            std::env::var(env_name).ok()
        })
    }

    pub fn register_models_dev_providers_with_resolver<F>(
        &mut self,
        providers: &[ModelsDevProvider],
        mut api_key_resolver: F,
    ) -> usize
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut registered = 0usize;
        for provider in providers {
            if !provider.openai_compatible {
                continue;
            }

            let Some(env_name) = provider.default_api_key_env() else {
                continue;
            };

            let Some(api_key) = api_key_resolver(&env_name) else {
                continue;
            };

            if api_key.trim().is_empty() {
                continue;
            }

            let Some(generic) = provider.clone().into_generic_provider(api_key) else {
                continue;
            };

            self.register(generic);
            registered += 1;
        }

        registered
    }

    pub fn register_enterprise_custom_providers(&mut self) -> usize {
        let mut providers = register_enterprise_custom_from_env();
        let enterprise_openai = register_enterprise_openai_compatible_from_env();
        for provider in enterprise_openai {
            providers.push(Box::new(provider));
        }

        let mut registered = 0usize;
        for provider in providers {
            self.insert(Arc::from(provider));
            registered += 1;
        }
        registered
    }

    pub fn route_provider_for_model(
        &self,
        model_index: &AggregatorModelIndex,
        model: &str,
    ) -> Option<Arc<dyn LlmProvider>> {
        let provider_id = model_index.route_provider_for_model(model)?;
        self.get(&provider_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::aggregator::AggregatorModelIndex;
    use crate::llm::discovery::DiscoveredProvider;
    use crate::llm::generic::GenericProvider;
    use crate::llm::models_dev::ModelsDevProvider;

    #[test]
    fn register_and_get_provider() {
        let provider = GenericProvider::new("groq", "https://api.groq.com/openai", "test");
        let mut registry = ProviderRegistry::new();
        registry.register(provider);

        assert_eq!(registry.len(), 1);
        assert!(registry.get("groq").is_some());
        assert_eq!(registry.list_ids(), vec!["groq".to_string()]);
    }

    #[test]
    fn registers_default_genai_providers() {
        let mut registry = ProviderRegistry::new();
        registry.register_default_genai_providers();

        assert!(registry.get("openai").is_some());
        assert!(registry.get("anthropic").is_some());
        assert!(registry.get("google").is_some());
        assert!(registry.get("groq").is_some());
        assert!(registry.get("openrouter").is_some());
        assert!(registry.get("ollama").is_some());
        assert!(registry.get("together").is_some());
        assert!(registry.get("mistral").is_some());
        assert!(registry.get("cohere").is_some());
        assert!(registry.get("deepseek").is_some());
        assert!(registry.get("xai").is_some());
    }

    #[test]
    fn bulk_presets_skip_without_env() {
        let mut registry = ProviderRegistry::new();
        let count = registry.register_openai_compatible_presets();
        // OpenCode is intentionally keyless for its free promotional models.
        // All other openai-compatible presets require API keys and should be skipped.
        assert_eq!(count, 1);
        assert!(registry.get("opencode").is_some());
    }

    #[tokio::test]
    async fn six_provider_regression_smoke() {
        let mut registry = ProviderRegistry::new();
        registry.register_default_genai_providers();

        let expected = [
            "openai",
            "anthropic",
            "google",
            "groq",
            "openrouter",
            "ollama",
        ];
        for id in expected {
            let provider = registry.get(id).expect("provider exists");
            let models = provider.get_models().await.expect("models should resolve");
            assert!(!models.is_empty());
        }
    }

    #[tokio::test]
    async fn optional_ollama_connection_test() {
        if std::env::var("DX_TEST_OLLAMA").ok().as_deref() != Some("1") {
            return;
        }

        let ollama = ollama_rs::Ollama::default();
        let result = ollama.list_local_models().await;
        assert!(result.is_ok(), "ollama should be reachable when DX_TEST_OLLAMA=1");
    }

    #[test]
    fn register_from_discovery_uses_resolver() {
        let catalog = DiscoveryCatalog {
            generated_at_unix: 0,
            providers: vec![DiscoveredProvider {
                id: "fireworks".to_string(),
                name: "Fireworks AI".to_string(),
                source: "litellm".to_string(),
                category: "fast-inference".to_string(),
                base_url: Some("https://api.fireworks.ai/inference/v1".to_string()),
                model_count: 10,
                openai_compatible: true,
                supports_tools: true,
                supports_vision: false,
                supports_audio: false,
                max_context_window: Some(128_000),
                avg_input_price_per_million: Some(0.5),
                avg_output_price_per_million: Some(1.5),
                docs_url: None,
                website: None,
                sample_models: vec!["accounts/fireworks/models/qwen3-30b-a3b".to_string()],
            }],
        };

        let mut registry = ProviderRegistry::new();
        let registered = registry.register_from_discovery_catalog_with_resolver(&catalog, |env| {
            if env == "FIREWORKS_API_KEY" {
                Some("token-1".to_string())
            } else {
                None
            }
        });

        assert_eq!(registered, 1);
        assert!(registry.get("fireworks").is_some());
    }

    #[test]
    fn register_models_dev_providers_works() {
        let providers = vec![ModelsDevProvider {
            slug: "nvidia".to_string(),
            name: "NVIDIA NIM".to_string(),
            category: "major-cloud".to_string(),
            base_url: Some("https://integrate.api.nvidia.com/v1".to_string()),
            model_count: 5,
            supports_tools: true,
            supports_vision: true,
            supports_audio: false,
            max_context_window: Some(128_000),
            avg_input_price_per_million: Some(0.2),
            avg_output_price_per_million: Some(0.8),
            docs_url: None,
            website: None,
            sample_models: vec!["meta/llama-3.1-70b-instruct".to_string()],
            openai_compatible: true,
        }];

        let mut registry = ProviderRegistry::new();
        let count = registry.register_models_dev_providers_with_resolver(&providers, |env| {
            if env == "NVIDIA_API_KEY" {
                Some("token-2".to_string())
            } else {
                None
            }
        });

        assert_eq!(count, 1);
        assert!(registry.get("nvidia").is_some());
    }

    #[test]
    fn register_enterprise_custom_providers_without_env_is_noop() {
        let mut registry = ProviderRegistry::new();
        let count = registry.register_enterprise_custom_providers();
        assert_eq!(count, 0);
    }

    #[test]
    fn routes_provider_by_model_index() {
        let mut registry = ProviderRegistry::new();
        registry.register(GenericProvider::new(
            "vercel-ai-gateway",
            "https://ai-gateway.vercel.sh/v1",
            "token",
        ));

        let mut index = AggregatorModelIndex::default();
        index.model_to_providers.insert(
            "anthropic/claude-sonnet-4.5".to_string(),
            vec!["vercel-ai-gateway".to_string()],
        );

        let provider = registry.route_provider_for_model(&index, "anthropic/claude-sonnet-4.5");
        assert!(provider.is_some());
        assert_eq!(provider.expect("provider").id(), "vercel-ai-gateway");
    }
}
