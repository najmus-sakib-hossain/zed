//! Scoring Module
//!
//! Exports the scoring plugin system and built-in plugins.

pub mod plugin;
pub mod plugins;

pub use plugin::{PluginLoader, PluginRegistry, RuleDefinition, ScoringPlugin};
pub use plugins::{PatternsPlugin, SecurityPlugin, StructurePlugin};
