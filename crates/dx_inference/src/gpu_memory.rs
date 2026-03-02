//! GPU memory manager — shares GPU resources across concurrent model loads.
//!
//! DX runs multiple models concurrently (grammar, prediction, voice, image gen).
//! This module tracks GPU memory usage and makes swap decisions.

use dx_core::DeviceTier;
use std::collections::HashMap;

/// A model slot occupying GPU memory.
#[derive(Debug, Clone)]
pub struct GpuSlot {
    /// Model ID.
    pub model_id: String,
    /// Feature using this slot.
    pub feature: GpuFeature,
    /// Estimated GPU memory usage in bytes.
    pub memory_bytes: u64,
    /// Whether this model is actively being used.
    pub active: bool,
    /// Priority (higher = harder to evict).
    pub priority: u32,
}

/// Feature categories that use GPU memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuFeature {
    /// LLM text generation.
    TextGeneration,
    /// Grammar checking (Tier 3 LLM).
    Grammar,
    /// Edit prediction / code completion.
    EditPrediction,
    /// Speech-to-text (Whisper).
    SpeechToText,
    /// Text-to-speech (Chatterbox/Piper).
    TextToSpeech,
    /// Image generation (SDXL/Flux).
    ImageGeneration,
    /// 3D model generation (TripoSR).
    ThreeDGeneration,
}

/// GPU memory manager — tracks and orchestrates model loading across features.
pub struct GpuMemoryManager {
    /// Available GPU memory in bytes.
    total_vram: u64,
    /// Currently loaded model slots.
    slots: HashMap<String, GpuSlot>,
    /// Device tier (affects eviction strategy).
    tier: DeviceTier,
}

impl GpuMemoryManager {
    /// Create a new GPU memory manager.
    pub fn new(total_vram: u64, tier: DeviceTier) -> Self {
        Self {
            total_vram,
            slots: HashMap::new(),
            tier,
        }
    }

    /// Total GPU memory in bytes.
    pub fn total_vram(&self) -> u64 {
        self.total_vram
    }

    /// Currently used GPU memory in bytes.
    pub fn used_vram(&self) -> u64 {
        self.slots.values().map(|s| s.memory_bytes).sum()
    }

    /// Available GPU memory in bytes.
    pub fn available_vram(&self) -> u64 {
        self.total_vram.saturating_sub(self.used_vram())
    }

    /// Check if a model can be loaded without evicting anything.
    pub fn can_fit(&self, memory_bytes: u64) -> bool {
        self.available_vram() >= memory_bytes
    }

    /// Reserve a GPU slot for a model.
    pub fn reserve(&mut self, slot: GpuSlot) -> bool {
        if !self.can_fit(slot.memory_bytes) {
            return false;
        }
        self.slots.insert(slot.model_id.clone(), slot);
        true
    }

    /// Release a GPU slot.
    pub fn release(&mut self, model_id: &str) -> Option<GpuSlot> {
        self.slots.remove(model_id)
    }

    /// Find the best candidate for eviction to free `needed_bytes`.
    pub fn eviction_candidate(&self, needed_bytes: u64) -> Option<String> {
        if self.available_vram() >= needed_bytes {
            return None; // No eviction needed
        }

        // Evict the lowest-priority inactive model
        self.slots
            .values()
            .filter(|s| !s.active)
            .min_by_key(|s| s.priority)
            .map(|s| s.model_id.clone())
    }

    /// Evict models until `needed_bytes` is available.
    pub fn evict_for(&mut self, needed_bytes: u64) -> Vec<String> {
        let mut evicted = Vec::new();
        while self.available_vram() < needed_bytes {
            if let Some(model_id) = self.eviction_candidate(needed_bytes) {
                self.slots.remove(&model_id);
                evicted.push(model_id);
            } else {
                break; // Can't evict more — all active
            }
        }
        evicted
    }

    /// Get memory pressure level (0.0 = empty, 1.0 = full).
    pub fn pressure(&self) -> f64 {
        if self.total_vram == 0 {
            return 1.0;
        }
        self.used_vram() as f64 / self.total_vram as f64
    }

    /// Should we downgrade model quality due to memory pressure?
    pub fn should_downgrade(&self) -> bool {
        let _ = self.tier;
        self.pressure() > 0.9
    }

    /// Get all currently loaded slots.
    pub fn slots(&self) -> impl Iterator<Item = &GpuSlot> {
        self.slots.values()
    }
}
