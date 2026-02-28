//! DX-Py SIMD - SIMD-Accelerated String Operations
//!
//! This crate provides SIMD-accelerated string operations using AVX2/AVX-512/NEON
//! instructions for 8-15x speedup over scalar implementations.
//!
//! ## Features
//!
//! - AVX-512 acceleration on x86_64 (64 bytes/iteration) - highest throughput
//! - AVX2 acceleration on x86_64 (32 bytes/iteration)
//! - NEON acceleration on ARM64 (16 bytes/iteration)
//! - Automatic CPU detection and dispatch
//! - Scalar fallback for compatibility
//!
//! ## Usage
//!
//! ```rust
//! use dx_py_simd::get_engine;
//!
//! let engine = get_engine();
//! let pos = engine.find("hello world", "world");
//! assert_eq!(pos, Some(6));
//! ```

pub mod avx2;
pub mod avx512;
pub mod dispatcher;
pub mod engine;
pub mod neon;
pub mod scalar;

pub use avx512::Avx512StringEngine;
pub use dispatcher::SimdDispatcher;
pub use engine::SimdStringEngine;
pub use neon::NeonStringEngine;

/// Get the best available SIMD engine for the current CPU
pub fn get_engine() -> Box<dyn SimdStringEngine> {
    SimdDispatcher::new().get_engine()
}
