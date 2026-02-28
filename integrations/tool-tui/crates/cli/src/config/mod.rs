//! Configuration module
//!
//! Provides configuration loading and management for DX projects.
//!
//! ## Submodules
//!
//! - `dx_config` - Project-level `dx.toml` configuration
//! - `gateway_config` - Gateway/CLI configuration types (YAML)
//! - `env` - Environment variable substitution
//! - `includes` - File includes with deep merge
//! - `schema` - JSON Schema generation and validation
//! - `config_validation` - Semantic config validation
//! - `watcher` - File watcher with hot-reload
//! - `migration` - TOML → YAML migration
//! - `encryption` - AES-256-GCM secret encryption
//! - `defaults` - Default configuration generation
//! - `manager` - Central configuration manager
//! - `formats` - Format configuration for serializer and markdown

pub mod dx_config;
pub mod formats;

// --- Gateway Configuration System (Sprint 1.3) ---
pub mod config_validation;
pub mod defaults;
pub mod encryption;
pub mod env;
pub mod gateway_config;
pub mod includes;
pub mod manager;
pub mod migration;
pub mod schema;
pub mod watcher;

// Re-export configuration types for public API
// These are intentionally exported for library consumers even if not used internally
#[allow(unused_imports)]
pub use dx_config::{
    BuildConfig, DEFAULT_CONFIG_FILE, DevConfig, DxConfig, FontToolConfig, IconToolConfig,
    MediaToolConfig, ProjectConfig, RuntimeConfig, StyleToolConfig, ToolsConfig,
};

// Re-export gateway config types
#[allow(unused_imports)]
pub use formats::{FormatConfig, SourceFormat};
#[allow(unused_imports)]
pub use gateway_config::GatewayCliConfig;
#[allow(unused_imports)]
pub use manager::ConfigManager;
