//! Unified TTS manager — routes requests to best available provider,
//! with fallback chain: Local (Piper) → ElevenLabs → OpenAI TTS → Azure.

use anyhow::Result;
use dx_core::{
    DeviceTier, TtsOutput, TtsProvider, TtsProviderId, TtsProviderLocation, TtsRequest, VoiceInfo,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Routing strategy for TTS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TtsRouteStrategy {
    /// Always try local first.
    LocalFirst,
    /// Always use cloud for max quality.
    CloudFirst,
    /// Pick cheapest available provider.
    CostOptimal,
    /// Use a fixed provider.
    Fixed,
}

/// Configuration for the TTS manager.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TtsManagerConfig {
    pub strategy: TtsRouteStrategy,
    pub preferred_provider: Option<TtsProviderId>,
    pub preferred_voice: Option<String>,
    pub device_tier: DeviceTier,
    pub max_cost_per_request_usd: Option<f64>,
}

impl Default for TtsManagerConfig {
    fn default() -> Self {
        Self {
            strategy: TtsRouteStrategy::LocalFirst,
            preferred_provider: None,
            preferred_voice: None,
            device_tier: DeviceTier::detect(),
            max_cost_per_request_usd: None,
        }
    }
}

/// Manages multiple TTS providers and routes requests intelligently.
pub struct TtsManager {
    providers: HashMap<TtsProviderId, Arc<dyn TtsProvider>>,
    fallback_order: Vec<TtsProviderId>,
    config: TtsManagerConfig,
}

impl TtsManager {
    pub fn new(config: TtsManagerConfig) -> Self {
        Self {
            providers: HashMap::new(),
            fallback_order: Vec::new(),
            config,
        }
    }

    /// Register a provider and place it in the fallback chain.
    pub fn register_provider(&mut self, provider: Arc<dyn TtsProvider>) {
        let id = provider.id().clone();
        self.providers.insert(id.clone(), provider);
        self.fallback_order.push(id);
    }

    /// Update routing config.
    pub fn set_config(&mut self, config: TtsManagerConfig) {
        self.config = config;
    }

    /// List all voices across all registered providers.
    pub async fn list_all_voices(&self) -> Vec<VoiceInfo> {
        let mut voices = Vec::new();
        for provider in self.providers.values() {
            if let Ok(mut v) = provider.list_voices().await {
                voices.append(&mut v);
            }
        }
        voices
    }

    /// Speak text using the best available provider.
    pub async fn speak(&self, request: &TtsRequest) -> Result<TtsOutput> {
        let ordered = self.ordered_providers();

        let mut last_err = None;
        for pid in &ordered {
            if let Some(provider) = self.providers.get(pid) {
                if !provider.is_available() {
                    continue;
                }
                match provider.speak(request).await {
                    Ok(output) => return Ok(output),
                    Err(e) => {
                        log::warn!("TTS provider {} failed: {e}", provider.name());
                        last_err = Some(e);
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("No TTS providers available")))
    }

    /// Return provider IDs in order defined by current strategy.
    fn ordered_providers(&self) -> Vec<TtsProviderId> {
        match self.config.strategy {
            TtsRouteStrategy::Fixed => {
                if let Some(ref pid) = self.config.preferred_provider {
                    vec![pid.clone()]
                } else {
                    self.fallback_order.clone()
                }
            }
            TtsRouteStrategy::LocalFirst => {
                let mut local = Vec::new();
                let mut cloud = Vec::new();
                for pid in &self.fallback_order {
                    if let Some(p) = self.providers.get(pid) {
                        if p.location() == TtsProviderLocation::Local {
                            local.push(pid.clone());
                        } else {
                            cloud.push(pid.clone());
                        }
                    }
                }
                local.extend(cloud);
                local
            }
            TtsRouteStrategy::CloudFirst => {
                let mut local = Vec::new();
                let mut cloud = Vec::new();
                for pid in &self.fallback_order {
                    if let Some(p) = self.providers.get(pid) {
                        if p.location() == TtsProviderLocation::Cloud {
                            cloud.push(pid.clone());
                        } else {
                            local.push(pid.clone());
                        }
                    }
                }
                cloud.extend(local);
                cloud
            }
            TtsRouteStrategy::CostOptimal => {
                // Sort by cost per character ascending
                let mut with_cost: Vec<_> = self
                    .fallback_order
                    .iter()
                    .filter_map(|pid| {
                        self.providers.get(pid).map(|p| {
                            let cost = p
                                .cost_per_character()
                                .map_or(u64::MAX, |c| c.as_microdollars());
                            (pid.clone(), cost)
                        })
                    })
                    .collect();
                with_cost.sort_by_key(|(_, c)| *c);
                with_cost.into_iter().map(|(pid, _)| pid).collect()
            }
        }
    }
}
