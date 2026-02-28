use crate::llm::discovery::DiscoveredProvider;
use crate::llm::generic::GenericProvider;
use crate::llm::types::{AuthRequirement, ProviderCapabilities, ProviderMetadata};

#[derive(Debug, Clone, PartialEq)]
pub struct ModelsDevProvider {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub base_url: Option<String>,
    pub model_count: usize,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub max_context_window: Option<u32>,
    pub avg_input_price_per_million: Option<f64>,
    pub avg_output_price_per_million: Option<f64>,
    pub docs_url: Option<String>,
    pub website: Option<String>,
    pub sample_models: Vec<String>,
    pub openai_compatible: bool,
}

impl ModelsDevProvider {
    pub fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.slug.clone(),
            name: self.name.clone(),
            category: self.category.clone(),
            auth_requirement: AuthRequirement::BearerToken,
            capabilities: ProviderCapabilities {
                chat: true,
                streaming: true,
                tools: self.supports_tools,
                vision: self.supports_vision,
                audio_input: self.supports_audio,
                audio_output: false,
                model_listing: true,
            },
            rate_limits: None,
            docs_url: self.docs_url.clone(),
            website: self.website.clone(),
        }
    }

    pub fn into_generic_provider(self, api_key: String) -> Option<GenericProvider> {
        if !self.openai_compatible {
            return None;
        }

        let metadata = self.metadata();
        let provider_id = self.slug.clone();
        let base_url = self.base_url?;

        Some(GenericProvider::new(provider_id, base_url, api_key).with_metadata(metadata))
    }

    pub fn default_api_key_env(&self) -> Option<String> {
        let normalized = self.slug.to_ascii_uppercase().replace('-', "_");
        match normalized.as_str() {
            "OPENAI" => Some("OPENAI_API_KEY".to_string()),
            "OPENROUTER" => Some("OPENROUTER_API_KEY".to_string()),
            "GROQ" => Some("GROQ_API_KEY".to_string()),
            "ANTHROPIC" => Some("ANTHROPIC_API_KEY".to_string()),
            "GOOGLE" | "GEMINI" => Some("GEMINI_API_KEY".to_string()),
            "OLLAMA" | "LMSTUDIO" | "LOCALAI" | "VLLM" | "LLAMA_CPP" => None,
            _ => Some(format!("{}_API_KEY", normalized)),
        }
    }
}

impl From<DiscoveredProvider> for ModelsDevProvider {
    fn from(provider: DiscoveredProvider) -> Self {
        Self {
            slug: provider.id,
            name: provider.name,
            category: provider.category,
            base_url: provider.base_url,
            model_count: provider.model_count,
            supports_tools: provider.supports_tools,
            supports_vision: provider.supports_vision,
            supports_audio: provider.supports_audio,
            max_context_window: provider.max_context_window,
            avg_input_price_per_million: provider.avg_input_price_per_million,
            avg_output_price_per_million: provider.avg_output_price_per_million,
            docs_url: provider.docs_url,
            website: provider.website,
            sample_models: provider.sample_models,
            openai_compatible: provider.openai_compatible,
        }
    }
}

pub fn map_models_dev_to_generic<F>(
    providers: &[ModelsDevProvider],
    mut api_key_resolver: F,
) -> Vec<GenericProvider>
where
    F: FnMut(&ModelsDevProvider) -> Option<String>,
{
    providers
        .iter()
        .filter_map(|provider| {
            let api_key = api_key_resolver(provider)?;
            if api_key.trim().is_empty() {
                return None;
            }
            provider.clone().into_generic_provider(api_key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::LlmProvider;

    #[test]
    fn models_dev_provider_maps_to_generic() {
        let provider = ModelsDevProvider {
            slug: "fireworks".to_string(),
            name: "Fireworks AI".to_string(),
            category: "fast-inference".to_string(),
            base_url: Some("https://api.fireworks.ai/inference/v1".to_string()),
            model_count: 12,
            supports_tools: true,
            supports_vision: true,
            supports_audio: false,
            max_context_window: Some(128_000),
            avg_input_price_per_million: Some(0.4),
            avg_output_price_per_million: Some(1.2),
            docs_url: Some("https://docs.fireworks.ai".to_string()),
            website: Some("https://fireworks.ai".to_string()),
            sample_models: vec!["accounts/fireworks/models/qwen3-30b-a3b".to_string()],
            openai_compatible: true,
        };

        let generic = provider
            .clone()
            .into_generic_provider("token-1".to_string())
            .expect("provider should map");
        assert_eq!(generic.id(), "fireworks");
        assert_eq!(generic.base_url(), "https://api.fireworks.ai/inference/v1");
    }

    #[test]
    fn map_models_dev_to_generic_filters_non_compatible() {
        let providers = vec![
            ModelsDevProvider {
                slug: "openai".to_string(),
                name: "OpenAI".to_string(),
                category: "major-cloud".to_string(),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model_count: 10,
                supports_tools: true,
                supports_vision: true,
                supports_audio: true,
                max_context_window: Some(200_000),
                avg_input_price_per_million: Some(1.0),
                avg_output_price_per_million: Some(3.0),
                docs_url: None,
                website: None,
                sample_models: vec!["gpt-4o-mini".to_string()],
                openai_compatible: true,
            },
            ModelsDevProvider {
                slug: "custom-non-openai".to_string(),
                name: "Custom".to_string(),
                category: "specialized".to_string(),
                base_url: Some("https://api.custom.ai".to_string()),
                model_count: 2,
                supports_tools: false,
                supports_vision: false,
                supports_audio: false,
                max_context_window: Some(8192),
                avg_input_price_per_million: None,
                avg_output_price_per_million: None,
                docs_url: None,
                website: None,
                sample_models: vec![],
                openai_compatible: false,
            },
        ];

        let mapped = map_models_dev_to_generic(&providers, |_| Some("test-token".to_string()));
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].id(), "openai");
    }
}
