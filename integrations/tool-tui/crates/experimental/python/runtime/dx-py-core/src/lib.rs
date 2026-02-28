//! Core object model for DX-Py runtime
//!
//! Provides the fundamental Python object types and runtime structures.
//!
//! ## Features
//!
//! - Lock-free reference counting via PyObjectHeader
//! - Core Python types: int, str, list, dict, tuple, function
//! - Stack frames for execution
//! - Built-in functions (print, len, type, range, etc.)
//! - Standard library modules (sys, os, io, json)
//! - Debugging support with line tables and tracebacks
//!
//! ## Error Handling
//!
//! All operations return `RuntimeResult<T>` for graceful error handling.

// Allow CPython API naming conventions for compatibility
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
// Allow some clippy lints for CPython API compatibility and code style
#![allow(clippy::result_large_err)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::useless_format)]
#![allow(clippy::format_in_format_args)]
#![allow(clippy::get_first)]
#![allow(clippy::ineffective_open_options)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::iter_cloned_collect)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::needless_return)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(mismatched_lifetime_syntaxes)]

pub mod buffer;
pub mod builtins;
pub mod capi;
pub mod cleanup;
pub mod context;
pub mod debug;
pub mod error;
pub mod gc;
pub mod gil;
pub mod header;
pub mod pydict;
pub mod pyexception;
pub mod pyframe;
pub mod pyfunction;
pub mod pygenerator;
pub mod pyint;
pub mod pylist;
pub mod pystr;
pub mod pytuple;
pub mod stdlib;
pub mod types;
pub mod weakref;
pub mod asyncio_runtime;

pub use buffer::{BufferInfo, BufferManager, BufferProvider};
pub use capi::{is_c_extension_loaded, load_c_extension, ExtensionLoader};
pub use cleanup::{CleanupManager, Finalizable};
pub use context::{with_context, ContextManager, EnterResult, ExitResult};
pub use debug::{Debugger, ExceptionInfo, LineTable, Traceback, TracebackFrame};
pub use error::{RuntimeError, RuntimeResult};
pub use gc::{add_potential_cycle, collect, force_collect, stats, GcStats};
pub use gil::{
    acquire_gil, gil_is_enabled, gil_is_held, gil_is_locked, set_gil_enabled, try_acquire_gil,
    try_acquire_gil_timeout, with_gil, without_gil, AllowThreadsGuard, Gil, GilGuard,
    PyGILState_STATE,
};
pub use header::PyObjectHeader;
pub use pydict::PyDict;
pub use pyexception::{exceptions, PyException};
pub use pyframe::PyFrame;
pub use pyfunction::{PyFunction, JIT_COMPILATION_THRESHOLD};
pub use pygenerator::{
    CoroutineResult, CoroutineState, GeneratorResult, GeneratorState, PyCoroutine, PyGenerator,
};
pub use pyint::PyInt;
pub use pylist::{PyCell, PyCode, PyIterator, PyList, PyModule, PySet, PyValue};
pub use pystr::PyStr;
pub use pytuple::PyTuple;
pub use types::{PyInstance, PySuper, PyType, PyTypeSlot, TypeFlags};
pub use weakref::{
    clear_all_weak_references, clear_weak_references, get_weak_ref_count, PyWeakProxy, PyWeakRef,
};
pub use asyncio_runtime::{run_coroutine, gather_coroutines, EventLoop};

/// Legacy error types module (deprecated, use RuntimeError instead)
#[allow(deprecated)]
mod legacy_errors {
    /// Legacy error types (deprecated, use RuntimeError instead)
    #[derive(Debug, thiserror::Error)]
    #[deprecated(since = "0.2.0", note = "Use RuntimeError instead")]
    pub enum CoreError {
        #[error("Type error: {0}")]
        TypeError(String),

        #[error("Value error: {0}")]
        ValueError(String),

        #[error("Index error: {0}")]
        IndexError(String),

        #[error("Key error: {0}")]
        KeyError(String),

        #[error("Attribute error: {0}")]
        AttributeError(String),

        #[error("Name error: {0}")]
        NameError(String),

        #[error("Runtime error: {0}")]
        RuntimeError(String),

        #[error("Overflow error")]
        OverflowError,
    }

    pub type CoreResult<T> = Result<T, CoreError>;
}

#[allow(deprecated)]
pub use legacy_errors::{CoreError, CoreResult};
