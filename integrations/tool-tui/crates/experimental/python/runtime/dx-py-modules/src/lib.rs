//! DX-Py Modules (DPM) - Binary Module Format and Import System
//!
//! This crate implements:
//! - Binary Module Format for pre-compiled Python modules with O(1) symbol lookup
//! - Python module import system with sys.path search, package imports, and relative imports
//! - Module execution engine for bytecode execution and object creation

pub mod compiler;
pub mod executor;
pub mod export_table;
pub mod format;
pub mod importer;
pub mod loader;

pub use compiler::DpmCompiler;
pub use executor::{
    CodeFlags, CodeObject, ExecutionError, ExecutionResult, ModuleExecutor, PyClass, PyFunction,
    PyInstance, PyValue,
};
pub use export_table::ExportTable;
pub use format::*;
pub use importer::{ImportError, ImportSystem, LoaderType, ModuleSpec, ModuleValue, PyModule};
pub use loader::DpmLoader;
