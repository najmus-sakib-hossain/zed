//! # DX Markdown (DXM)
//!
//! Context compiler that transforms standard Markdown into token-optimized output for LLMs.
//!
//! DXM achieves 15-65% token reduction (varies by content type) compared to standard Markdown
//! while preserving 100% semantic content.
//!
//! ## Triple Format Architecture (2026 Update)
//!
//! - **Human Format** (Front-facing .md files) - Readable, standard Markdown on disk
//! - **LLM Format** (.dx/markdown/*.llm) - Token-optimized for AI consumption
//! - **Machine Format** (.dx/markdown/*.machine) - Binary for performance
//!
//! ### New Architecture (January 2026)
//! - Front-facing .md files now contain **Human format** (readable)
//! - LLM format moved to `.dx/markdown/*.llm` (token-optimized)
//! - Machine format remains in `.dx/markdown/*.machine` (binary)
//!
//! ## Core API (Stable)
//!
//! The public API is intentionally minimal for 1.0 stability:
//!
//! ```rust,ignore
//! use dx_markdown::{DxMarkdown, CompilerConfig, CompileResult};
//!
//! // Core compilation
//! let compiler = DxMarkdown::new(CompilerConfig::default())?;
//! let result = compiler.compile(input)?;
//! println!("Saved {:.1}% tokens", result.savings_percent());
//! ```
//!
//! ## Advanced Features (Unstable)
//!
//! Advanced features are available through feature flags and may change:
//! - `wasm`: WebAssembly bindings
//! - Git integration, benchmarking, and format conversion are internal APIs

// Allow expect/unwrap in test code
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]
// Allow dead_code for internal modules that are used through re-exports and internal APIs.
// Rust's dead code analysis is conservative and doesn't track all usage patterns in library crates.
#![allow(dead_code)]

// ============================================================================
// CORE MODULES (Internal - not part of stable API)
// ============================================================================
mod analysis;
mod auto_formatter;
pub mod beautifier;
// OLD CUSTOM BINARY FORMAT - COMMENTED OUT (now using RKYV via machine module)
// pub mod binary;
pub mod compiler;
pub mod convert;
pub mod diagrams;
mod dictionary;
pub mod figlet_manager;
mod filler;
pub mod filter;
mod format_detector;
pub mod human_formatter;
pub mod human_llm_converter;
mod human_parser;
pub mod machine;
pub mod markdown;
mod minify;
pub mod parser;
pub mod red_list_config;
mod refs;
pub mod section_filter;
mod security;
mod serializer;
pub mod simd;
mod table;
mod table_renderer;
mod token;
pub mod token_optimizer;
pub mod tokenizer;
pub mod types;
pub mod workflow;

// ============================================================================
// EXPERIMENTAL MODULES (Unstable - may be removed or changed)
// ============================================================================
#[doc(hidden)]
pub mod benchmark;
#[doc(hidden)]
pub mod git;
#[doc(hidden)]
pub mod llm_models;

// Re-export benchmark types for examples
#[doc(hidden)]
pub use benchmark::{BenchmarkReporter, BenchmarkRunner, BenchmarkTokenizer, FileCategory};

// ============================================================================
// PUBLIC API - Error types (Stable)
// ============================================================================
pub mod error;
pub use error::{CompileError, ConvertError, ParseError};

// ============================================================================
// PUBLIC API - Core Types (Stable for 1.0)
// ============================================================================

// Core compiler
pub use compiler::DxMarkdown;

// Configuration
pub use types::{CompileResult, CompilerConfig, SavingsBreakdown, TokenizerType};

// Core types (for external crates that need to work with DXM documents)
pub use types::{
    CellValue, CodeBlockNode, ColumnDef, DxmDocument, DxmMeta, DxmNode, HeaderNode, InlineNode,
    ListItem, ListNode, Priority, SemanticBlockNode, SemanticBlockType, TableNode,
};

// Tokenizer (needed for benchmarks and examples)
pub use tokenizer::Tokenizer;

// Token optimizer (integrates dx-serializer token counter)
pub use token_optimizer::{
    OptimizationStrategy, OptimizationSuggestion, TokenAnalysis, TokenOptimizationResult,
    TokenOptimizer,
};

// Section filtering for LLM optimization
pub use section_filter::{
    SectionFilterConfig, SectionInfo, SectionType, analyze_sections, filter_sections,
};

// Human ↔ LLM conversion with section filtering
pub use human_llm_converter::{
    HumanFileWatcher, generate_human_llm_from_md, human_format_to_llm, llm_format_to_human,
    md_to_human_format, regenerate_llm_from_human_file,
};

// Markdown beautifier and workflow
pub use beautifier::{BeautifierConfig, MarkdownBeautifier, autofix_markdown, lint_markdown};
pub use workflow::{MarkdownWorkflow, WorkflowConfig, WorkflowResult};

// ============================================================================
// PUBLIC API - Format Conversion Functions (Stable)
// ============================================================================

// Auto-detection and parsing
pub use convert::{
    auto_parse, doc_to_human, doc_to_llm, doc_to_machine, human_to_llm, llm_to_human,
    machine_to_human,
};

// Individual optimization functions (for benchmarks)
pub use compiler::{strip_badges, strip_images, strip_urls};

// ============================================================================
// WASM BINDINGS (Feature-gated)
// ============================================================================
#[cfg(feature = "wasm")]
pub mod wasm;

// ============================================================================
// INTERNAL TEST MODULES
// ============================================================================
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)] // Test code can use expect/unwrap
mod round_trip_tests;
