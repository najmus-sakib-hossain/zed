//! Model cache — manages downloaded model files on disk.
//!
//! Models are stored in `~/.dx/models/` with metadata tracking download state,
//! SHA256 verification, and disk usage.

use anyhow::Result;
use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Format of a model file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    /// GGUF quantized format (llama.cpp / Candle).
    Gguf,
    /// SafeTensors format (Candle / HuggingFace).
    SafeTensors,
    /// ONNX Runtime format (Piper TTS, Whisper).
    Onnx,
    /// PyTorch checkpoint (requires conversion).
    Pytorch,
}

impl ModelFormat {
    /// File extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ModelFormat::Gguf => "gguf",
            ModelFormat::SafeTensors => "safetensors",
            ModelFormat::Onnx => "onnx",
            ModelFormat::Pytorch => "bin",
        }
    }
}

/// A model purpose/category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelPurpose {
    /// Text generation (LLM).
    TextGeneration,
    /// Speech to text (Whisper).
    SpeechToText,
    /// Text to speech (Piper, Chatterbox, Kokoro).
    TextToSpeech,
    /// Image generation (Stable Diffusion, Flux).
    ImageGeneration,
    /// Embedding model.
    Embedding,
    /// Grammar/writing improvement.
    Grammar,
}

/// A cached model entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModel {
    /// Unique model identifier (e.g., "smollm2-360m-q4_k_m").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Model purpose.
    pub purpose: ModelPurpose,
    /// File format.
    pub format: ModelFormat,
    /// HuggingFace repo ID (e.g., "HuggingFaceTB/SmolLM2-360M-Instruct-GGUF").
    pub hf_repo: String,
    /// Filename within the repo.
    pub hf_filename: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// SHA256 hash for verification.
    pub sha256: Option<String>,
    /// Local file path (when downloaded).
    pub local_path: Option<PathBuf>,
    /// Minimum device tier required to run this model.
    pub min_tier: DeviceTier,
    /// Whether this model is currently downloaded and verified.
    pub downloaded: bool,
}

/// Model cache manager — tracks available and downloadable models.
pub struct ModelCache {
    tier: DeviceTier,
    cache_dir: PathBuf,
    models: Vec<CachedModel>,
}

impl ModelCache {
    /// Create a new model cache for the given device tier.
    pub fn new(tier: DeviceTier) -> Self {
        let cache_dir = dirs_path().join("models");

        let models = default_model_catalog(tier);

        Self {
            tier,
            cache_dir,
            models,
        }
    }

    /// Number of models currently available (downloaded + verified).
    pub fn available_count(&self) -> usize {
        self.models.iter().filter(|m| m.downloaded).count()
    }

    /// Number of models pending download.
    pub fn pending_count(&self) -> usize {
        self.models.iter().filter(|m| !m.downloaded && m.min_tier <= self.tier).count()
    }

    /// Get all models suitable for the current tier.
    pub fn models_for_tier(&self) -> Vec<&CachedModel> {
        self.models.iter().filter(|m| m.min_tier <= self.tier).collect()
    }

    /// Get a specific model by ID.
    pub fn get_model(&self, id: &str) -> Option<&CachedModel> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Total disk space used by downloaded models in bytes.
    pub fn disk_usage(&self) -> u64 {
        self.models
            .iter()
            .filter(|m| m.downloaded)
            .map(|m| m.size_bytes)
            .sum()
    }

    /// Check if a model file exists on disk and update its status.
    pub fn verify_model(&mut self, id: &str) -> Result<bool> {
        if let Some(model) = self.models.iter_mut().find(|m| m.id == id) {
            if let Some(ref path) = model.local_path {
                if path.exists() {
                    model.downloaded = true;
                    return Ok(true);
                }
            }
            model.downloaded = false;
        }
        Ok(false)
    }
}

/// Get the DX home directory (`~/.dx/`).
fn dirs_path() -> PathBuf {
    if let Ok(dx_home) = std::env::var("DX_HOME") {
        return PathBuf::from(dx_home);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dx")
}

/// Default model catalog organized by download priority.
fn default_model_catalog(tier: DeviceTier) -> Vec<CachedModel> {
    let mut models = Vec::new();

    // Priority 1: Whisper Tiny (STT) — needed for voice input
    models.push(CachedModel {
        id: "whisper-tiny-en".to_string(),
        name: "Whisper Tiny English".to_string(),
        purpose: ModelPurpose::SpeechToText,
        format: ModelFormat::Gguf,
        hf_repo: "ggerganov/whisper.cpp".to_string(),
        hf_filename: "ggml-tiny.en.bin".to_string(),
        size_bytes: 75_000_000,
        sha256: None,
        local_path: None,
        min_tier: DeviceTier::UltraLow,
        downloaded: false,
    });

    // Priority 2: Piper TTS tiny — voice output
    models.push(CachedModel {
        id: "piper-lessac-low".to_string(),
        name: "Piper Lessac Low Quality".to_string(),
        purpose: ModelPurpose::TextToSpeech,
        format: ModelFormat::Onnx,
        hf_repo: "rhasspy/piper-voices".to_string(),
        hf_filename: "en_US-lessac-low.onnx".to_string(),
        size_bytes: 15_000_000,
        sha256: None,
        local_path: None,
        min_tier: DeviceTier::UltraLow,
        downloaded: false,
    });

    // Priority 3: SmolLM2 360M — ultra-light LLM for grammar + basic completion
    models.push(CachedModel {
        id: "smollm2-360m-q4".to_string(),
        name: "SmolLM2 360M Q4_K_M".to_string(),
        purpose: ModelPurpose::TextGeneration,
        format: ModelFormat::Gguf,
        hf_repo: "HuggingFaceTB/SmolLM2-360M-Instruct-GGUF".to_string(),
        hf_filename: "smollm2-360m-instruct-q4_k_m.gguf".to_string(),
        size_bytes: 200_000_000,
        sha256: None,
        local_path: None,
        min_tier: DeviceTier::UltraLow,
        downloaded: false,
    });

    // Tier 2+ models
    if tier >= DeviceTier::Low {
        models.push(CachedModel {
            id: "whisper-base-en".to_string(),
            name: "Whisper Base English".to_string(),
            purpose: ModelPurpose::SpeechToText,
            format: ModelFormat::Gguf,
            hf_repo: "ggerganov/whisper.cpp".to_string(),
            hf_filename: "ggml-base.en.bin".to_string(),
            size_bytes: 142_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::Low,
            downloaded: false,
        });

        models.push(CachedModel {
            id: "piper-lessac-medium".to_string(),
            name: "Piper Lessac Medium Quality".to_string(),
            purpose: ModelPurpose::TextToSpeech,
            format: ModelFormat::Onnx,
            hf_repo: "rhasspy/piper-voices".to_string(),
            hf_filename: "en_US-lessac-medium.onnx".to_string(),
            size_bytes: 65_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::Low,
            downloaded: false,
        });
    }

    // Tier 3+ models
    if tier >= DeviceTier::Mid {
        models.push(CachedModel {
            id: "qwen3-1.7b-q4".to_string(),
            name: "Qwen3 1.7B Q4_K_M".to_string(),
            purpose: ModelPurpose::TextGeneration,
            format: ModelFormat::Gguf,
            hf_repo: "Qwen/Qwen3-1.7B-GGUF".to_string(),
            hf_filename: "qwen3-1.7b-q4_k_m.gguf".to_string(),
            size_bytes: 1_000_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::Mid,
            downloaded: false,
        });

        models.push(CachedModel {
            id: "whisper-small-en".to_string(),
            name: "Whisper Small English".to_string(),
            purpose: ModelPurpose::SpeechToText,
            format: ModelFormat::Gguf,
            hf_repo: "ggerganov/whisper.cpp".to_string(),
            hf_filename: "ggml-small.en.bin".to_string(),
            size_bytes: 244_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::Mid,
            downloaded: false,
        });
    }

    // Tier 4+ models
    if tier >= DeviceTier::High {
        models.push(CachedModel {
            id: "llama3.1-8b-q5".to_string(),
            name: "Llama 3.1 8B Q5_K_M".to_string(),
            purpose: ModelPurpose::TextGeneration,
            format: ModelFormat::Gguf,
            hf_repo: "bartowski/Meta-Llama-3.1-8B-Instruct-GGUF".to_string(),
            hf_filename: "Meta-Llama-3.1-8B-Instruct-Q5_K_M.gguf".to_string(),
            size_bytes: 5_700_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::High,
            downloaded: false,
        });

        models.push(CachedModel {
            id: "chatterbox-turbo".to_string(),
            name: "Chatterbox Turbo TTS".to_string(),
            purpose: ModelPurpose::TextToSpeech,
            format: ModelFormat::Onnx,
            hf_repo: "ResembleAI/chatterbox".to_string(),
            hf_filename: "chatterbox-turbo.onnx".to_string(),
            size_bytes: 500_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::High,
            downloaded: false,
        });
    }

    // Tier 5 (Ultra) models
    if tier >= DeviceTier::Ultra {
        models.push(CachedModel {
            id: "whisper-large-v3".to_string(),
            name: "Whisper Large V3".to_string(),
            purpose: ModelPurpose::SpeechToText,
            format: ModelFormat::Gguf,
            hf_repo: "ggerganov/whisper.cpp".to_string(),
            hf_filename: "ggml-large-v3.bin".to_string(),
            size_bytes: 1_500_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::Ultra,
            downloaded: false,
        });

        models.push(CachedModel {
            id: "llama3.1-70b-q4".to_string(),
            name: "Llama 3.1 70B Q4_K_M".to_string(),
            purpose: ModelPurpose::TextGeneration,
            format: ModelFormat::Gguf,
            hf_repo: "bartowski/Meta-Llama-3.1-70B-Instruct-GGUF".to_string(),
            hf_filename: "Meta-Llama-3.1-70B-Instruct-Q4_K_M.gguf".to_string(),
            size_bytes: 40_000_000_000,
            sha256: None,
            local_path: None,
            min_tier: DeviceTier::Ultra,
            downloaded: false,
        });
    }

    models
}

/// Helper: `dirs` crate home_dir equivalent (inline to avoid external dep).
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(windows))]
        {
            std::env::var("HOME").ok().map(PathBuf::from)
        }
    }
}
