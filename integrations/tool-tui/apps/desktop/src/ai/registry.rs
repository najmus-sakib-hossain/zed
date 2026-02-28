use super::credentials::CredentialStore;
use super::{AiProvider, AiProviderKind};
use std::collections::HashMap;

/// AI Registry - manages all available providers and models
pub struct AiRegistry {
    providers: HashMap<AiProviderKind, AiProvider>,
    active_provider: Option<AiProviderKind>,
    active_model: Option<String>,
    pub credentials: CredentialStore,
}

impl AiRegistry {
    pub fn new() -> Self {
        let mut providers = HashMap::new();
        let credentials = CredentialStore::load();

        // Initialize all providers
        for kind in AiProviderKind::all() {
            providers.insert(*kind, AiProvider::new(*kind));
        }

        Self {
            providers,
            active_provider: Some(AiProviderKind::Anthropic),
            active_model: Some("claude-sonnet-4-20250514".to_string()),
            credentials,
        }
    }

    pub fn get_provider(&self, kind: AiProviderKind) -> Option<&AiProvider> {
        self.providers.get(&kind)
    }

    #[allow(dead_code)]
    pub fn get_provider_mut(&mut self, kind: AiProviderKind) -> Option<&mut AiProvider> {
        self.providers.get_mut(&kind)
    }

    #[allow(dead_code)]
    pub fn all_providers(&self) -> impl Iterator<Item = &AiProvider> {
        self.providers.values()
    }

    pub fn active_provider(&self) -> Option<&AiProvider> {
        self.active_provider.and_then(|kind| self.providers.get(&kind))
    }

    pub fn active_provider_kind(&self) -> Option<AiProviderKind> {
        self.active_provider
    }

    pub fn set_active_provider(&mut self, kind: AiProviderKind) {
        self.active_provider = Some(kind);
        // Set the first model as active when switching providers
        if let Some(provider) = self.providers.get(&kind) {
            self.active_model = provider.models.first().map(|m| m.id.clone());
        }
    }

    pub fn active_model(&self) -> Option<String> {
        self.active_model.clone()
    }

    pub fn set_active_model(&mut self, model_id: String) {
        self.active_model = Some(model_id);
    }

    /// Check if the active provider is authenticated.
    pub fn is_active_authenticated(&self) -> bool {
        if let Some(kind) = self.active_provider {
            if !kind.needs_api_key() {
                return true; // Local providers don't need API keys
            }
            self.credentials.has_api_key(kind.id())
        } else {
            false
        }
    }

    /// Get API key for the active provider.
    pub fn active_api_key(&self) -> Option<&str> {
        self.active_provider.and_then(|kind| self.credentials.get_api_key(kind.id()))
    }

    /// Set API key for a provider.
    pub fn set_api_key(&mut self, provider_id: &str, key: String) {
        self.credentials.set_api_key(provider_id, key);
    }

    #[allow(dead_code)]
    pub fn get_active_model_info(&self) -> Option<(AiProviderKind, String)> {
        self.active_provider
            .zip(self.active_model.as_ref())
            .map(|(provider, model)| (provider, model.clone()))
    }
}

impl Default for AiRegistry {
    fn default() -> Self {
        Self::new()
    }
}
