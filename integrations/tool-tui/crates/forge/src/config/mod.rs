//! Configuration module for DX Forge
//!
//! Provides configuration validation and management.

pub mod validator;

pub use validator::{ConfigValidator, ValidationError, ValidationResult};
