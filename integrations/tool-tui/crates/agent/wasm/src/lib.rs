//! # DX WASM
//!
//! WASM compilation and runtime for DX Agent.
//!
//! This crate enables the AGI-like capability of creating new integrations
//! by compiling code from any language (Python, JavaScript, Go, Rust) to
//! WebAssembly and executing it within the DX runtime.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    DX WASM COMPILER                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  Python ─────┐                                              │
//! │              │                                              │
//! │  JavaScript ─┼──→ WASM Compiler ──→ .wasm ──→ DX Runtime   │
//! │              │                                              │
//! │  Go ─────────┤                                              │
//! │              │                                              │
//! │  Rust ───────┘                                              │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Supported Languages
//!
//! | Language | Compiler | Status |
//! |----------|----------|--------|
//! | Python | Pyodide / Pyo3 | Planned |
//! | JavaScript | wasm-bindgen | Planned |
//! | Go | TinyGo | Planned |
//! | Rust | rustc | Supported |
//! | WASM | (native) | Supported |

pub mod compiler;
pub mod module;
pub mod runtime;

pub use compiler::{CompilerConfig, WasmCompiler};
pub use module::{ModuleExports, WasmModule};
pub use runtime::{RuntimeConfig, WasmRuntime};

use thiserror::Error;

/// WASM-specific errors
#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("Invalid WASM: {0}")]
    InvalidWasm(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, WasmError>;
