//! Built-in Scoring Plugins
//!
//! Provides default plugins for security, design patterns, and structure analysis.

mod patterns;
mod security;
mod structure;

pub use patterns::PatternsPlugin;
pub use security::SecurityPlugin;
pub use structure::StructurePlugin;
