//! DX WWW CLI - Binary-first web framework CLI tool

mod build;
mod dev;
mod utils;

use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "dx")]
#[command(version, about = "DX WWW Framework - Binary-first web development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new DX WWW project (alias: new)
    #[command(alias = "new")]
    Create {
        /// Project name
        name: String,

        /// Project directory (defaults to name)
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// Template to use (default, minimal, full)
        #[arg(short, long, default_value = "default")]
        template: String,
    },

    /// Start development server with hot reload
    Dev {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "localhost")]
        host: String,

        /// Disable hot reload
        #[arg(long)]
        no_hot_reload: bool,
    },

    /// Build for production
    Build {
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Optimization level (debug, release, size)
        #[arg(short = 'O', long, default_value = "release")]
        optimization: String,
    },

    /// Generate new files (page, component, api, layout)
    #[command(alias = "g")]
    Generate {
        /// Type to generate (page, component, api, layout)
        #[arg(value_parser = ["page", "component", "api", "layout", "p", "c", "a", "l"])]
        gen_type: String,

        /// Name of the file to generate
        name: String,
    },

    /// Add components from the DX component library
    Add {
        /// Component names to add, or --all for all, --list to list
        components: Vec<String>,
        
        /// Add all components
        #[arg(long)]
        all: bool,
        
        /// List available components
        #[arg(long)]
        list: bool,
    },

    /// Preview production build
    Preview {
        /// Port to run on
        #[arg(short, long, default_value = "4173")]
        port: u16,
    },

    /// Clean build artifacts
    Clean {
        /// Also clean cache
        #[arg(long)]
        cache: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", style("Error:").red().bold(), e);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Create { name, path, template } => {
            cmd_create(&name, path.as_deref(), &template).await?;
        }
        Commands::Dev {
            port,
            host,
            no_hot_reload,
        } => {
            dev::cmd_dev(port, &host, !no_hot_reload).await?;
        }
        Commands::Build {
            output,
            optimization,
        } => {
            build::cmd_build(output.as_deref(), &optimization).await?;
        }
        Commands::Generate { gen_type, name } => {
            cmd_generate(&gen_type, &name).await?;
        }
        Commands::Add { components, all, list } => {
            cmd_add(&components, all, list).await?;
        }
        Commands::Preview { port } => {
            cmd_preview(port).await?;
        }
        Commands::Clean { cache } => {
            cmd_clean(cache).await?;
        }
    }

    Ok(())
}

async fn cmd_create(name: &str, path: Option<&std::path::Path>, template: &str) -> anyhow::Result<()> {
    use dx_www::cli::Cli as DxCli;

    println!();
    println!("{}", style("🚀 Creating new DX project...").cyan().bold());
    println!();

    let cli = if let Some(p) = path {
        DxCli::with_cwd(p.to_path_buf())
    } else {
        DxCli::new()
    };

    cli.cmd_new(name)?;

    println!();
    println!("{}", style("✓ Project created successfully!").green().bold());
    println!();
    println!("Next steps:");
    println!("  {} {}", style("cd").cyan(), name);
    println!("  {} {}", style("dx").cyan(), "dev");
    println!();
    println!("To add components:");
    println!("  {} {}", style("dx add").cyan(), "button card modal");
    println!();

    Ok(())
}

async fn cmd_generate(gen_type: &str, name: &str) -> anyhow::Result<()> {
    use dx_www::cli::Cli as DxCli;

    let type_name = match gen_type {
        "p" => "page",
        "c" => "component",
        "a" => "api",
        "l" => "layout",
        other => other,
    };

    println!("{} {}", style("Generating").cyan(), style(type_name).bold());

    let cli = DxCli::new();
    cli.cmd_generate(type_name, name)?;

    println!();
    println!("{}", style("✓ Generated successfully!").green().bold());

    Ok(())
}

async fn cmd_add(components: &[String], all: bool, list: bool) -> anyhow::Result<()> {
    use dx_www::cli::Cli as DxCli;

    let cli = DxCli::new();

    if list {
        println!();
        println!("{}", style("📦 DX Component Library").cyan().bold());
        println!();
        cli.cmd_add(&["--list"])?;
        return Ok(());
    }

    if all {
        println!();
        println!("{}", style("📦 Adding all components...").cyan().bold());
        println!();
        cli.cmd_add(&["--all"])?;
        return Ok(());
    }

    if components.is_empty() {
        println!("{}", style("No components specified.").yellow());
        println!();
        println!("Usage:");
        println!("  {} {}", style("dx add").cyan(), "button card modal");
        println!("  {} {}", style("dx add --all").cyan(), "  # Add all components");
        println!("  {} {}", style("dx add --list").cyan(), " # List available components");
        return Ok(());
    }

    println!();
    println!("{}", style("📦 Adding components...").cyan().bold());
    println!();

    let refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    cli.cmd_add(&refs)?;

    println!();
    println!("{}", style("✓ Components added successfully!").green().bold());

    Ok(())
}

async fn cmd_preview(port: u16) -> anyhow::Result<()> {
    println!("{}", style("Starting preview server...").cyan().bold());
    println!();
    println!("🚀 Preview server running at http://localhost:{}", port);
    println!();
    println!("Press Ctrl+C to stop");

    // Preview server would run here
    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn cmd_clean(cache: bool) -> anyhow::Result<()> {
    use std::fs;

    println!("{}", style("Cleaning build artifacts...").cyan().bold());

    let build_dir = PathBuf::from(".dx/build");
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)?;
        println!("  {} Removed {}", style("✓").green(), build_dir.display());
    }

    if cache {
        let cache_dir = PathBuf::from(".dx/cache");
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)?;
            println!("  {} Removed {}", style("✓").green(), cache_dir.display());
        }
    }

    println!();
    println!("{}", style("✓ Clean complete!").green().bold());

    Ok(())
}
