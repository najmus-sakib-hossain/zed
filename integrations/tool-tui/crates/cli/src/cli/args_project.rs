//! Project command arguments

use std::path::PathBuf;

use clap::{Args, ValueEnum};

/// Arguments for the init command
#[derive(Args)]
pub struct InitArgs {
    /// Project name
    #[arg(index = 1)]
    pub name: Option<String>,

    /// Project template to use
    #[arg(short, long, value_enum, default_value = "default")]
    pub template: ProjectTemplate,

    /// Skip interactive prompts
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Initialize in current directory
    #[arg(long)]
    pub here: bool,
}

#[derive(ValueEnum, Clone, Default)]
pub enum ProjectTemplate {
    #[default]
    Default,
    Minimal,
    Full,
    Api,
    Web,
    Cli,
}

/// Arguments for the dev command
#[derive(Args)]
pub struct DevArgs {
    /// Port to run dev server on
    #[arg(short, long, default_value = "3000")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "localhost")]
    pub host: String,

    /// Open browser automatically
    #[arg(short, long)]
    pub open: bool,

    /// Enable HTTPS
    #[arg(long)]
    pub https: bool,

    /// Clear cache before starting
    #[arg(long)]
    pub clear: bool,
}

/// Arguments for the build command
#[derive(Args)]
pub struct BuildArgs {
    /// Build target
    #[arg(short, long, value_enum, default_value = "release")]
    pub target: BuildTarget,

    /// Output directory
    #[arg(short, long, default_value = "dist")]
    pub output: PathBuf,

    /// Enable source maps
    #[arg(long)]
    pub sourcemap: bool,

    /// Skip minification
    #[arg(long)]
    pub no_minify: bool,

    /// Analyze bundle size
    #[arg(long)]
    pub analyze: bool,
}

#[derive(ValueEnum, Clone, Default)]
pub enum BuildTarget {
    Dev,
    #[default]
    Release,
    Web,
    Node,
    Cloudflare,
    Vercel,
    Netlify,
}

/// Arguments for the run command
#[derive(Args)]
pub struct RunArgs {
    /// Script or command to run, followed by optional arguments
    #[arg(trailing_var_arg = true, num_args = 0..)]
    pub script_and_args: Vec<String>,

    /// Watch for changes and re-run
    #[arg(short, long)]
    pub watch: bool,
}

/// Arguments for the test command
#[derive(Args)]
pub struct TestArgs {
    /// Test pattern to match
    #[arg(index = 1)]
    pub pattern: Option<String>,

    /// Watch for changes and re-run
    #[arg(short, long)]
    pub watch: bool,

    /// Run tests in parallel
    #[arg(long)]
    pub parallel: bool,

    /// Generate coverage report
    #[arg(long)]
    pub coverage: bool,

    /// Update snapshots
    #[arg(short, long)]
    pub update: bool,
}

/// Arguments for the deploy command
#[derive(Args)]
pub struct DeployArgs {
    /// Deployment target
    #[arg(short, long, value_enum)]
    pub target: Option<DeployTarget>,

    /// Skip build step
    #[arg(long)]
    pub no_build: bool,

    /// Preview deployment (don't promote to production)
    #[arg(long)]
    pub preview: bool,

    /// Force deployment even with warnings
    #[arg(short, long)]
    pub force: bool,
}

#[derive(ValueEnum, Clone)]
pub enum DeployTarget {
    Vercel,
    Netlify,
    Cloudflare,
    Aws,
    Gcp,
    Azure,
}
