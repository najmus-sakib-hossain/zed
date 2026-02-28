//! DX-Py Types - Speculative Type Prediction and Inline Caches
//!
//! This crate implements:
//! - Monomorphic inline caches for single-type call sites
//! - Polymorphic inline caches (PIC) for 2-4 types
//! - Type predictor with statistics
//! - Deoptimization handler

pub mod deopt;
pub mod inline_cache;
pub mod pic;
pub mod predictor;

pub use deopt::DeoptHandler;
pub use inline_cache::{CacheState, InlineCache};
pub use pic::PolymorphicInlineCache;
pub use predictor::TypePredictor;
