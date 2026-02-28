//! DX-Py Decorators - Compiler-Inlined Decorators
//!
//! This crate implements compiler-inlined decorators for zero-overhead
//! decorator application at compile time.

pub mod dataclass;
pub mod inlineable;
pub mod inliner;
pub mod lru_cache;

pub use dataclass::DataclassInfo;
pub use inlineable::InlineableDecorator;
pub use inliner::DecoratorInliner;
pub use lru_cache::InlineLruCache;
