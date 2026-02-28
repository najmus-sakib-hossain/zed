//! dx-py CLI - Ultra-fast Python package manager
//!
//! A high-performance Python package manager that is 5-50x faster than uv.

use clap::{Parser, Subcommand};
use clap_complete::Shell;

mod commands;

use commands::{
    add, build, cache, completions, init, install, lock, pip, publish, python, remove, run, sync,
    tool,
};

/// Ultra-fast Python package manager
#[derive(Parser)]
#[command(name = "dx-py")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Run a module as a script (like python -m)
    #[arg(short = 'm', global = true)]
    module: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Python project
    Init {
        /// Python version to use
        #[arg(long)]
        python: Option<String>,
        /// Project name
        #[arg(long)]
        name: Option<String>,
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Add dependencies to the project
    Add {
        /// Packages to add (e.g., "requests>=2.0")
        packages: Vec<String>,
        /// Add as development dependency
        #[arg(short = 'D', long)]
        dev: bool,
        /// Add as optional dependency group
        #[arg(long)]
        optional: Option<String>,
    },

    /// Remove dependencies from the project
    Remove {
        /// Packages to remove
        packages: Vec<String>,
        /// Remove from development dependencies
        #[arg(short = 'D', long)]
        dev: bool,
    },

    /// Generate lock file from dependencies
    Lock {
        /// Update all packages to latest versions
        #[arg(long)]
        upgrade: bool,
    },

    /// Install packages from lock file
    Sync {
        /// Install development dependencies
        #[arg(long)]
        dev: bool,
        /// Install optional dependency groups
        #[arg(long)]
        extras: Vec<String>,
        /// Show verbose output with cache statistics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Lock and sync (convenience command)
    Install {
        /// Install development dependencies
        #[arg(long)]
        dev: bool,
        /// Install optional dependency groups
        #[arg(long)]
        extras: Vec<String>,
        /// Show verbose output with cache statistics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run a command in the virtual environment
    Run {
        /// Command and arguments to run
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Python version management
    Python {
        #[command(subcommand)]
        command: PythonCommands,
    },

    /// Global tool management (pipx replacement)
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },

    /// Build package for distribution
    Build {
        /// Output directory
        #[arg(short, long, default_value = "dist")]
        output: String,
        /// Build wheel only
        #[arg(long)]
        wheel: bool,
        /// Build sdist only
        #[arg(long)]
        sdist: bool,
    },

    /// Publish package to PyPI
    Publish {
        /// Repository URL
        #[arg(long)]
        repository: Option<String>,
        /// API token
        #[arg(long)]
        token: Option<String>,
        /// Distribution files to upload
        #[arg(default_value = "dist/*")]
        files: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Cache management
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// pip-compatible commands
    #[command(subcommand_negates_reqs = true)]
    Pip {
        #[command(subcommand)]
        command: Option<PipCommands>,
    },

    /// Execute a Python script (shorthand for run python)
    #[command(name = "exec", hide = true)]
    Exec {
        /// Script file to execute
        script: String,
        /// Arguments to pass to the script
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PythonCommands {
    /// Install a Python version
    Install {
        /// Python version to install (e.g., "3.12.0")
        version: String,
    },
    /// List installed Python versions
    List,
    /// Pin Python version for the current project
    Pin {
        /// Python version to pin
        version: String,
    },
    /// Show which Python would be used
    Which,
}

#[derive(Subcommand)]
enum ToolCommands {
    /// Install a tool globally
    Install {
        /// Tool package name
        name: String,
        /// Python version to use
        #[arg(long)]
        python: Option<String>,
    },
    /// Run a tool ephemerally (without installing)
    Run {
        /// Tool package name
        name: String,
        /// Arguments to pass to the tool
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// List installed tools
    List,
    /// Uninstall a tool
    Uninstall {
        /// Tool name to uninstall
        name: String,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Clean cached data
    Clean {
        /// Clear layout cache
        #[arg(long)]
        layouts: bool,
        /// Clear package store
        #[arg(long)]
        store: bool,
        /// Clear all caches
        #[arg(long)]
        all: bool,
    },
    /// Show cache statistics
    Stats,
}

#[derive(Subcommand)]
enum PipCommands {
    /// Install packages
    Install {
        /// Packages to install
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Uninstall packages
    Uninstall {
        /// Packages to uninstall
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Output installed packages in requirements format
    Freeze,
    /// List installed packages
    List {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Show information about installed packages
    Show {
        /// Packages to show
        packages: Vec<String>,
    },
    /// Download packages
    Download {
        /// Packages to download
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build wheels from requirements
    Wheel {
        /// Packages to build
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Verify installed packages have compatible dependencies
    Check,
}

/// Run a module as a script (like python -m)
fn run_module(module: &str, args: &[String]) -> dx_py_core::Result<()> {
    let mut cmd_args = vec!["python".to_string(), "-m".to_string(), module.to_string()];
    cmd_args.extend(args.iter().cloned());
    run::run(&cmd_args)
}

/// Execute a Python script
fn run_script(script: &str, args: &[String]) -> dx_py_core::Result<()> {
    let mut cmd_args = vec!["python".to_string(), script.to_string()];
    cmd_args.extend(args.iter().cloned());
    run::run(&cmd_args)
}

fn main() {
    // Handle special cases for -m and script execution
    let args: Vec<String> = std::env::args().collect();

    // Check for -m flag (module execution)
    if args.len() > 2 && args[1] == "-m" {
        let module = &args[2];
        let module_args: Vec<String> = args[3..].to_vec();
        let result = run_module(module, &module_args);
        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Check if first argument is a .py file (script execution)
    if args.len() > 1 && args[1].ends_with(".py") && !args[1].starts_with('-') {
        let script = &args[1];
        let script_args: Vec<String> = args[2..].to_vec();
        let result = run_script(script, &script_args);
        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Init { python, name, path }) => {
            init::run(&path, name.as_deref(), python.as_deref())
        }
        Some(Commands::Add {
            packages,
            dev,
            optional,
        }) => add::run(&packages, dev, optional.as_deref()),
        Some(Commands::Remove { packages, dev }) => remove::run(&packages, dev),
        Some(Commands::Lock { upgrade }) => lock::run(upgrade),
        Some(Commands::Sync {
            dev,
            extras,
            verbose,
        }) => sync::run(dev, &extras, verbose),
        Some(Commands::Install {
            dev,
            extras,
            verbose,
        }) => install::run(dev, &extras, verbose),
        Some(Commands::Run { command }) => run::run(&command),
        Some(Commands::Python { command }) => match command {
            PythonCommands::Install { version } => python::install(&version),
            PythonCommands::List => python::list(),
            PythonCommands::Pin { version } => python::pin(&version),
            PythonCommands::Which => python::which(),
        },
        Some(Commands::Tool { command }) => match command {
            ToolCommands::Install { name, python } => tool::install(&name, python.as_deref()),
            ToolCommands::Run { name, args } => tool::run(&name, &args),
            ToolCommands::List => tool::list(),
            ToolCommands::Uninstall { name } => tool::uninstall(&name),
        },
        Some(Commands::Build {
            output,
            wheel,
            sdist,
        }) => build::run(&output, wheel, sdist),
        Some(Commands::Publish {
            repository,
            token,
            files,
        }) => publish::run(repository.as_deref(), token.as_deref(), &files),
        Some(Commands::Completions { shell }) => completions::run(shell),
        Some(Commands::Cache { command }) => match command {
            CacheCommands::Clean {
                layouts,
                store,
                all,
            } => cache::clean(layouts, store, all),
            CacheCommands::Stats => cache::stats(),
        },
        Some(Commands::Pip { command }) => match command {
            Some(PipCommands::Install { args }) => {
                let mut full_args = vec!["install".to_string()];
                full_args.extend(args);
                pip::run(&full_args)
            }
            Some(PipCommands::Uninstall { args }) => {
                let mut full_args = vec!["uninstall".to_string()];
                full_args.extend(args);
                pip::run(&full_args)
            }
            Some(PipCommands::Freeze) => pip::run(&["freeze".to_string()]),
            Some(PipCommands::List { args }) => {
                let mut full_args = vec!["list".to_string()];
                full_args.extend(args);
                pip::run(&full_args)
            }
            Some(PipCommands::Show { packages }) => {
                let mut full_args = vec!["show".to_string()];
                full_args.extend(packages);
                pip::run(&full_args)
            }
            Some(PipCommands::Download { args }) => {
                let mut full_args = vec!["download".to_string()];
                full_args.extend(args);
                pip::run(&full_args)
            }
            Some(PipCommands::Wheel { args }) => {
                let mut full_args = vec!["wheel".to_string()];
                full_args.extend(args);
                pip::run(&full_args)
            }
            Some(PipCommands::Check) => pip::run(&["check".to_string()]),
            None => pip::run(&[]),
        },
        Some(Commands::Exec { script, args }) => run_script(&script, &args),
        None => {
            // No command provided, show help
            println!("dx-py - Ultra-fast Python package manager");
            println!();
            println!("Usage: dx-py <COMMAND>");
            println!();
            println!("Commands:");
            println!("  init         Initialize a new Python project");
            println!("  add          Add dependencies to the project");
            println!("  remove       Remove dependencies from the project");
            println!("  lock         Generate lock file from dependencies");
            println!("  sync         Install packages from lock file");
            println!("  install      Lock and sync (convenience command)");
            println!("  run          Run a command in the virtual environment");
            println!("  python       Python version management");
            println!("  tool         Global tool management (pipx replacement)");
            println!("  build        Build package for distribution");
            println!("  publish      Publish package to PyPI");
            println!("  pip          pip-compatible commands");
            println!("  completions  Generate shell completions");
            println!("  cache        Cache management");
            println!();
            println!("Options:");
            println!("  -m <MODULE>  Run a module as a script (like python -m)");
            println!("  <SCRIPT.py>  Execute a Python script");
            println!("  -h, --help   Print help");
            println!("  -V, --version Print version");
            println!();
            println!("Examples:");
            println!("  dx-py init                    # Initialize a new project");
            println!("  dx-py add requests            # Add a dependency");
            println!("  dx-py pip install requests    # pip-compatible install");
            println!("  dx-py -m pytest               # Run pytest module");
            println!("  dx-py script.py               # Execute a Python script");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
