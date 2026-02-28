//! Configuration module
//!
//! Provides configuration types and serialization for dx-py.

mod serde_impl;
mod types;

pub use serde_impl::{parse_and_validate, validate_config, ConfigError};
pub use types::DxPyConfig;
