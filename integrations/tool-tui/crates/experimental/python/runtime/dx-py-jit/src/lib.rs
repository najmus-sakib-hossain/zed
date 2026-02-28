//! DX-Py JIT - Tiered JIT Compiler with Cranelift Backend
//!
//! This crate implements a 4-tier JIT compilation strategy for the DX-Py runtime,
//! providing progressive optimization based on execution profiles.
//!
//! ## Compilation Tiers
//!
//! | Tier | Name | Threshold | Description |
//! |------|------|-----------|-------------|
//! | 0 | Interpreter | 0 | Bytecode interpretation with profiling |
//! | 1 | Baseline JIT | 100 calls | Fast compile, moderate speedup |
//! | 2 | Optimizing JIT | 1000 calls | Type-specialized with guards |
//! | 3 | AOT Optimized | 10000 calls | Profile-guided, persistent |
//!
//! ## Features
//!
//! - [`CompilationTier`]: Tier definitions and thresholds
//! - [`FunctionProfile`]: Execution profiling for tier promotion
//! - [`TypeFeedback`]: Type observation for specialization
//! - [`TieredJit`]: Main JIT compiler interface
//! - [`OsrManager`]: On-stack replacement for hot loops
//!
//! ## Usage
//!
//! ```rust
//! use dx_py_jit::{TieredJit, FunctionId, CompilationTier};
//!
//! let jit = TieredJit::new();
//! let func_id = FunctionId(1);
//!
//! // Get or create a profile
//! let profile = jit.get_profile(func_id, 100, 5);
//!
//! // Record calls
//! for _ in 0..100 {
//!     profile.record_call();
//! }
//!
//! // Check for tier promotion
//! if let Some(tier) = jit.check_promotion(func_id) {
//!     println!("Promote to {:?}", tier);
//! }
//! ```
//!
//! ## Type Feedback
//!
//! The JIT collects type information at each bytecode location:
//!
//! - **Monomorphic**: Single type observed - can emit specialized code
//! - **Polymorphic**: 2-4 types - emit type guards with fallback
//! - **Megamorphic**: Too many types - use generic code
//!
//! ## Deoptimization
//!
//! When type guards fail, the JIT deoptimizes back to the interpreter:
//!
//! 1. Save live values from registers/stack
//! 2. Reconstruct interpreter state
//! 3. Continue execution in interpreter
//! 4. Re-profile for better specialization

pub mod aot;
pub mod baseline;
pub mod compiler;
pub mod deopt;
pub mod helpers;
pub mod optimizing;
pub mod osr;
pub mod profile;
pub mod tier;

pub use aot::{
    hash_source, helper_indices, AotCache, AotCacheHeader, AotError, CacheStats, CachedCode,
    Relocation, RelocationType, RuntimeHelperTable, AOT_MAGIC, AOT_VERSION,
};
pub use baseline::{BaselineCompiler, CompiledCode, JitError, TranslationState};
pub use compiler::{CompilationFailure, CompiledFunction, ExecutionMode, FunctionId, TieredJit};
pub use deopt::{
    DeoptFrameBuilder, DeoptFrameState, DeoptManager, DeoptMetadata, DeoptResult, DeoptStatistics,
    DeoptValue, ValueLocation, TypeGuard, TypeGuardKind, DeoptHandler, InterpreterFallbackState,
    type_tags, rt_type_guard_is_int, rt_type_guard_is_float, rt_type_guard_is_string,
    rt_type_guard_is_not_none, rt_trigger_deopt, rt_check_type_or_deopt,
};
pub use helpers::{
    rt_call_function, rt_call_method, rt_contains, rt_contains_dict, rt_contains_list,
    rt_contains_string, rt_power, rt_string_compare, rt_string_concat, rt_string_repeat,
    RuntimeHelpers,
};
pub use optimizing::{DeoptPoint, DeoptReason, OptimizedCode, OptimizingCompiler, Specialization};
pub use osr::OsrManager;
pub use profile::{FunctionProfile, PyType, TypeFeedback, TypeFeedbackSummary, TypeState};
pub use tier::CompilationTier;
