//! uv configuration compatibility module
//!
//! Provides functionality for parsing and applying uv configuration files.

mod config;

pub use config::{MergedConfig, PythonPreference, UvConfig, UvConfigLoader};
