//! CLI Command Handlers
//!
//! Individual command implementations for the driven CLI.

mod analyze;
mod benchmark;
mod cache;
mod convert;
mod hook;
mod init;
mod module;
mod sign;
mod steer;
mod sync;
mod template;
mod validate;

pub use analyze::AnalyzeCommand;
pub use benchmark::{BenchmarkCommand, BenchmarkResults};
pub use cache::{CacheCommand, CacheStats};
pub use convert::ConvertCommand;
pub use hook::{HookCommand, HookInfo, TriggerResult, print_hook_details, print_hooks_table};
pub use init::InitCommand;
pub use module::{
    InstallResult, ModuleCommand, ModuleDetails, ModuleInfo, UninstallResult, UpdateResult,
    print_module_details, print_modules_table,
};
pub use sign::{SignCommand, generate_keypair};
pub use steer::{SteerCommand, SteeringInfo, print_steering_details, print_steering_table};
pub use sync::SyncCommand;
pub use template::TemplateCommand;
pub use validate::ValidateCommand;

use crate::Result;
use std::path::Path;

/// Common options for all commands
#[derive(Debug, Clone)]
pub struct CommonOptions {
    /// Project root path
    pub project_root: std::path::PathBuf,
    /// Verbosity level (0-3)
    pub verbosity: u8,
    /// Enable color output
    pub color: bool,
}

impl Default for CommonOptions {
    fn default() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_default(),
            verbosity: 1,
            color: true,
        }
    }
}

/// Print a success message
pub fn print_success(msg: &str) {
    use console::style;
    println!("{} {}", style("✓").green().bold(), msg);
}

/// Print an error message
pub fn print_error(msg: &str) {
    use console::style;
    eprintln!("{} {}", style("✗").red().bold(), msg);
}

/// Print a warning message
pub fn print_warning(msg: &str) {
    use console::style;
    println!("{} {}", style("!").yellow().bold(), msg);
}

/// Print an info message
pub fn print_info(msg: &str) {
    use console::style;
    println!("{} {}", style("ℹ").blue(), msg);
}

/// Create a progress bar
pub fn create_progress_bar(len: u64, message: &str) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};

    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a spinner
pub fn create_spinner(message: &str) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap());
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Resolve project root
pub fn resolve_project_root(path: Option<&Path>) -> Result<std::path::PathBuf> {
    match path {
        Some(p) => Ok(p.to_path_buf()),
        None => std::env::current_dir().map_err(|e| {
            crate::DrivenError::Config(format!("Failed to get current directory: {}", e))
        }),
    }
}
