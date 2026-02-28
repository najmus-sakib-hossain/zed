//! # dx-compat-ffi
//!
//! Foreign Function Interface compatibility layer.
//!
//! Provides safe wrappers for:
//! - Dynamic library loading (DLL, dylib, .so)
//! - C ABI function calls
//! - Pointer operations and memory management
//! - Struct layout definitions

#![warn(missing_docs)]

mod error;
mod library;
mod types;

pub use error::{FfiError, FfiResult};
pub use library::{dlopen, library_name, library_prefix, library_suffix, DynamicLibrary};
pub use types::{
    read_cstring, CStringWrapper, FfiType, FfiValue, StructField, StructLayout, StructLayoutBuilder,
};

/// Pointer operations module.
pub mod ptr;
