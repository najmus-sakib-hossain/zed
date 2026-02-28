//! Shell and self-management command arguments

use clap::{Args, Subcommand, ValueEnum};

/// Arguments for the shell command
#[derive(Args)]
pub struct ShellArgs {
    #[command(subcommand)]
    pub command: ShellCommands,
}

#[derive(Subcommand)]
pub enum ShellCommands {
    /// Install shell integration
    Install {
        /// Shell type (auto-detected if not specified)
        #[arg(short, long, value_enum)]
        shell: Option<ShellType>,

        /// Force reinstall even if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall shell integration
    Uninstall {
        /// Shell type (auto-detected if not specified)
        #[arg(short, long, value_enum)]
        shell: Option<ShellType>,
    },

    /// Show current shell integration status
    Status,

    /// Print shell integration script (for manual installation)
    Print {
        /// Shell type
        #[arg(short, long, value_enum)]
        shell: ShellType,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Nushell,
}

/// Arguments for the self command
#[derive(Args)]
pub struct SelfArgs {
    #[command(subcommand)]
    pub command: SelfCommands,
}

#[derive(Subcommand)]
pub enum SelfCommands {
    /// Check for updates
    Update {
        /// Force update even if already on latest
        #[arg(short, long)]
        force: bool,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Show DX CLI information
    Info,

    /// Uninstall DX CLI
    Uninstall {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

/// Arguments for the config command
#[derive(Args)]
pub struct ConfigArgs {
    /// Reset configuration and run onboarding again
    #[arg(short, long)]
    pub reset: bool,

    /// Show current configuration
    #[arg(short, long)]
    pub show: bool,
}
