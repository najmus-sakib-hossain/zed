//! GGUF model format definitions and loading support.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// GGUF quantization levels — determines model size vs quality tradeoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GgufQuantization {
    /// 2-bit quantization — smallest, lowest quality
    Q2K,
    /// 3-bit quantization
    Q3KS,
    Q3KM,
    Q3KL,
    /// 4-bit quantization — best balance for most hardware
    Q4_0,
    Q4_1,
    Q4KS,
    Q4KM,
    /// 5-bit quantization — higher quality
    Q5_0,
    Q5_1,
    Q5KS,
    Q5KM,
    /// 6-bit quantization
    Q6K,
    /// 8-bit quantization — near-original quality
    Q8_0,
    /// 16-bit float — full precision
    F16,
    /// 32-bit float — original weights
    F32,
}

impl GgufQuantization {
    /// Approximate size multiplier relative to parameter count.
    /// Returns bytes per parameter.
    pub fn bytes_per_param(&self) -> f64 {
        match self {
            Self::Q2K => 0.3125,
            Self::Q3KS | Self::Q3KM | Self::Q3KL => 0.4375,
            Self::Q4_0 | Self::Q4_1 | Self::Q4KS | Self::Q4KM => 0.5625,
            Self::Q5_0 | Self::Q5_1 | Self::Q5KS | Self::Q5KM => 0.6875,
            Self::Q6K => 0.8125,
            Self::Q8_0 => 1.0,
            Self::F16 => 2.0,
            Self::F32 => 4.0,
        }
    }

    /// Estimated file size in bytes for a model with given parameter count.
    pub fn estimated_size_bytes(&self, params_billions: f64) -> u64 {
        (params_billions * 1e9 * self.bytes_per_param()) as u64
    }

    /// Human-readable quantization name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Q2K => "Q2_K",
            Self::Q3KS => "Q3_K_S",
            Self::Q3KM => "Q3_K_M",
            Self::Q3KL => "Q3_K_L",
            Self::Q4_0 => "Q4_0",
            Self::Q4_1 => "Q4_1",
            Self::Q4KS => "Q4_K_S",
            Self::Q4KM => "Q4_K_M",
            Self::Q5_0 => "Q5_0",
            Self::Q5_1 => "Q5_1",
            Self::Q5KS => "Q5_K_S",
            Self::Q5KM => "Q5_K_M",
            Self::Q6K => "Q6_K",
            Self::Q8_0 => "Q8_0",
            Self::F16 => "F16",
            Self::F32 => "F32",
        }
    }

    /// Recommended quantization for available VRAM.
    pub fn for_vram_gb(vram_gb: f64, params_billions: f64) -> Self {
        let budget = vram_gb * 1e9 * 0.85; // 85% of VRAM for model
        if params_billions * 1e9 * 0.3125 < budget {
            // Can fit Q2K, try higher
            if params_billions * 1e9 * 1.0 < budget {
                Self::Q8_0
            } else if params_billions * 1e9 * 0.6875 < budget {
                Self::Q5KM
            } else if params_billions * 1e9 * 0.5625 < budget {
                Self::Q4KM
            } else if params_billions * 1e9 * 0.4375 < budget {
                Self::Q3KM
            } else {
                Self::Q2K
            }
        } else {
            Self::Q2K // Absolute minimum
        }
    }
}

/// Information about a GGUF model file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufModelInfo {
    /// Model identifier (e.g., "qwen2.5-3b-instruct")
    pub model_id: String,
    /// Human-readable name
    pub name: String,
    /// Parameter count in billions
    pub params_billions: f64,
    /// Quantization level
    pub quantization: GgufQuantization,
    /// File size in bytes
    pub file_size: u64,
    /// Expected SHA256 hash for verification
    pub sha256: Option<String>,
    /// Hugging Face repo ID
    pub hf_repo: String,
    /// Filename within the repo
    pub hf_filename: String,
    /// Local file path (once downloaded)
    pub local_path: Option<PathBuf>,
    /// Context window size
    pub context_length: usize,
    /// Model purpose
    pub purpose: ModelPurpose,
}

/// What a model is designed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelPurpose {
    /// General chat/instruction following
    Chat,
    /// Code completion and generation
    Code,
    /// Grammar and prose improvement
    Prose,
    /// Speech-to-text transcription
    Stt,
    /// Text-to-speech synthesis
    Tts,
    /// Image generation (diffusion)
    ImageGen,
    /// Image understanding (vision)
    Vision,
    /// Text embeddings
    Embedding,
}

/// Well-known GGUF models organized by purpose and size.
pub fn recommended_models() -> Vec<GgufModelInfo> {
    vec![
        // Small chat models (Tier 1-2)
        GgufModelInfo {
            model_id: "smollm2-360m".into(),
            name: "SmolLM2 360M".into(),
            params_billions: 0.36,
            quantization: GgufQuantization::Q4KM,
            file_size: 256_000_000,
            sha256: None,
            hf_repo: "HuggingFaceTB/SmolLM2-360M-Instruct-GGUF".into(),
            hf_filename: "smollm2-360m-instruct-q4_k_m.gguf".into(),
            local_path: None,
            context_length: 2048,
            purpose: ModelPurpose::Chat,
        },
        // Medium chat models (Tier 3)
        GgufModelInfo {
            model_id: "qwen2.5-1.5b".into(),
            name: "Qwen 2.5 1.5B".into(),
            params_billions: 1.5,
            quantization: GgufQuantization::Q4KM,
            file_size: 1_000_000_000,
            sha256: None,
            hf_repo: "Qwen/Qwen2.5-1.5B-Instruct-GGUF".into(),
            hf_filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf".into(),
            local_path: None,
            context_length: 32768,
            purpose: ModelPurpose::Chat,
        },
        // Large chat models (Tier 4)
        GgufModelInfo {
            model_id: "qwen2.5-7b".into(),
            name: "Qwen 2.5 7B".into(),
            params_billions: 7.0,
            quantization: GgufQuantization::Q4KM,
            file_size: 4_500_000_000,
            sha256: None,
            hf_repo: "Qwen/Qwen2.5-7B-Instruct-GGUF".into(),
            hf_filename: "qwen2.5-7b-instruct-q4_k_m.gguf".into(),
            local_path: None,
            context_length: 131072,
            purpose: ModelPurpose::Chat,
        },
        // Code models
        GgufModelInfo {
            model_id: "qwen2.5-coder-3b".into(),
            name: "Qwen 2.5 Coder 3B".into(),
            params_billions: 3.0,
            quantization: GgufQuantization::Q4KM,
            file_size: 2_000_000_000,
            sha256: None,
            hf_repo: "Qwen/Qwen2.5-Coder-3B-Instruct-GGUF".into(),
            hf_filename: "qwen2.5-coder-3b-instruct-q4_k_m.gguf".into(),
            local_path: None,
            context_length: 32768,
            purpose: ModelPurpose::Code,
        },
        // Embedding model
        GgufModelInfo {
            model_id: "nomic-embed-text-v1.5".into(),
            name: "Nomic Embed Text v1.5".into(),
            params_billions: 0.137,
            quantization: GgufQuantization::F16,
            file_size: 274_000_000,
            sha256: None,
            hf_repo: "nomic-ai/nomic-embed-text-v1.5-GGUF".into(),
            hf_filename: "nomic-embed-text-v1.5.f16.gguf".into(),
            local_path: None,
            context_length: 8192,
            purpose: ModelPurpose::Embedding,
        },
    ]
}
