//! Provider registry — central registration and lookup for all providers.
//!
//! Manages both Universe A (LLM) and Universe B (Media) providers in a
//! single registry with health monitoring and auto-failover.

use crate::llm_provider::{LlmProvider, LlmProviderId};
use crate::media_provider::{MediaProvider, MediaProviderId, MediaType};
use crate::tts_provider::{TtsProvider, TtsProviderId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Central registry for all DX providers.
pub struct DxProviderRegistry {
    llm_providers: RwLock<HashMap<LlmProviderId, Arc<dyn LlmProvider>>>,
    media_providers: RwLock<HashMap<MediaProviderId, Arc<dyn MediaProvider>>>,
    tts_providers: RwLock<HashMap<TtsProviderId, Arc<dyn TtsProvider>>>,
}

impl DxProviderRegistry {
    pub fn new() -> Self {
        Self {
            llm_providers: RwLock::new(HashMap::new()),
            media_providers: RwLock::new(HashMap::new()),
            tts_providers: RwLock::new(HashMap::new()),
        }
    }

    // ---- Universe A: LLM Providers ----

    pub fn register_llm_provider(&self, provider: Arc<dyn LlmProvider>) {
        let id = provider.id().clone();
        self.llm_providers.write().insert(id, provider);
    }

    pub fn get_llm_provider(&self, id: &LlmProviderId) -> Option<Arc<dyn LlmProvider>> {
        self.llm_providers.read().get(id).cloned()
    }

    pub fn list_llm_providers(&self) -> Vec<Arc<dyn LlmProvider>> {
        self.llm_providers.read().values().cloned().collect()
    }

    pub fn available_llm_providers(&self) -> Vec<Arc<dyn LlmProvider>> {
        self.llm_providers
            .read()
            .values()
            .filter(|p| p.is_available())
            .cloned()
            .collect()
    }

    // ---- Universe B: Media Providers ----

    pub fn register_media_provider(&self, provider: Arc<dyn MediaProvider>) {
        let id = provider.id().clone();
        self.media_providers.write().insert(id, provider);
    }

    pub fn get_media_provider(&self, id: &MediaProviderId) -> Option<Arc<dyn MediaProvider>> {
        self.media_providers.read().get(id).cloned()
    }

    pub fn list_media_providers(&self) -> Vec<Arc<dyn MediaProvider>> {
        self.media_providers.read().values().cloned().collect()
    }

    pub fn media_providers_for_type(&self, media_type: MediaType) -> Vec<Arc<dyn MediaProvider>> {
        self.media_providers
            .read()
            .values()
            .filter(|p| p.supported_media_types().contains(&media_type) && p.is_available())
            .cloned()
            .collect()
    }

    // ---- TTS Providers ----

    pub fn register_tts_provider(&self, provider: Arc<dyn TtsProvider>) {
        let id = provider.id().clone();
        self.tts_providers.write().insert(id, provider);
    }

    pub fn get_tts_provider(&self, id: &TtsProviderId) -> Option<Arc<dyn TtsProvider>> {
        self.tts_providers.read().get(id).cloned()
    }

    pub fn list_tts_providers(&self) -> Vec<Arc<dyn TtsProvider>> {
        self.tts_providers.read().values().cloned().collect()
    }

    pub fn available_tts_providers(&self) -> Vec<Arc<dyn TtsProvider>> {
        self.tts_providers
            .read()
            .values()
            .filter(|p| p.is_available())
            .cloned()
            .collect()
    }
}

impl Default for DxProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
