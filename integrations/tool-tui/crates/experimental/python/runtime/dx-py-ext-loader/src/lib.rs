//! DX-Py Extension Loader
//!
//! This crate provides infrastructure for loading CPython C extensions (.pyd/.so files)
//! into the DX-Py runtime. It handles:
//! - Extension discovery across search paths
//! - Platform-specific extension naming (Windows vs Unix)
//! - ABI version detection and compatibility checking
//! - Dynamic library loading via libloading
//! - CPython C API function table for extensions

pub mod abi;
pub mod capi_table;
pub mod discovery;
pub mod error;
pub mod loader;

pub use abi::{AbiCompatibility, AbiVersion};
pub use capi_table::{ApiUsageTracker, CApiTable};
pub use discovery::ExtensionDiscovery;
pub use error::{ExtensionError, ExtensionResult};
pub use loader::{ApiUsageReport, ExtensionLoader, LoadedExtension};
