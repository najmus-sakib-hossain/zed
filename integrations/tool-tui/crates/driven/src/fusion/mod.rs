//! DX Fusion Mode - Pre-Compiled Template System
//!
//! Inspired by dx-js-bundler's fusion mode for 0.7ms template loading.

mod binary_cache;
mod hot_cache;
mod speculative_loader;
mod template_module;

pub use binary_cache::{BinaryCache, CacheEntry, CacheKey};
pub use hot_cache::{HotCache, TemplateCacheEntry};
pub use speculative_loader::{PredictionEngine, SpeculativeLoader};
pub use template_module::{FusionHeader, FusionModule, TemplateSlot};

/// Fusion format magic bytes
pub const FUSION_MAGIC: &[u8; 4] = b"DRVF";

/// Fusion format version
pub const FUSION_VERSION: u16 = 1;
