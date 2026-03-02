//! DX Local Inference Engine — embedded ML inference for offline/free operation.
//!
//! This crate provides:
//! - Model cache management (download, verify, clean)
//! - GGUF model loading and inference pipeline
//! - Integration points for Candle and llama.cpp backends
//! - Progressive download strategy for first-launch experience
//! - Concurrent model loading with GPU memory sharing
//!
//! The actual Candle and llama.cpp crate dependencies are behind feature flags
//! since they have heavy native dependencies. The core architecture and model
//! management work without them.

pub mod cache;
pub mod download;
pub mod gguf;
pub mod inference;
pub mod progressive;

pub use cache::ModelCache;
pub use download::ModelDownloader;
pub use gguf::{GgufModelInfo, GgufQuantization};
pub use inference::{InferenceBackend, InferenceEngine, InferenceRequest, InferenceResult};
pub use progressive::ProgressiveLoader;
