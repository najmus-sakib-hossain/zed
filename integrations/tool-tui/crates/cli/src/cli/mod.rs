//! CLI definition and command routing

// Minimal args for onboarding build
// pub mod args_animation;
// pub mod args_project;
// pub mod args_shell;
// pub mod args_utility;
mod commands;
// mod executor;  // Commented out - depends on all commands
// mod hybrid_executor;  // Commented out - depends on executor
mod styles;
mod types;

// Minimal exports for onboarding build
// pub use args_animation::{AnimateArgs, AnimateCommand, AnimationType, MediaType};
// pub use args_project::{
//     BuildArgs, BuildTarget, DeployArgs, DeployTarget, DevArgs, InitArgs, ProjectTemplate, RunArgs,
//     TestArgs,
// };
// pub use args_shell::{SelfArgs, SelfCommands, ShellArgs};
// pub use args_utility::{CleanArgs, CompletionShell, CompletionsArgs, InfoArgs, TreeArgs};
pub use commands::Commands;
// pub use hybrid_executor::HybridExecutor;  // Commented out for minimal build

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::ui::theme::{ColorMode, Theme};

/// DX - The Binary-First Development Experience
///
/// Build faster. Ship smaller. Zero compromise.
#[derive(Parser)]
#[command(
    name = "dx",
    author,
    version,
    about = "The Binary-First Development Experience",
    long_about = None,
    propagate_version = true,
    styles = styles::get_styles(),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Path to configuration file
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        // Determine color mode based on flags
        let color_mode = if self.no_color {
            ColorMode::Never
        } else {
            ColorMode::Auto
        };

        let _theme = Theme::with_color_mode(color_mode);

        // Handle no command case - run onboarding
        if self.command.is_none() {
            // Always run onboarding when no command is provided
            return crate::commands::onboard::run().await;
        }

        let _command = self.command.unwrap();

        // Minimal executor for onboarding
        eprintln!("Command not available in minimal build. Only onboarding is enabled.");
        Ok(())
    }
}
