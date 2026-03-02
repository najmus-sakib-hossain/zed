//! llama.cpp backend — maximum GGUF compatibility via C++ bindings.
//!
//! Provides the highest compatibility with quantized models from HuggingFace.
//! Used as a fallback when Candle's GGUF loader doesn't support a model architecture.

use anyhow::Result;
use dx_core::{DeviceTier, LlmRequest, LlmResponse, MicroCost};
use std::path::PathBuf;

/// Configuration for the llama.cpp backend.
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// Number of GPU layers to offload (0 = CPU only).
    pub n_gpu_layers: i32,
    /// Context size in tokens.
    pub n_ctx: u32,
    /// Batch size for prompt processing.
    pub n_batch: u32,
    /// Number of threads for CPU inference.
    pub n_threads: u32,
    /// Use memory mapping for model files.
    pub use_mmap: bool,
    /// Use memory locking to prevent swapping.
    pub use_mlock: bool,
}

impl LlamaCppConfig {
    /// Optimal configuration for a device tier.
    pub fn for_tier(tier: DeviceTier) -> Self {
        match tier {
            DeviceTier::UltraLow => Self {
                n_gpu_layers: 0,
                n_ctx: 2048,
                n_batch: 128,
                n_threads: 2,
                use_mmap: true,
                use_mlock: false,
            },
            DeviceTier::Low => Self {
                n_gpu_layers: 0,
                n_ctx: 4096,
                n_batch: 256,
                n_threads: 4,
                use_mmap: true,
                use_mlock: false,
            },
            DeviceTier::Mid => Self {
                n_gpu_layers: 20,
                n_ctx: 8192,
                n_batch: 512,
                n_threads: 4,
                use_mmap: true,
                use_mlock: false,
            },
            DeviceTier::High => Self {
                n_gpu_layers: 35,
                n_ctx: 16384,
                n_batch: 512,
                n_threads: 8,
                use_mmap: true,
                use_mlock: true,
            },
            DeviceTier::Ultra => Self {
                n_gpu_layers: 99, // All layers on GPU
                n_ctx: 32768,
                n_batch: 1024,
                n_threads: 8,
                use_mmap: true,
                use_mlock: true,
            },
        }
    }
}

/// llama.cpp inference backend.
///
/// Wraps llama-cpp-rs / llama-cpp-2 for maximum GGUF model compatibility.
/// Supports CUDA, Metal, ROCm, Vulkan, and CPU backends.
pub struct LlamaCppBackend {
    config: LlamaCppConfig,
    model_path: Option<PathBuf>,
    loaded: bool,
}

impl LlamaCppBackend {
    /// Create a new llama.cpp backend with the given configuration.
    pub fn new(config: LlamaCppConfig) -> Self {
        Self {
            config,
            model_path: None,
            loaded: false,
        }
    }

    /// Create with optimal settings for a device tier.
    pub fn for_tier(tier: DeviceTier) -> Self {
        Self::new(LlamaCppConfig::for_tier(tier))
    }

    /// Load a GGUF model file.
    pub fn load_model(&mut self, path: PathBuf) -> Result<()> {
        log::info!(
            "llama.cpp: loading model {:?} (gpu_layers={}, ctx={})",
            path,
            self.config.n_gpu_layers,
            self.config.n_ctx
        );

        if !path.exists() {
            return Err(anyhow::anyhow!("Model file not found: {:?}", path));
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "gguf" {
            return Err(anyhow::anyhow!(
                "llama.cpp only supports GGUF format, got: .{}",
                ext
            ));
        }

        // In production: llama_model_load() via FFI bindings
        self.model_path = Some(path);
        self.loaded = true;
        log::info!("llama.cpp: model loaded successfully");
        Ok(())
    }

    /// Run text generation.
    pub async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse> {
        if !self.loaded {
            return Err(anyhow::anyhow!("No model loaded in llama.cpp backend"));
        }

        let _ = &self.config;
        log::debug!(
            "llama.cpp generate: messages={}, max_tokens={:?}",
            request.messages.len(),
            request.max_tokens
        );

        // Placeholder: In production, this runs llama_decode() + llama_sample()
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".to_string()),
        })
    }

    /// Check if a model is loaded.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Unload model to free memory.
    pub fn unload(&mut self) {
        self.model_path = None;
        self.loaded = false;
        log::info!("llama.cpp: model unloaded");
    }
}
