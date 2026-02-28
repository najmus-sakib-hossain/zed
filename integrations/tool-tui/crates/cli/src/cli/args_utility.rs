//! Utility command arguments

use std::path::PathBuf;

use clap::{Args, ValueEnum};

/// Arguments for the info command
#[derive(Args)]
pub struct InfoArgs {
    /// Show detailed system information
    #[arg(short, long)]
    pub system: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(ValueEnum, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Yaml,
}

/// Arguments for the clean command
#[derive(Args)]
pub struct CleanArgs {
    /// Clean all caches and artifacts
    #[arg(short, long)]
    pub all: bool,

    /// Clean only build artifacts
    #[arg(long)]
    pub build: bool,

    /// Clean only caches
    #[arg(long)]
    pub cache: bool,

    /// Dry run (show what would be deleted)
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for the completions command
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

#[derive(ValueEnum, Clone)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Elvish,
}

/// Arguments for the tree command
#[derive(Args)]
pub struct TreeArgs {
    /// Directory to display (defaults to current directory)
    #[arg(index = 1)]
    pub path: Option<PathBuf>,

    /// Maximum depth to traverse
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Show hidden files
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Show code statistics (lines of code, comments)
    #[arg(long)]
    pub stats: bool,

    /// Sort by size (largest first)
    #[arg(short, long)]
    pub size: bool,
}
