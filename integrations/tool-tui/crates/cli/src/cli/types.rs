//! CLI type definitions

use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// Arguments for the init command
#[derive(Args)]
pub struct InitArgs {
    /// Project name
    #[arg(index = 1)]
    pub name: Option<String>,

    /// Project template
    #[arg(short, long, value_enum)]
    pub template: Option<ProjectTemplate>,

    /// Skip git initialization
    #[arg(long)]
    pub no_git: bool,

    /// Skip dependency installation
    #[arg(long)]
    pub no_install: bool,
}

#[derive(ValueEnum, Clone, Default)]
pub enum ProjectTemplate {
    #[default]
    Default,
    Minimal,
    Full,
}

/// Arguments for the dev command
#[derive(Args)]
pub struct DevArgs {
    /// Port to run dev server on
    #[arg(short, long, default_value = "3000")]
    pub port: u16,

    /// Open browser automatically
    #[arg(short, long)]
    pub open: bool,

    /// Enable HTTPS
    #[arg(long)]
    pub https: bool,

    /// Host to bind to
    #[arg(long, default_value = "localhost")]
    pub host: String,
}

/// Arguments for the build command
#[derive(Args)]
pub struct BuildArgs {
    /// Build target
    #[arg(short, long, value_enum, default_value = "release")]
    pub target: BuildTarget,

    /// Output directory
    #[arg(short, long)]
    pub out_dir: Option<PathBuf>,

    /// Enable source maps
    #[arg(long)]
    pub sourcemap: bool,

    /// Minify output
    #[arg(long, default_value = "true")]
    pub minify: bool,
}

#[derive(ValueEnum, Clone, Default)]
pub enum BuildTarget {
    Dev,
    #[default]
    Release,
    Production,
}

/// Arguments for the run command
#[derive(Args)]
pub struct RunArgs {
    /// Script or command to run, followed by optional arguments
    #[arg(trailing_var_arg = true, num_args = 0..)]
    pub args: Vec<String>,
}

/// Arguments for the test command
#[derive(Args)]
pub struct TestArgs {
    /// Test pattern to match
    #[arg(index = 1)]
    pub pattern: Option<String>,

    /// Run tests in watch mode
    #[arg(short, long)]
    pub watch: bool,

    /// Run tests with coverage
    #[arg(long)]
    pub coverage: bool,

    /// Number of parallel test threads
    #[arg(short = 'j', long)]
    pub jobs: Option<usize>,
}

/// Arguments for the deploy command
#[derive(Args)]
pub struct DeployArgs {
    /// Deployment target
    #[arg(short, long, value_enum)]
    pub target: Option<DeployTarget>,

    /// Production deployment
    #[arg(long)]
    pub production: bool,

    /// Preview deployment
    #[arg(long)]
    pub preview: bool,
}

#[derive(ValueEnum, Clone)]
pub enum DeployTarget {
    Vercel,
    Netlify,
    Cloudflare,
    Aws,
}

/// Arguments for the shell command
#[derive(Args)]
pub struct ShellArgs {
    #[command(subcommand)]
    pub command: ShellCommands,
}

#[derive(clap::Subcommand)]
pub enum ShellCommands {
    /// Install shell integration
    Install {
        /// Shell type
        #[arg(value_enum)]
        shell: Option<ShellType>,
    },
    /// Uninstall shell integration
    Uninstall {
        /// Shell type
        #[arg(value_enum)]
        shell: Option<ShellType>,
    },
    /// Show shell integration status
    Status,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
}

/// Arguments for the self command
#[derive(Args)]
pub struct SelfArgs {
    #[command(subcommand)]
    pub command: SelfCommands,
}

#[derive(clap::Subcommand)]
pub enum SelfCommands {
    /// Check for updates
    Update {
        /// Force update even if already up to date
        #[arg(long)]
        force: bool,
    },
    /// Uninstall dx
    Uninstall {
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

/// Arguments for the info command
#[derive(Args)]
pub struct InfoArgs {
    /// Show detailed system information
    #[arg(short, long)]
    pub detailed: bool,

    /// Output format
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,
}

#[derive(ValueEnum, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Arguments for the clean command
#[derive(Args)]
pub struct CleanArgs {
    /// Clean all caches and artifacts
    #[arg(short, long)]
    pub all: bool,

    /// Clean build artifacts
    #[arg(long)]
    pub build: bool,

    /// Clean cache
    #[arg(long)]
    pub cache: bool,
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
}

/// Arguments for the tree command
#[derive(Args)]
pub struct TreeArgs {
    /// Directory to display (defaults to current directory)
    #[arg(index = 1)]
    pub path: Option<PathBuf>,

    /// Maximum depth to display
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Show hidden files
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Show file sizes
    #[arg(short, long)]
    pub size: bool,
}

/// Arguments for the animate command
#[derive(Args)]
pub struct AnimateArgs {
    #[command(subcommand)]
    pub command: AnimateCommand,
}

#[derive(clap::Subcommand)]
pub enum AnimateCommand {
    /// Show an animation
    Show {
        /// Animation type
        #[arg(value_enum)]
        animation: AnimationType,
    },
    /// Play a sound
    Sound {
        /// Sound type
        #[arg(value_enum)]
        sound: SoundType,
    },
}

#[derive(ValueEnum, Clone)]
pub enum AnimationType {
    Confetti,
    Fireworks,
    Matrix,
}

#[derive(ValueEnum, Clone)]
pub enum SoundType {
    Success,
    Error,
    Notification,
}
