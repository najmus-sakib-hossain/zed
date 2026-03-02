//! Inference engine — runs GGUF models locally using Candle or llama.cpp backends.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::gguf::{GgufModelInfo, GgufQuantization};

/// The backend engine used for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferenceBackend {
    /// HuggingFace Candle — pure-Rust, CUDA/Metal support.
    Candle,
    /// llama.cpp via FFI — broad hardware support, GGUF native.
    LlamaCpp,
}

impl InferenceBackend {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Candle => "Candle (Rust)",
            Self::LlamaCpp => "llama.cpp (C++)",
        }
    }
}

/// Configuration for inference execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub backend: InferenceBackend,
    pub n_threads: usize,
    pub n_gpu_layers: i32,
    pub context_length: usize,
    pub batch_size: usize,
    pub use_mmap: bool,
    pub use_mlock: bool,
    pub seed: Option<u64>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        let n_threads = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        Self {
            backend: InferenceBackend::LlamaCpp,
            n_threads,
            n_gpu_layers: -1, // auto-detect
            context_length: 4096,
            batch_size: 512,
            use_mmap: true,
            use_mlock: false,
            seed: None,
        }
    }
}

/// A request to the local inference engine.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub repetition_penalty: f32,
    pub stop_sequences: Vec<String>,
}

impl Default for InferenceRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            system_prompt: None,
            max_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repetition_penalty: 1.1,
            stop_sequences: Vec::new(),
        }
    }
}

/// Result from running local inference.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub text: String,
    pub tokens_generated: usize,
    pub tokens_per_second: f64,
    pub prompt_tokens: usize,
    pub total_duration_ms: u64,
}

/// A streaming token from the inference engine.
#[derive(Debug, Clone)]
pub struct StreamToken {
    pub text: String,
    pub is_final: bool,
    pub tokens_generated: usize,
}

/// Manages loading and running local models.
pub struct InferenceEngine {
    config: InferenceConfig,
    loaded_model: Option<LoadedModel>,
}

struct LoadedModel {
    info: GgufModelInfo,
    path: PathBuf,
    _quantization: GgufQuantization,
}

impl InferenceEngine {
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            config,
            loaded_model: None,
        }
    }

    /// Load a model from a GGUF file.
    pub fn load_model(&mut self, path: PathBuf, info: GgufModelInfo) -> Result<()> {
        let quantization = info.quantization;

        // Estimate memory requirements
        let estimated_bytes = quantization.estimated_size_bytes(info.parameters);
        let estimated_gb = estimated_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        log::info!(
            "Loading model {} ({}, {:.1} GB estimated) from {:?}",
            info.name,
            quantization.label(),
            estimated_gb,
            path
        );

        // In a real implementation, this would call into Candle or llama.cpp
        // to actually load the model weights. For now, we track the model metadata.
        match self.config.backend {
            InferenceBackend::Candle => {
                log::info!("Using Candle backend with {} threads", self.config.n_threads);
                // candle_core::Device, candle_transformers, etc.
            }
            InferenceBackend::LlamaCpp => {
                log::info!(
                    "Using llama.cpp backend with {} threads, {} GPU layers",
                    self.config.n_threads,
                    self.config.n_gpu_layers
                );
                // llama_cpp_2::LlamaModel::load_from_file()
            }
        }

        self.loaded_model = Some(LoadedModel {
            info,
            path,
            _quantization: quantization,
        });

        Ok(())
    }

    /// Unload the currently loaded model.
    pub fn unload_model(&mut self) {
        if let Some(model) = self.loaded_model.take() {
            log::info!("Unloaded model: {}", model.info.name);
        }
    }

    /// Check if a model is currently loaded.
    pub fn is_loaded(&self) -> bool {
        self.loaded_model.is_some()
    }

    /// Get information about the loaded model.
    pub fn loaded_model_info(&self) -> Option<&GgufModelInfo> {
        self.loaded_model.as_ref().map(|m| &m.info)
    }

    /// Run inference (non-streaming) on the loaded model.
    pub async fn complete(&self, request: &InferenceRequest) -> Result<InferenceResult> {
        let model = self
            .loaded_model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;

        let start = std::time::Instant::now();

        // Build the full prompt with system prompt if provided
        let full_prompt = match &request.system_prompt {
            Some(sys) => format!("<|system|>\n{}\n<|user|>\n{}\n<|assistant|>\n", sys, request.prompt),
            None => format!("<|user|>\n{}\n<|assistant|>\n", request.prompt),
        };

        log::debug!(
            "Running inference on {} with {} max tokens, temp={}",
            model.info.name,
            request.max_tokens,
            request.temperature
        );

        // Stub — in production this would call into the actual backend
        let _full_prompt = full_prompt;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        Ok(InferenceResult {
            text: String::new(),
            tokens_generated: 0,
            tokens_per_second: 0.0,
            prompt_tokens: 0,
            total_duration_ms: elapsed_ms,
        })
    }

    /// Run inference with streaming token output.
    pub async fn stream(
        &self,
        request: &InferenceRequest,
        token_callback: impl Fn(StreamToken) + Send + 'static,
    ) -> Result<InferenceResult> {
        let model = self
            .loaded_model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;

        let start = std::time::Instant::now();

        log::debug!(
            "Streaming inference on {} with {} max tokens",
            model.info.name,
            request.max_tokens
        );

        // Stub — in production, each generated token would be sent via callback
        token_callback(StreamToken {
            text: String::new(),
            is_final: true,
            tokens_generated: 0,
        });

        let elapsed_ms = start.elapsed().as_millis() as u64;

        Ok(InferenceResult {
            text: String::new(),
            tokens_generated: 0,
            tokens_per_second: 0.0,
            prompt_tokens: 0,
            total_duration_ms: elapsed_ms,
        })
    }

    /// Estimate GPU layers to offload based on available VRAM.
    pub fn auto_gpu_layers(model_size_bytes: u64, available_vram_bytes: u64) -> i32 {
        if available_vram_bytes == 0 {
            return 0; // No GPU
        }

        // Each layer is roughly uniform in size
        // Typical models have 24-80 layers
        // Reserve 512MB for KV cache and runtime overhead
        let usable_vram = available_vram_bytes.saturating_sub(512 * 1024 * 1024);
        let fraction = usable_vram as f64 / model_size_bytes as f64;

        // Assume 32 layers as default, scale proportionally
        let layers = (fraction * 32.0).min(999.0) as i32;
        layers.max(0)
    }

    /// Get the current configuration.
    pub fn config(&self) -> &InferenceConfig {
        &self.config
    }

    /// Update configuration (must unload and reload model for changes to take effect).
    pub fn set_config(&mut self, config: InferenceConfig) {
        self.config = config;
    }
}

/// Helper to detect available GPU memory for auto-configuration.
pub fn detect_gpu_vram() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    // Windows: Check NVIDIA via nvidia-smi
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,memory.total,memory.free", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(", ").collect();
                    if parts.len() >= 3 {
                        gpus.push(GpuInfo {
                            name: parts[0].trim().to_string(),
                            total_vram_mb: parts[1].trim().parse().unwrap_or(0),
                            free_vram_mb: parts[2].trim().parse().unwrap_or(0),
                            backend: GpuBackend::Cuda,
                        });
                    }
                }
            }
        }
    }

    // macOS: Apple Silicon unified memory
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Ok(total_bytes) = text.trim().parse::<u64>() {
                    let total_mb = total_bytes / (1024 * 1024);
                    // Apple Silicon shares system memory for GPU
                    // Typically 60-75% available for GPU use
                    let gpu_mb = total_mb * 70 / 100;
                    gpus.push(GpuInfo {
                        name: "Apple Silicon (Unified Memory)".into(),
                        total_vram_mb: gpu_mb,
                        free_vram_mb: gpu_mb / 2, // conservative estimate
                        backend: GpuBackend::Metal,
                    });
                }
            }
        }
    }

    // Linux: Check NVIDIA or AMD
    #[cfg(target_os = "linux")]
    {
        // Try NVIDIA first
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,memory.total,memory.free", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(", ").collect();
                    if parts.len() >= 3 {
                        gpus.push(GpuInfo {
                            name: parts[0].trim().to_string(),
                            total_vram_mb: parts[1].trim().parse().unwrap_or(0),
                            free_vram_mb: parts[2].trim().parse().unwrap_or(0),
                            backend: GpuBackend::Cuda,
                        });
                    }
                }
            }
        }

        // Try ROCm for AMD
        if let Ok(output) = std::process::Command::new("rocm-smi")
            .args(["--showmeminfo", "vram", "--csv"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines().skip(1) {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        gpus.push(GpuInfo {
                            name: format!("AMD GPU {}", parts[0].trim()),
                            total_vram_mb: parts[1].trim().parse::<u64>().unwrap_or(0) / (1024 * 1024),
                            free_vram_mb: parts[2].trim().parse::<u64>().unwrap_or(0) / (1024 * 1024),
                            backend: GpuBackend::Rocm,
                        });
                    }
                }
            }
        }
    }

    gpus
}

/// Information about a detected GPU.
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub total_vram_mb: u64,
    pub free_vram_mb: u64,
    pub backend: GpuBackend,
}

/// GPU compute backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    Cuda,
    Metal,
    Rocm,
    Vulkan,
    Cpu,
}
