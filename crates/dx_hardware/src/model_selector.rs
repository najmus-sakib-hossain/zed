//! Model selector — picks the best model for the device tier.

use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};

/// A model recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    /// Model identifier.
    pub model_id: String,
    /// Human-readable name.
    pub display_name: String,
    /// Parameter count for reference.
    pub parameters: Option<u64>,
    /// Where to run: local or cloud.
    pub location: ModelLocation,
    /// Why this model was chosen.
    pub reason: String,
}

/// Where a model runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelLocation {
    Local,
    Cloud,
}

/// Selects optimal models based on hardware capabilities.
pub struct ModelSelector {
    tier: DeviceTier,
}

impl ModelSelector {
    pub fn new(tier: DeviceTier) -> Self {
        Self { tier }
    }

    /// Recommend the best LLM for general use.
    pub fn recommend_llm(&self) -> ModelRecommendation {
        match self.tier {
            DeviceTier::Embedded => ModelRecommendation {
                model_id: "gpt-4o-mini".into(),
                display_name: "GPT-4o Mini (Cloud)".into(),
                parameters: None,
                location: ModelLocation::Cloud,
                reason: "Device too limited for local inference".into(),
            },
            DeviceTier::LowEnd => ModelRecommendation {
                model_id: "qwen2.5-1.5b-q4".into(),
                display_name: "Qwen 2.5 1.5B (Local)".into(),
                parameters: Some(1_500_000_000),
                location: ModelLocation::Local,
                reason: "Small model fits in limited RAM".into(),
            },
            DeviceTier::MidRange => ModelRecommendation {
                model_id: "llama-3.2-3b-q4".into(),
                display_name: "Llama 3.2 3B (Local)".into(),
                parameters: Some(3_000_000_000),
                location: ModelLocation::Local,
                reason: "Good balance of speed and quality".into(),
            },
            DeviceTier::HighEnd => ModelRecommendation {
                model_id: "llama-3.1-8b-q4".into(),
                display_name: "Llama 3.1 8B (Local)".into(),
                parameters: Some(8_000_000_000),
                location: ModelLocation::Local,
                reason: "High-quality local inference with GPU offload".into(),
            },
            DeviceTier::Workstation => ModelRecommendation {
                model_id: "qwen2.5-32b-q4".into(),
                display_name: "Qwen 2.5 32B (Local)".into(),
                parameters: Some(32_000_000_000),
                location: ModelLocation::Local,
                reason: "Large model with excellent quality, fits GPU".into(),
            },
        }
    }

    /// Recommend the best STT model.
    pub fn recommend_stt(&self) -> ModelRecommendation {
        match self.tier {
            DeviceTier::Embedded | DeviceTier::LowEnd => ModelRecommendation {
                model_id: "whisper-tiny".into(),
                display_name: "Whisper Tiny".into(),
                parameters: Some(39_000_000),
                location: ModelLocation::Local,
                reason: "Minimal resource usage".into(),
            },
            DeviceTier::MidRange => ModelRecommendation {
                model_id: "whisper-base".into(),
                display_name: "Whisper Base".into(),
                parameters: Some(74_000_000),
                location: ModelLocation::Local,
                reason: "Good accuracy in moderate RAM".into(),
            },
            DeviceTier::HighEnd | DeviceTier::Workstation => ModelRecommendation {
                model_id: "whisper-medium".into(),
                display_name: "Whisper Medium".into(),
                parameters: Some(769_000_000),
                location: ModelLocation::Local,
                reason: "Best accuracy for robust hardware".into(),
            },
        }
    }

    /// Recommend the best TTS model.
    pub fn recommend_tts(&self) -> ModelRecommendation {
        match self.tier {
            DeviceTier::Embedded => ModelRecommendation {
                model_id: "piper-tiny".into(),
                display_name: "Piper Tiny".into(),
                parameters: None,
                location: ModelLocation::Local,
                reason: "Minimal resource TTS".into(),
            },
            DeviceTier::LowEnd | DeviceTier::MidRange => ModelRecommendation {
                model_id: "piper-medium".into(),
                display_name: "Piper Medium".into(),
                parameters: None,
                location: ModelLocation::Local,
                reason: "Good quality local TTS".into(),
            },
            DeviceTier::HighEnd | DeviceTier::Workstation => ModelRecommendation {
                model_id: "chatterbox-turbo".into(),
                display_name: "Chatterbox Turbo".into(),
                parameters: None,
                location: ModelLocation::Local,
                reason: "Best local TTS quality with voice cloning".into(),
            },
        }
    }
}
