//! # dx-compiler Library API
//!
//! The core compilation library for the dx-www framework. This crate provides
//! the complete TSX-to-binary compilation pipeline, transforming React-like
//! components into highly optimized binary artifacts.
//!
//! ## Overview
//!
//! The dx-compiler transforms TSX/JSX source files through a multi-stage pipeline:
//!
//! 1. **Parsing** - TSX source → Abstract Syntax Tree (AST)
//! 2. **Analysis** - Determine runtime variant (Micro 338B or Macro 7.5KB)
//! 3. **Tree Shaking** - Remove unused code paths
//! 4. **Splitting** - Separate static templates from dynamic bindings
//! 5. **Code Generation** - Generate HTIP binary and Rust code
//! 6. **Packing** - Create final `.dxb` artifact
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use dx_compiler::{compile_tsx, analyze_tsx, can_compile};
//! use std::path::Path;
//!
//! // Check if a file can be compiled
//! let entry = Path::new("src/App.tsx");
//! if can_compile(entry) {
//!     // Analyze without compiling
//!     let (metrics, variant) = analyze_tsx(entry, false).unwrap();
//!     println!("Runtime: {:?}, Components: {}", variant, metrics.component_count);
//!     
//!     // Full compilation
//!     let output = Path::new("dist");
//!     let result = compile_tsx(entry, output, true).unwrap();
//!     println!("Compiled in {}ms, size: {} bytes",
//!              result.compile_time_ms, result.total_size);
//! }
//! ```
//!
//! ## Architecture
//!

// Allow certain clippy lints for this compiler crate
#![allow(clippy::collapsible_if)] // Nested if statements improve readability for parsing logic
#![allow(clippy::should_implement_trait)] // Custom add/next methods are intentional
#![allow(clippy::regex_creation_in_loops)] // Regex creation in loops is acceptable for parsing
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   TSX/JSX   │────▶│   Parser    │────▶│  Splitter   │────▶│   Codegen   │
//! │   Source    │     │   (OXC)     │     │             │     │   (HTIP)    │
//! └─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
//!                                                                    │
//!                                                                    ▼
//!                                                              ┌─────────────┐
//!                                                              │   .dxb      │
//!                                                              │   Binary    │
//!                                                              └─────────────┘
//! ```
//!
//! ## Runtime Variants
//!
//! The compiler automatically selects the optimal runtime based on application complexity:
//!
//! - **Micro Runtime (338 bytes)** - For simple, static-heavy applications
//! - **Macro Runtime (7.5 KB)** - For complex, interactive applications
//!
//! ## Feature Flags
//!
//! - `oxc` - Enable OXC-based parser for improved performance (recommended)
//!
//! ## Modules
//!
//! - [`parser`] - TSX/JSX parsing and AST generation
//! - [`splitter`] - Template/binding separation
//! - [`codegen`] - HTIP binary generation
//! - [`analyzer`] - Complexity analysis and runtime selection
//! - [`linker`] - Symbol resolution and project scanning

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;

// Re-export modules
pub mod analyzer;
pub mod codegen;
pub mod codegen_macro;
pub mod codegen_micro;
pub mod config;
pub mod dev_server;
pub mod dx_parser; // DX format parser (.pg, .cp, .lyt files)
pub mod errors;
pub mod linker;
pub mod loader;
pub mod packer;
pub mod parser;
pub mod pwa;
pub mod rpc;
pub mod schema_parser;
pub mod splitter;

// Binary Dawn Features - Core binary structures
pub mod animation;
pub mod components; // shadcn-style component library
pub mod content;
pub mod control;
pub mod cron;
pub mod di;
pub mod forms;
pub mod guards;
pub mod handlers;
pub mod islands;
pub mod jobs;
pub mod keepalive;
pub mod liveview;
pub mod optimistic;
pub mod reactivity;
pub mod resumability;
pub mod router;
pub mod server_component;
pub mod streaming;
pub mod suspense;
pub mod teleport;
pub mod transitions;

// Production-ready features (10/10)
pub mod binary_compiler; // .dxob binary compilation
pub mod devtools; // Browser DevTools extension
pub mod hmr; // Hot Module Replacement
pub mod turso; // Turso database integration
pub mod wasm_compiler; // Multi-language WASM compilation

// Binary Dawn Features - Progressive Enhancement and Code Splitting
pub mod code_splitting;
pub mod progressive;

// Binary Dawn Features - Type Safety and Admin
pub mod admin;
pub mod types;

// OXC-based parser (optional - requires oxc feature)
#[cfg(feature = "oxc")]
pub mod swc_parser;

// Ecosystem integrations
pub mod cmd;
pub mod ecosystem;
pub mod feature_tree_shaking;
pub mod template_registry;
pub mod www_config;

pub use analyzer::*;
pub use linker::*;

/// Result of a successful TSX compilation.
///
/// Contains paths to all generated artifacts and metadata about the compilation
/// process, including timing information and the selected runtime variant.
///
/// # Example
///
/// ```rust,ignore
/// use dx_compiler::compile_tsx;
/// use std::path::Path;
///
/// let result = compile_tsx(
///     Path::new("src/App.tsx"),
///     Path::new("dist"),
///     false
/// ).unwrap();
///
/// println!("Generated files:");
/// println!("  HTIP: {}", result.htip_path.display());
/// println!("  Templates: {}", result.templates_path.display());
/// if let Some(rust) = &result.rust_path {
///     println!("  Rust: {}", rust.display());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// The runtime variant that was selected (micro or macro)
    pub runtime_variant: analyzer::RuntimeVariant,
    /// Complexity metrics that drove the decision
    pub metrics: analyzer::ComplexityMetrics,
    /// Path to the generated HTIP binary file
    pub htip_path: PathBuf,
    /// Path to the generated templates JSON file
    pub templates_path: PathBuf,
    /// Path to the generated Rust code (if any)
    pub rust_path: Option<PathBuf>,
    /// Total compilation time in milliseconds
    pub compile_time_ms: u128,
    /// Total size of output artifacts in bytes
    pub total_size: u64,
}

/// Compile a TSX entry file to optimized binary artifacts.
///
/// This is the main entry point for the dx-compiler. It performs the complete
/// compilation pipeline from TSX source to binary artifacts.
///
/// # Arguments
///
/// * `entry` - Path to the entry `.tsx` file (e.g., `src/App.tsx`)
/// * `output` - Directory where artifacts will be written (created if needed)
/// * `verbose` - Enable verbose logging to stdout
///
/// # Returns
///
/// A [`CompileResult`] containing paths to generated artifacts and metadata.
///
/// # Errors
///
/// Returns an error if:
/// - The entry file cannot be read or parsed
/// - The output directory cannot be created
/// - Code generation fails
/// - File writing fails
///
/// # Example
///
/// ```rust,ignore
/// use dx_compiler::compile_tsx;
/// use std::path::Path;
///
/// let result = compile_tsx(
///     Path::new("src/App.tsx"),
///     Path::new("dist"),
///     true  // verbose output
/// )?;
///
/// println!("Compilation complete!");
/// println!("  Runtime: {:?}", result.runtime_variant);
/// println!("  Time: {}ms", result.compile_time_ms);
/// println!("  Size: {} bytes", result.total_size);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn compile_tsx(entry: &Path, output: &Path, verbose: bool) -> Result<CompileResult> {
    let start_time = Instant::now();

    // Ensure output directory exists
    std::fs::create_dir_all(output).context("Failed to create output directory")?;

    if verbose {
        println!("🏭 Compiling {} → {}", entry.display(), output.display());
    }

    // Step 0: Linker Scan
    let search_root = entry.parent().unwrap_or_else(|| Path::new("."));
    let symbol_table = linker::scan_project(search_root, verbose)?;

    // Step 1: Parse
    let parsed_ast =
        parser::parse_entry(entry, &symbol_table, verbose).context("Failed to parse entry file")?;

    // Step 2: Analyze & Decide
    let (metrics, runtime_variant) = analyzer::analyze_and_decide(&parsed_ast, verbose)?;

    if verbose {
        println!(
            "  🧠 {} runtime selected",
            match runtime_variant {
                analyzer::RuntimeVariant::Micro => "Micro (338B)",
                analyzer::RuntimeVariant::Macro => "Macro (7.5KB)",
            }
        );
    }

    // Step 3: Tree Shake
    let shaken = parser::tree_shake(parsed_ast, verbose)?;

    // Step 4: Split
    let (templates, bindings, state_schema) = splitter::split_components(shaken, verbose)?;

    // Step 5: Generate HTIP Binary
    let (htip_stream, _string_table) =
        codegen::generate_htip(&templates, &bindings, &state_schema, verbose)?;

    // Step 6: Write HTIP to disk
    let htip_path = output.join("app.htip");
    std::fs::write(&htip_path, &htip_stream)?;

    // Step 7: Generate templates.json
    let templates_json = serde_json::to_string_pretty(&templates)?;
    let templates_path = output.join("templates.json");
    std::fs::write(&templates_path, &templates_json)?;

    // Step 8: Generate Rust code based on runtime variant
    let rust_path = if runtime_variant == analyzer::RuntimeVariant::Micro {
        let rust_code =
            codegen_micro::generate_micro(&templates, &bindings, &state_schema, verbose)?;
        let path = output.join("generated.rs");
        std::fs::write(&path, &rust_code)?;
        Some(path)
    } else {
        // Macro mode
        codegen_macro::serialize_layout(&templates, output)?;
        let rust_code =
            codegen_macro::generate_macro(&templates, &bindings, &state_schema, verbose)?;
        let path = output.join("generated.rs");
        std::fs::write(&path, &rust_code)?;
        Some(path)
    };

    // Step 9: Pack into .dxb (using pack_dxb_htip for compatibility)
    packer::pack_dxb_htip(output, &templates, &htip_stream, verbose)?;

    // Calculate total size
    let mut total_size = 0u64;
    let dxb_path = output.join("app.dxb");
    if htip_path.exists() {
        total_size += std::fs::metadata(&htip_path)?.len();
    }
    if dxb_path.exists() {
        total_size += std::fs::metadata(&dxb_path)?.len();
    }

    let compile_time_ms = start_time.elapsed().as_millis();

    if verbose {
        println!("✓ Compilation complete in {}ms", compile_time_ms);
        println!("  Total size: {} bytes", total_size);
    }

    Ok(CompileResult {
        runtime_variant,
        metrics,
        htip_path,
        templates_path,
        rust_path,
        compile_time_ms,
        total_size,
    })
}

/// Analyze a TSX file and return complexity metrics without compiling.
///
/// This is useful for build tools that want to understand the application
/// complexity and runtime selection without performing a full compilation.
/// Much faster than [`compile_tsx`] when you only need metrics.
///
/// # Arguments
///
/// * `entry` - Path to the entry `.tsx` file
/// * `verbose` - Enable verbose logging
///
/// # Returns
///
/// A tuple of ([`ComplexityMetrics`], [`RuntimeVariant`]) indicating the
/// application's complexity and which runtime would be selected.
///
/// # Example
///
/// ```rust,no_run
/// use dx_compiler::{analyze_tsx, analyzer::RuntimeVariant};
/// use std::path::Path;
///
/// let (metrics, variant) = analyze_tsx(Path::new("src/App.tsx"), false)?;
///
/// match variant {
///     RuntimeVariant::Micro => println!("Simple app - using 338B runtime"),
///     RuntimeVariant::Macro => println!("Complex app - using 7.5KB runtime"),
/// }
///
/// println!("Component count: {}", metrics.component_count);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn analyze_tsx(
    entry: &Path,
    verbose: bool,
) -> Result<(analyzer::ComplexityMetrics, analyzer::RuntimeVariant)> {
    let search_root = entry.parent().unwrap_or_else(|| Path::new("."));
    let symbol_table = linker::scan_project(search_root, verbose)?;
    let parsed_ast = parser::parse_entry(entry, &symbol_table, verbose)?;
    analyzer::analyze_and_decide(&parsed_ast, verbose)
}

/// Quick compilation check - returns true if entry file can be compiled.
///
/// This is a fast check that only parses the entry file without performing
/// full compilation. Useful for IDE integrations and build tool validation.
///
/// # Arguments
///
/// * `entry` - Path to the entry `.tsx` file
///
/// # Returns
///
/// `true` if the file can be parsed successfully, `false` otherwise.
///
/// # Example
///
/// ```rust,no_run
/// use dx_compiler::can_compile;
/// use std::path::Path;
///
/// let files = vec!["App.tsx", "Header.tsx", "Footer.tsx"];
/// for file in files {
///     let path = Path::new("src").join(file);
///     if can_compile(&path) {
///         println!("✓ {} is valid", file);
///     } else {
///         println!("✗ {} has errors", file);
///     }
/// }
/// ```
pub fn can_compile(entry: &Path) -> bool {
    let symbol_table = linker::SymbolTable::new();
    parser::parse_entry(entry, &symbol_table, false).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_compile_tsx_basic() {
        let temp = TempDir::new().unwrap();
        let entry = temp.path().join("App.tsx");
        let output = temp.path().join("dist");

        // Create a simple TSX file
        fs::write(
            &entry,
            r#"
export default function App() {
    return <div>Hello World</div>;
}
        "#,
        )
        .unwrap();

        let result = compile_tsx(&entry, &output, false);
        assert!(result.is_ok(), "Compilation should succeed");

        let compile_result = result.unwrap();
        assert!(compile_result.htip_path.exists());
        assert!(compile_result.templates_path.exists());
    }

    #[test]
    fn test_analyze_tsx() {
        let temp = TempDir::new().unwrap();
        let entry = temp.path().join("App.tsx");

        fs::write(
            &entry,
            r#"
import { useState } from 'dx';
export default function App() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
        "#,
        )
        .unwrap();

        let result = analyze_tsx(&entry, false);
        assert!(result.is_ok(), "Analysis should succeed");
    }
}
