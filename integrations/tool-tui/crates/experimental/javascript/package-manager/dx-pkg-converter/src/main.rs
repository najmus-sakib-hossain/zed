//! DX Package Converter
//!
//! Converts npm .tgz packages to .dxp binary format

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

mod converter;
mod downloader;
mod format;

use converter::PackageConverter;
use downloader::NpmDownloader;

#[derive(Parser)]
#[command(name = "dx-convert")]
#[command(about = "Convert npm packages to DX binary format (.dxp)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a local .tgz file to .dxp
    File {
        /// Input .tgz file
        input: PathBuf,

        /// Output .dxp file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Download and convert a package from npm
    Download {
        /// Package name (e.g., react, lodash)
        package: String,

        /// Version (default: latest)
        #[arg(short, long)]
        version: Option<String>,

        /// Output directory
        #[arg(short, long, default_value = ".dx-registry")]
        output: PathBuf,
    },

    /// Batch convert multiple packages
    Batch {
        /// File containing package names (one per line)
        packages: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = ".dx-registry")]
        output: PathBuf,

        /// Number of concurrent downloads
        #[arg(short, long, default_value = "5")]
        concurrency: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::File { input, output } => {
            convert_file(input, output).await?;
        }
        Commands::Download {
            package,
            version,
            output,
        } => {
            download_and_convert(&package, version.as_deref(), &output).await?;
        }
        Commands::Batch {
            packages,
            output,
            concurrency,
        } => {
            batch_convert(&packages, &output, concurrency).await?;
        }
    }

    Ok(())
}

/// Convert a local .tgz file
async fn convert_file(input: PathBuf, output: Option<PathBuf>) -> Result<()> {
    println!("{}", "📦 Converting package...".cyan().bold());
    println!("   Input:  {}", input.display());

    let converter = PackageConverter::new();
    let output_path = converter.convert_file(&input, output.as_ref()).await?;

    println!("{}", "✅ Conversion complete!".green().bold());
    println!("   Output: {}", output_path.display());

    Ok(())
}

/// Download from npm and convert
async fn download_and_convert(name: &str, version: Option<&str>, output_dir: &Path) -> Result<()> {
    println!("{}", "🌐 Downloading from npm...".cyan().bold());
    println!("   Package: {}", name);

    let downloader = NpmDownloader::new();
    let version = version.unwrap_or("latest");

    // Download .tgz
    let tgz_data = downloader.download(name, version).await?;

    println!("{}", "📦 Converting to .dxp...".cyan().bold());

    // Convert to .dxp
    let converter = PackageConverter::new();
    let output_path = converter.convert_bytes(name, version, &tgz_data, output_dir).await?;

    println!("{}", "✅ Conversion complete!".green().bold());
    println!("   Output: {}", output_path.display());

    Ok(())
}

/// Batch convert multiple packages
async fn batch_convert(packages_file: &Path, output_dir: &Path, concurrency: usize) -> Result<()> {
    let packages_list =
        std::fs::read_to_string(packages_file).context("Failed to read packages file")?;

    let packages: Vec<&str> = packages_list.lines().filter(|l| !l.is_empty()).collect();

    println!("{}", format!("📦 Converting {} packages...", packages.len()).cyan().bold());

    let pb = ProgressBar::new(packages.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let downloader = NpmDownloader::new();
    let converter = PackageConverter::new();
    let output_dir = output_dir.to_path_buf();

    // Process in batches
    use futures::stream::{self, StreamExt};

    let results: Vec<Result<String>> = stream::iter(packages)
        .map(|package| {
            let downloader = downloader.clone();
            let converter = converter.clone();
            let output_dir = output_dir.clone();
            let pb = pb.clone();

            async move {
                pb.set_message(format!("Processing {}", package));

                // Download
                let tgz_data = downloader.download(package, "latest").await?;

                // Convert
                let _output_path =
                    converter.convert_bytes(package, "latest", &tgz_data, &output_dir).await?;

                pb.inc(1);

                Ok(package.to_string())
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    pb.finish_with_message("Done!");

    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let failed_count = results.len() - success_count;

    println!();
    println!("{}", format!("✅ Converted: {}", success_count).green().bold());
    if failed_count > 0 {
        println!("{}", format!("❌ Failed: {}", failed_count).red().bold());
    }

    Ok(())
}
