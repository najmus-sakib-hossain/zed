//! dx-pkg-cli: Command-Line Interface
//!
//! Commands:
//! - dx install [packages...]
//! - dx add <package>
//! - dx remove <package>

use anyhow::Result;
use clap::{Parser, Subcommand};

mod background;
mod commands;

#[derive(Parser)]
#[command(name = "dx")]
#[command(about = "DX Package Manager - Fast npm-compatible package manager")]
#[command(long_about = "DX Package Manager

A blazingly fast npm-compatible package manager with O(1) cached installs.

FEATURES:
  • O(1) cached installs via pre-built layouts
  • Content-addressable storage
  • Hardlink/reflink extraction
  • Workspace support
  • Private registry authentication
  • Lifecycle scripts

EXAMPLES:
  dx install                Install all dependencies from package.json
  dx add lodash             Add lodash to dependencies
  dx add -D typescript      Add typescript to devDependencies
  dx add -g typescript      Install typescript globally
  dx remove lodash          Remove lodash from dependencies
  dx run build              Run the 'build' script from package.json
  dx list                   List installed packages
  dx list -g                List globally installed packages
  dx outdated               Show packages with newer versions available
  dx audit                  Check for security vulnerabilities

For more information, visit: https://github.com/dx-tools/dx-javascript")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output for debugging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Install dependencies from package.json
    ///
    /// Installs all dependencies listed in package.json. On first run, builds
    /// a cache that makes subsequent installs nearly instant (O(1) via symlink).
    ///
    /// Examples:
    ///   dx install              Install all dependencies
    ///   dx install --frozen     Install with frozen lockfile (for CI)
    ///   dx install --production Install only production dependencies
    ///   dx install --v3         Use v3 Binary Dawn mode with all optimizations
    Install {
        /// Specific packages to install (optional)
        packages: Vec<String>,

        /// Use frozen lockfile - fail if lockfile is out of date (for CI)
        #[arg(long)]
        frozen: bool,

        /// Install production dependencies only (skip devDependencies)
        #[arg(long)]
        production: bool,

        /// Use npm proxy mode for maximum compatibility
        #[arg(long, default_value = "true")]
        npm_mode: bool,

        /// Use v3 Binary Dawn mode with all optimizations
        #[arg(long)]
        v3: bool,
    },
    /// Add a package to dependencies
    ///
    /// Resolves the package version, adds it to package.json, and installs it.
    ///
    /// Examples:
    ///   dx add lodash           Add latest version of lodash
    ///   dx add lodash@4.17.21   Add specific version
    ///   dx add lodash@^4.17.0   Add with version constraint
    ///   dx add -D typescript    Add to devDependencies
    ///   dx add -g typescript    Install globally
    Add {
        /// Package names with optional version (e.g., react, lodash@^4.17.0)
        packages: Vec<String>,

        /// Add to devDependencies instead of dependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Add to global packages
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Remove a package from dependencies
    ///
    /// Removes the package from package.json and node_modules.
    Remove {
        /// Package name to remove
        package: String,
    },
    /// Update packages to latest compatible versions
    ///
    /// Checks for newer versions that satisfy the version constraints
    /// in package.json and updates them.
    Update {
        /// Specific package to update (updates all if not specified)
        package: Option<String>,
    },
    /// List installed packages
    ///
    /// Shows all installed packages with their versions.
    ///
    /// Examples:
    ///   dx list              List direct dependencies
    ///   dx list --depth 1    Show one level of transitive dependencies
    ///   dx list -g           List globally installed packages
    List {
        /// Depth of dependency tree to show (0 = direct deps only)
        #[arg(short, long, default_value = "0")]
        depth: usize,

        /// List global packages
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Show packages with newer versions available
    ///
    /// Compares installed versions against the latest available versions.
    Outdated,
    /// Check for security vulnerabilities
    ///
    /// Scans dependencies against known vulnerability databases.
    Audit,
    /// Run a script from package.json
    ///
    /// Executes the specified script with pre/post hooks.
    ///
    /// Examples:
    ///   dx run build                    Run the 'build' script
    ///   dx run test -- --coverage       Run test with extra arguments
    ///   dx run build --filter "@org/*"  Run in matching workspace packages
    Run {
        /// Script name to run
        script: String,

        /// Arguments to pass to the script
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Filter packages in workspace (e.g., "@scope/*")
        #[arg(long)]
        filter: Option<String>,
    },
    /// Execute a command with node_modules/.bin in PATH
    ///
    /// Runs a command with local binaries available.
    Exec {
        /// Command to execute
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Download and execute a package without installing
    ///
    /// Similar to npx - downloads and runs a package temporarily.
    ///
    /// Examples:
    ///   dx dlx create-react-app my-app   Create a new React app
    ///   dx dlx typescript@latest --help  Run latest TypeScript compiler
    Dlx {
        /// Package to run (e.g., typescript, create-react-app@latest)
        package: String,

        /// Arguments to pass to the package
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run performance benchmark
    Benchmark {
        /// Number of benchmark runs
        #[arg(short, long, default_value = "3")]
        runs: usize,

        /// Use v3 mode for benchmarking
        #[arg(long)]
        v3: bool,
    },
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            packages,
            frozen,
            production,
            npm_mode,
            v3,
        } => {
            if v3 {
                // Use new v3.0 Binary Dawn mode
                commands::install_v3::install_v3(frozen, production).await?;
            } else if npm_mode && packages.is_empty() {
                // Use v1.6 npm proxy mode for full installs
                commands::install_npm::install(frozen, production).await?;
            } else {
                // Use old mode for specific packages or when npm_mode=false
                commands::install::run(packages, cli.verbose).await?;
            }
        }
        Commands::Benchmark { runs, v3 } => {
            if v3 {
                commands::install_v3::benchmark_v3(runs).await?;
            } else {
                println!("⚠️  Benchmark only available for v3 mode");
                println!("   Use: dx benchmark --v3 --runs 3");
            }
        }
        Commands::Add {
            packages,
            dev,
            global,
        } => {
            if global {
                commands::global::install_global(&packages, cli.verbose).await?;
            } else {
                // Handle multiple packages
                for package in &packages {
                    commands::add::run(package, dev, cli.verbose).await?;
                }
            }
        }
        Commands::Remove { package } => {
            commands::remove::run(&package, cli.verbose).await?;
        }
        Commands::Update { package } => {
            commands::update::run(package.as_deref(), cli.verbose).await?;
        }
        Commands::List { depth, global } => {
            if global {
                commands::global::list_global(cli.verbose).await?;
            } else {
                commands::list::run(depth, cli.verbose).await?;
            }
        }
        Commands::Outdated => {
            commands::outdated::run(cli.verbose).await?;
        }
        Commands::Audit => {
            commands::audit::run(cli.verbose).await?;
        }
        Commands::Run {
            script,
            args,
            filter,
        } => {
            commands::run::run(&script, &args, filter.as_deref(), cli.verbose).await?;
        }
        Commands::Exec { command, args } => {
            commands::exec::run(&command, &args, cli.verbose).await?;
        }
        Commands::Dlx { package, args } => {
            commands::dlx::run(&package, &args, cli.verbose).await?;
        }
        Commands::Version => {
            println!("dx v3.0.0");
            println!("Fast npm-compatible package manager");
        }
    }

    Ok(())
}
