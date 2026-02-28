//! # dx-compat-shell
//!
//! Shell scripting compatibility layer.
//!
//! Provides Bun-compatible shell scripting with:
//! - Template literal-style command execution
//! - Safe argument escaping
//! - Command chaining and pipelines
//! - Environment variable support

#![warn(missing_docs)]

mod command;
mod error;
mod output;

pub use command::{escape, Pipeline, ShellCommand};
pub use error::{ShellError, ShellResult};
pub use output::ShellOutput;

/// Execute a shell command asynchronously.
pub async fn shell(cmd: &str, args: &[&str]) -> ShellResult<ShellOutput> {
    ShellCommand::new(cmd).args(args).run().await
}

/// Execute a shell command synchronously.
pub fn shell_sync(cmd: &str, args: &[&str]) -> ShellResult<ShellOutput> {
    ShellCommand::new(cmd).args(args).run_sync()
}

/// Parse and execute a shell string.
pub async fn exec(shell_str: &str) -> ShellResult<ShellOutput> {
    ShellCommand::parse(shell_str).run().await
}

/// Parse and execute a shell string synchronously.
pub fn exec_sync(shell_str: &str) -> ShellResult<ShellOutput> {
    ShellCommand::parse(shell_str).run_sync()
}
