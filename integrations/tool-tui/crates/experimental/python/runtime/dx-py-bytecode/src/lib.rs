//! DX-Py Bytecode (DPB) - Zero Parse Binary Format
//!
//! This crate implements the Binary Python Bytecode format, a zero-parse
//! binary format that replaces CPython's .pyc files with 25x faster loading.

pub mod compiler;
pub mod format;
pub mod loader;
pub mod printer;

pub use compiler::DpbCompiler;
pub use format::*;
pub use loader::DpbLoader;
pub use printer::DpbPrettyPrinter;
