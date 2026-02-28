//! Bytecode interpreter for DX-Py runtime
//!
//! Implements the dispatch loop for DPB bytecode execution.
//!
//! ## Features
//!
//! - Bytecode dispatch with computed goto optimization
//! - JIT integration for tiered compilation
//! - Async integration for async/await support
//! - Error propagation with Python exception semantics
//! - Exception handling with try/except/finally support

pub mod async_integration;
pub mod dispatch;
pub mod exception_handler;
pub mod jit_integration;
pub mod opcodes;
pub mod vm;

pub use async_integration::{AsyncError, AsyncRuntime, FutureResult};
pub use dispatch::Dispatcher;
pub use exception_handler::{ExceptionHandler, ExceptionHandlerInfo, ExceptionResult, TryBlock};
pub use jit_integration::{JitError, JitIntegration, JitStats};
pub use vm::VirtualMachine;

use dx_py_core::RuntimeError;

/// Interpreter error types
#[derive(Debug, thiserror::Error)]
pub enum InterpreterError {
    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Name error: {0}")]
    NameError(String),

    #[error("Value error: {0}")]
    ValueError(String),

    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Key error: {0}")]
    KeyError(String),

    #[error("Attribute error: {0}")]
    AttributeError(String),

    #[error("Import error: {0}")]
    ImportError(String),

    #[error("Stop iteration")]
    StopIteration,

    #[error("System exit: {0}")]
    SystemExit(i32),

    #[error("{0}")]
    RuntimeError(#[from] RuntimeError),

    #[error("Exception: {0:?}")]
    Exception(dx_py_core::pylist::PyValue),
}

pub type InterpreterResult<T> = Result<T, InterpreterError>;
