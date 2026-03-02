//! DX Inference — Local ML inference engine for offline/free operation.
//!
//! This crate provides embedded inference via Candle and llama.cpp bindings,
//! enabling DX to run LLM, image generation, TTS, and STT models locally
//! without any cloud dependency.
//!
//! ## Architecture
//!
//! - **Candle backend:** Pure Rust ML framework with CUDA/Metal/CPU support
//! - **llama.cpp backend:** Maximum GGUF compatibility via C++ bindings
//! - **Model manager:** Downloads, verifies, caches, and swaps models
//! - **GPU memory manager:** Shares GPU across grammar + prediction + voice
//!
//! ## Progressive Download Strategy
//!
//! Models are downloaded in priority order based on device tier:
//! 1. Harper grammar (bundled, ~5MB) — instant
//! 2. Piper TTS tiny (~15MB) — 15 seconds
//! 3. Whisper Tiny (~75MB) — 45 seconds
//! 4. SmolLM2/Qwen3 (~200-400MB) — 90 seconds
//! 5. Full model suite — 180 seconds

mod candle_backend;
mod download_manager;
mod gpu_memory;
mod llama_backend;
mod model_cache;

pub use candle_backend::CandleBackend;
pub use download_manager::{DownloadManager, DownloadPriority, DownloadTask};
pub use gpu_memory::GpuMemoryManager;
pub use llama_backend::LlamaCppBackend;
pub use model_cache::{CachedModel, ModelCache, ModelFormat};

use dx_core::DeviceTier;

/// Initialize the local inference engine for the detected device tier.
///
/// This sets up the model cache directory, GPU memory manager, and
/// queues model downloads based on the device's capabilities.
pub fn init(tier: DeviceTier) {
    log::info!("DX Inference initializing for tier: {}", tier.display_name());

    let cache = ModelCache::new(tier);
    log::info!(
        "Model cache: {} models available, {} pending download",
        cache.available_count(),
        cache.pending_count()
    );
}
