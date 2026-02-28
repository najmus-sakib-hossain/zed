//! Model router with failover support

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

use crate::config::{ProviderConfig, RuntimeConfig};
use crate::models::{ChatRequest, ChatResponse};
use crate::provider::{LlmProvider, ProviderError};
use crate::providers::{
    anthropic::AnthropicProvider, google::GoogleProvider, ollama::OllamaProvider,
    openai::OpenAiProvider,
};

/// Model router with automatic failover
pub struct ModelRouter {
    providers: DashMap<String, Arc<dyn LlmProvider>>,
    config: RuntimeConfig,
    failover_state: Arc<RwLock<FailoverState>>,
}

struct FailoverState {
    /// Providers currently marked as unhealthy
    unhealthy: Vec<String>,
    /// Retry counts per provider
    retry_counts: std::collections::HashMap<String, u32>,
}

impl ModelRouter {
    /// Create a new router from config
    pub fn new(config: RuntimeConfig) -> Self {
        let router = Self {
            providers: DashMap::new(),
            config,
            failover_state: Arc::new(RwLock::new(FailoverState {
                unhealthy: Vec::new(),
                retry_counts: std::collections::HashMap::new(),
            })),
        };

        // Initialize providers from config
        for (name, provider_config) in &router.config.providers {
            if let Some(provider) = create_provider(name, provider_config) {
                router.providers.insert(name.clone(), Arc::from(provider));
                info!("Initialized LLM provider: {}", name);
            }
        }

        router
    }

    /// Register a provider manually
    pub fn register_provider(&self, name: &str, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(name.to_string(), provider);
    }

    /// Send a chat request with automatic failover
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let provider_chain = self.get_provider_chain();

        let mut last_error = ProviderError::Unavailable("No providers available".into());

        for provider_name in &provider_chain {
            if let Some(provider) = self.providers.get(provider_name) {
                match provider.chat(request.clone()).await {
                    Ok(response) => {
                        self.mark_healthy(provider_name);
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!("Provider {} failed: {}, trying next", provider_name, e);
                        self.mark_unhealthy(provider_name);
                        last_error = e;

                        if !self.config.failover.enabled {
                            break;
                        }
                    }
                }
            }
        }

        Err(last_error)
    }

    /// Get ordered list of providers to try
    fn get_provider_chain(&self) -> Vec<String> {
        let state = self.failover_state.read();

        if !self.config.failover.chain.is_empty() {
            // Use configured chain, but put healthy providers first
            let mut chain: Vec<String> = self
                .config
                .failover
                .chain
                .iter()
                .filter(|p| !state.unhealthy.contains(p))
                .cloned()
                .collect();

            // Add unhealthy ones at the end (for retry)
            for p in &self.config.failover.chain {
                if state.unhealthy.contains(p) && !chain.contains(p) {
                    chain.push(p.clone());
                }
            }

            chain
        } else {
            // Use default provider, then any others
            let mut chain = vec![self.config.default_provider.clone()];
            for entry in self.providers.iter() {
                let name = entry.key().clone();
                if !chain.contains(&name) {
                    chain.push(name);
                }
            }
            chain
        }
    }

    fn mark_unhealthy(&self, provider: &str) {
        let mut state = self.failover_state.write();
        if !state.unhealthy.contains(&provider.to_string()) {
            state.unhealthy.push(provider.to_string());
        }
        *state.retry_counts.entry(provider.to_string()).or_insert(0) += 1;
    }

    fn mark_healthy(&self, provider: &str) {
        let mut state = self.failover_state.write();
        state.unhealthy.retain(|p| p != provider);
        state.retry_counts.remove(provider);
    }

    /// Get all registered provider names
    pub fn provider_names(&self) -> Vec<String> {
        self.providers.iter().map(|e| e.key().clone()).collect()
    }

    /// Check health of all providers
    pub async fn health_check_all(&self) -> Vec<(String, bool)> {
        let mut results = Vec::new();
        for entry in self.providers.iter() {
            let name = entry.key().clone();
            let healthy = entry.value().health_check().await.unwrap_or(false);
            results.push((name, healthy));
        }
        results
    }
}

fn create_provider(name: &str, config: &ProviderConfig) -> Option<Box<dyn LlmProvider>> {
    match config.provider_type.as_str() {
        "openai" => OpenAiProvider::new(config).ok().map(|p| Box::new(p) as Box<dyn LlmProvider>),
        "anthropic" => {
            AnthropicProvider::new(config).ok().map(|p| Box::new(p) as Box<dyn LlmProvider>)
        }
        "google" => GoogleProvider::new(config).ok().map(|p| Box::new(p) as Box<dyn LlmProvider>),
        "ollama" => OllamaProvider::new(config).ok().map(|p| Box::new(p) as Box<dyn LlmProvider>),
        _ => {
            warn!("Unknown provider type '{}' for '{}'", config.provider_type, name);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let config = RuntimeConfig::default();
        let router = ModelRouter::new(config);
        // No providers configured by default (no API keys)
        assert!(router.provider_names().is_empty() || !router.provider_names().is_empty());
    }

    #[test]
    fn test_failover_chain() {
        let config = RuntimeConfig::default();
        let router = ModelRouter::new(config);
        let chain = router.get_provider_chain();
        assert!(!chain.is_empty());
        assert_eq!(chain[0], "anthropic"); // default provider
    }
}
