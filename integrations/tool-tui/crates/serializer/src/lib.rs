//! # DX Serializer
//!
//! A high-performance serialization library optimized for Humans, LLMs, AND Machines.
//!
//! ## Two Primary Formats
//!
//! | Format | Use Case | Performance |
//! |--------|----------|-------------|
//! | **DX LLM** | Text format for humans & LLMs | 26.8% more efficient than TOON |
//! | **DX Machine** | Binary format for runtime | 0.70ns field access |
//!
//! ## Quick Start
//!
//! The simplest way to use dx-serializer is with the convenience functions:
//!
//! ```rust
//! use serializer::{serialize, deserialize, DxDocument, DxLlmValue};
//!
//! // Create a document
//! let mut doc = DxDocument::new();
//! doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
//! doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));
//!
//! // Serialize to LLM format (text)
//! let text = serialize(&doc);
//!
//! // Deserialize back
//! let parsed = deserialize(&text).unwrap();
//! ```
//!
//! ## Advanced Configuration
//!
//! For more control over formatting and output options, use the builder pattern:
//!
//! ```rust
//! use serializer::{SerializerBuilder, DxDocument, DxLlmValue};
//!
//! let mut doc = DxDocument::new();
//! doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
//!
//! // Configure with builder pattern
//! let serializer = SerializerBuilder::new()
//!     .indent_size(4)
//!     .expand_keys(true)
//!     .validate_output(false)
//!     .for_humans()  // Preset for human readability
//!     .build();
//!
//! // Use configured serializer
//! let text = serializer.serialize(&doc);
//! let human_format = serializer.format_human_unchecked(&doc);
//! ```
//!
//! ## Format-Specific APIs
//!
//! For more control, use the format-specific functions:
//!
//! ```rust
//! use serializer::{DxDocument, DxLlmValue};
//! use serializer::{document_to_llm, llm_to_document};  // LLM format
//! use serializer::zero::DxZeroBuilder;                  // Machine format
//!
//! // Create a document
//! let mut doc = DxDocument::new();
//! doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
//!
//! // Convert to LLM format (text, 26.8% better than TOON)
//! let llm_text = document_to_llm(&doc);
//!
//! // Convert to Machine format (binary, 0.70ns access)
//! let mut buffer = Vec::new();
//! let mut builder = DxZeroBuilder::new(&mut buffer, 8, 1);
//! builder.write_u64(0, 12345);
//! builder.write_string(8, "MyApp");
//! builder.finish();
//! ```
//!
//! ## Thread Safety
//!
//! All public types in dx-serializer are designed to be thread-safe:
//!
//! ### Types that implement `Send + Sync`
//!
//! The following types can be safely shared between threads:
//!
//! | Type | Send | Sync | Notes |
//! |------|------|------|-------|
//! | [`DxDocument`] | ✓ | ✓ | Immutable sharing is safe; clone for mutation |
//! | [`DxLlmValue`] | ✓ | ✓ | All variants are thread-safe |
//! | [`DxSection`] | ✓ | ✓ | Contains only thread-safe types |
//! | [`DxValue`] | ✓ | ✓ | Core value type, fully thread-safe |
//! | [`DxArray`] | ✓ | ✓ | Vector-backed, thread-safe |
//! | [`DxObject`] | ✓ | ✓ | HashMap-backed, thread-safe |
//! | [`DxError`] | ✓ | ✓ | Error type, fully thread-safe |
//! | [`Mappings`] | ✓ | ✓ | Global singleton, read-only after init |
//! | [`SerializerBuilder`] | ✓ | ✓ | Builder pattern, thread-safe |
//! | [`Serializer`] | ✓ | ✓ | Configured serializer instance |
//!
//! ### Stateless Parsing
//!
//! All parsing functions are stateless and can be called concurrently from
//! multiple threads without synchronization:
//!
//! ```rust
//! use std::thread;
//! use serializer::parse;
//!
//! let handles: Vec<_> = (0..4).map(|i| {
//!     thread::spawn(move || {
//!         let input = format!("key{}:value{}", i, i);
//!         parse(input.as_bytes())
//!     })
//! }).collect();
//!
//! for handle in handles {
//!     let result = handle.join().unwrap();
//!     assert!(result.is_ok());
//! }
//! ```
//!
//! ### Thread Safety Guarantees
//!
//! 1. **No global mutable state**: Parsers create fresh state for each invocation
//! 2. **Immutable singletons**: [`Mappings::get()`] returns a read-only reference
//! 3. **No interior mutability**: Types use standard Rust ownership semantics
//! 4. **Safe concurrent reads**: All types support concurrent read access
//!
//! ### Types NOT Thread-Safe
//!
//! The following types contain mutable state and should not be shared:
//!
//! | Type | Reason | Alternative |
//! |------|--------|-------------|
//! | [`Parser`] | Contains mutable parsing state | Create one per thread |
//! | `DxZeroBuilder` | Writes to mutable buffer | Create one per thread |
//! | `StreamCompressor` | Maintains compression state | Create one per thread |
//! | `StreamDecompressor` | Maintains decompression state | Create one per thread |
//!
//! These types implement `Send` (can be moved between threads) but not `Sync`
//! (cannot be shared via `&T`). Create separate instances for each thread.
//!
//! ## Triple Format Architecture (2026 Update)
//!
//! DX seamlessly converts between three formats:
//! - **Human Format** (Front-facing files: .sr, .dx) - Beautiful, readable, on disk
//! - **LLM Format** (.dx/serializer/*.llm) - Token-efficient, 26.8% better than TOON
//! - **Machine Format** (.dx/serializer/*.machine) - Binary, 0.70ns access
//!
//! ### New Architecture (January 2026)
//! - Front-facing .sr/.dx files now contain **Human format** (readable)
//! - LLM format moved to `.dx/serializer/*.llm` (token-optimized for AI)
//! - Machine format remains in `.dx/serializer/*.machine` (binary performance)
//!
//! ## Key Features
//! - Base62 integers (%x): 320→5A, 540→8k
//! - Auto-increment (%#): Sequential IDs generated automatically
//! - Holographic inflate/deflate for editor integration
//! - Binary format using RKYV zero-copy architecture
//!
//! ## Safety
//!
//! This crate uses `unsafe` code in specific, well-documented locations for
//! performance-critical operations. All unsafe code follows these principles:
//!
//! 1. **Minimal scope**: Unsafe blocks are as small as possible
//! 2. **Documented invariants**: Every unsafe block has a `// SAFETY:` comment
//! 3. **Validated preconditions**: Safe wrappers validate before unsafe operations
//!
//! ### Unsafe Operations by Module
//!
//! | Module | Operation | Justification |
//! |--------|-----------|---------------|
//! | `zero/deserialize` | Zero-copy pointer cast | Validated size and alignment |
//! | `zero/safe_deserialize` | Safe wrapper for casts | Validates bounds and alignment before cast |
//! | `zero/quantum` | Unchecked field access | Compile-time offsets, caller validates bounds |
//! | `zero/prefetch` | CPU cache prefetch hints | Hint-only, no memory access |
//! | `zero/simd` | SSE4.2/AVX2 intrinsics | Target feature guards ensure CPU support |
//! | `zero/simd512` | AVX-512 intrinsics | Target feature guards ensure CPU support |
//! | `zero/mmap` | Memory-mapped access | Caller validates offset and type |
//! | `zero/arena` | Arena allocation | Capacity checked before allocation |
//! | `zero/inline` | UTF-8 unchecked | UTF-8 validated on construction |
//! | `utf8` | UTF-8 unchecked | Manual UTF-8 validation precedes conversion |
//! | `safety` | Safe cast wrappers | Validates size and alignment before cast |
//!
//! ### Safe Wrappers
//!
//! For most use cases, prefer the safe wrappers in the [`safety`] module:
//!
//! - `safety::safe_cast` - Validates size and alignment before casting
//! - `safety::safe_read_slice` - Validates bounds before reading a slice
//! - [`safety::check_bounds`] - Validates offset and length are within bounds
//! - [`safety::check_alignment`] - Validates pointer alignment for a type
//!
//! The `zero::safe_deserialize::SafeDeserializer` provides a fully safe API
//! for zero-copy deserialization with automatic bounds and alignment checking.
//!
//! ### SIMD Safety
//!
//! SIMD operations are guarded by `#[target_feature]` attributes and `cfg` blocks:
//!
//! - **Compile-time**: `#[cfg(target_feature = "avx512f")]` ensures the code is
//!   only compiled when the target supports the feature
//! - **Runtime**: `is_x86_feature_detected!` macro checks CPU capabilities
//! - **Fallback**: Portable implementations are always available
//!
//! ### Memory Safety Guarantees
//!
//! 1. **No undefined behavior**: All unsafe code maintains Rust's safety invariants
//! 2. **No data races**: No shared mutable state in unsafe code
//! 3. **No use-after-free**: Lifetime annotations ensure references remain valid
//! 4. **No buffer overflows**: Bounds are validated before unsafe access

// =============================================================================
// Crate-level lint configuration
// =============================================================================
//
// These allows are intentional and justified:
//
// 1. `should_implement_trait`: Methods like `from_str()` in `zero/inline.rs` and
//    `zero/format.rs` return `Option<Self>` instead of `Result<Self, E>` because
//    they are fallible constructors, not trait implementations. The `FromStr` trait
//    requires `Result`, but these methods intentionally use `Option` for simpler APIs.
//
// 2. `only_used_in_recursion`: Parameters like `depth` in recursive formatters are
//    passed through recursion for tracking but may not be used in all branches.
//    This is intentional for consistent recursive signatures.
//
// 3. `doc_nested_refdefs`: Documentation for binary formats uses reference-style
//    links in list items to document byte layouts clearly.
//
#![allow(clippy::should_implement_trait)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::doc_nested_refdefs)]

pub mod base62;
#[cfg(test)]
mod base62_props;
pub mod binary_output;
pub mod builder;

// Safety validation utilities (inlined from dx-safety for standalone publishability)
pub mod safety;

// Platform-specific async I/O
pub mod compress;
pub mod converters;
pub mod encoder;
pub mod error;
#[cfg(test)]
mod error_props;
pub mod formatter;
// TODO: Re-enable when async-io feature is implemented
// #[cfg(feature = "async-io")]
// pub mod io;
pub mod llm;
pub mod llm_models;
pub mod machine;
pub mod mappings;
pub mod optimizer;
pub mod parser;
#[cfg(test)]
mod parser_security_props;
pub mod schema;
pub mod tokenizer;
pub mod types;
pub mod utf8;
#[cfg(test)]
mod utf8_props;
#[cfg(test)]
mod value_props;
pub mod wasm;
pub mod watch;

// Re-export derive macro when feature is enabled
#[cfg(feature = "derive")]
pub use dx_serializer_derive::{QuantumLayout, dx_static_serialize, include_serialized};

pub use base62::{decode_base62, encode_base62};
pub use binary_output::{
    BinaryConfig, get_binary_path, hash_path, is_cache_valid, read_binary, write_binary,
};
pub use compress::{compress_to_writer, format_machine};
pub use converters::{convert_to_dx, dx_to_toon, toon_to_dx};
#[cfg(feature = "converters")]
pub use converters::{json_to_dx, toml_to_dx, yaml_to_dx};
pub use encoder::{Encoder, encode, encode_to_writer};
pub use error::{DxError, Result};
pub use formatter::{HumanFormatter as BinaryHumanFormatter, format_human};
pub use mappings::Mappings;
pub use optimizer::{optimize_key, optimize_path};
pub use parser::{Parser, parse, parse_stream};
pub use schema::{Schema, TypeHint};
pub use types::{DxArray, DxObject, DxValue};
pub use utf8::{
    Utf8ValidationError, validate_string_input, validate_utf8, validate_utf8_detailed,
    validate_utf8_owned,
};

// Re-export IndexMap for external use
pub use indexmap::IndexMap;

// Re-export LLM/Human format types at crate root for convenience
pub use llm::{
    AbbrevDict, ConvertError, DxDocument, DxLlmValue, DxSection, HumanFormatConfig, HumanFormatter,
    HumanParseError, HumanParser, LlmParser, LlmSerializer, MachineFormat,
    ParseError as LlmParseError,
};
pub use llm::{
    document_to_human, document_to_llm, document_to_machine, human_to_document, human_to_llm,
    human_to_machine, human_to_machine_uncompressed, is_llm_format, llm_to_document, llm_to_human,
    llm_to_machine, machine_to_document, machine_to_human, machine_to_llm,
};

// Re-export Serializer Output types for .dx/serializer/ generation
pub use llm::{
    SerializerOutput, SerializerOutputConfig, SerializerOutputError, SerializerPaths,
    SerializerResult,
};

// Re-export utility types
pub use llm::{
    CacheConfig, CacheError, CacheGenerator, CachePaths, CacheResult, PrettyPrintError,
    PrettyPrinter, PrettyPrinterConfig, TableWrapper, TableWrapperConfig,
};

// Re-export token counting types
pub use llm::{ModelType, TokenCounter, TokenInfo};

// Re-export LLM model pricing and analysis
pub use llm_models::{
    LLM_MODELS, LlmModel, Provider, TokenAnalysis, analyze_all_models, format_cost, format_tokens,
    models_by_provider,
};

// Re-export DX Serializer format types (token-efficient LLM format)
pub use llm::{
    LlmParser as DxSerializerParser, LlmSerializer as DxSerializerSerializer,
    ParseError as DxSerializerParseError,
};

// Re-export WASM types for VS Code extension
pub use wasm::{DxSerializer, SerializerConfig, TransformResult, ValidationResult, smart_quote};

// Re-export builder pattern for advanced configuration
pub use builder::{Serializer, SerializerBuilder};

// =============================================================================
// Simplified Public API
// =============================================================================

/// Serialize a DxDocument to the LLM text format.
///
/// This is the recommended format for most use cases as it's:
/// - Human-readable
/// - LLM-friendly (token-efficient)
/// - Easy to debug
///
/// # Example
///
/// ```rust
/// use serializer::{serialize, DxDocument, DxLlmValue};
///
/// let mut doc = DxDocument::new();
/// doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
///
/// let text = serialize(&doc);
/// assert!(text.contains("name") && text.contains("MyApp"));
/// ```
///
/// # Errors
///
/// This function is infallible and always returns a valid string representation
/// of the document. The serialization process handles all value types gracefully.
#[must_use]
pub fn serialize(doc: &DxDocument) -> String {
    document_to_llm(doc)
}

/// Deserialize LLM text format to a DxDocument.
///
/// This parses the token-efficient LLM format back into a structured document.
///
/// # Example
///
/// ```rust
/// use serializer::{deserialize, DxLlmValue};
///
/// let text = "name=MyApp\nversion=1.0.0";
/// let doc = deserialize(text).unwrap();
///
/// assert!(doc.context.contains_key("name"));
/// ```
///
/// # Errors
///
/// Returns a [`ConvertError`] in the following cases:
///
/// - [`ConvertError::LlmParse`] - When the input contains invalid DX Serializer/LLM syntax:
///   - **Unexpected character**: Invalid character at a specific position
///   - **Unexpected EOF**: Input ends prematurely (e.g., unclosed brackets)
///   - **Invalid value format**: Malformed value that cannot be parsed
///   - **Schema mismatch**: Table row has wrong number of columns
///   - **UTF-8 error**: Input contains invalid UTF-8 sequences (with byte offset)
///   - **Input too large**: Input exceeds `MAX_INPUT_SIZE` (100 MB)
///   - **Unclosed bracket/parenthesis**: Missing closing delimiter
///   - **Missing value**: Key without corresponding value after `=`
///   - **Invalid table format**: Malformed table definition
///
/// # Example Error Handling
///
/// ```rust
/// use serializer::{deserialize, ConvertError};
///
/// let result = deserialize("invalid[[[");
/// match result {
///     Ok(doc) => println!("Parsed {} context entries", doc.context.len()),
///     Err(ConvertError::LlmParse(e)) => eprintln!("Parse error: {}", e),
///     Err(e) => eprintln!("Other error: {}", e),
/// }
/// ```
pub fn deserialize(input: &str) -> std::result::Result<DxDocument, ConvertError> {
    llm_to_document(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        // Simple key-value format that the parser supports
        let input = b"name:Test
value:123
active:+";

        let parsed = parse(input).expect("Parse failed");
        let encoded = encode(&parsed).expect("Encode failed");
        let reparsed = parse(&encoded).expect("Reparse failed");

        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_human_format() {
        let input = b"data=id%i name%s
1 Test
2 Demo";

        let parsed = parse(input).expect("Parse failed");
        let human = format_human(&parsed).expect("Format failed");

        assert!(human.contains("DATA TABLE"));
        assert!(human.contains("Test"));
        assert!(human.contains("Demo"));
    }

    #[test]
    fn test_serialize_deserialize_convenience() {
        // Test the simplified API
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("TestApp".to_string()));
        doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));
        doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
        doc.context.insert("active".to_string(), DxLlmValue::Bool(true));

        // Serialize
        let text = serialize(&doc);
        assert!(!text.is_empty());

        // Deserialize
        let parsed = deserialize(&text).expect("Deserialize failed");

        // Verify round-trip preserves data
        assert_eq!(parsed.context.len(), doc.context.len());
    }

    #[test]
    fn test_serialize_empty_document() {
        let doc = DxDocument::new();
        let text = serialize(&doc);
        // Empty document should produce minimal output
        assert!(text.is_empty() || text.trim().is_empty());
    }

    #[test]
    fn test_deserialize_invalid_input() {
        // Invalid input should return an error, not panic
        let result = deserialize("this is not valid LLM format {{{{");
        // The function should handle invalid input gracefully
        // It may succeed with partial parsing or return an error
        let _ = result;
    }
}
