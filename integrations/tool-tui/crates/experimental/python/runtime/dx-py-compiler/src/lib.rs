//! DX-Py Source-to-Bytecode Compiler
//!
//! This crate provides the compiler that transforms Python AST to DPB bytecode.
//! It bridges the parser and bytecode modules to enable direct execution of
//! Python source files.
//!
//! ## Features
//!
//! - Symbol table for name resolution (local, global, free, cell variables)
//! - Bytecode emitter with jump patching
//! - AST-to-bytecode compilation for all Python constructs
//! - Error reporting with line numbers
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_py_compiler::SourceCompiler;
//!
//! let source = "x = 1 + 2";
//! let mut compiler = SourceCompiler::new("<stdin>".into());
//! let code = compiler.compile_module_source(source)?;
//! ```

pub mod compiler;
pub mod emitter;
pub mod error;
pub mod symbol_table;

pub use compiler::SourceCompiler;
pub use emitter::BytecodeEmitter;
pub use error::{CompileError, CompileResult};
pub use symbol_table::{Scope, ScopeType, Symbol, SymbolTable};
