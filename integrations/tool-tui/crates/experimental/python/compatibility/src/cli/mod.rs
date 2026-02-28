//! CLI compatibility module
//!
//! Provides pip-compatible CLI commands and Python script execution.

mod pip;
mod runner;
mod module_runner;

pub use pip::{PipCommand, PipCommandResult, PipCompatLayer};
pub use runner::{ScriptRunner, ScriptResult, RunnerError};
pub use module_runner::{ModuleRunner, ModuleResult};
