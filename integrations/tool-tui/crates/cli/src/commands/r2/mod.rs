//! Cloudflare R2 Storage Integration
//!
//! # Configuration
//! - Access Key: de8218aea33b1e1c6195107290c78448
//! - Endpoint: https://2410e99bde64ed52a9d6c2395a440b0b.r2.cloudflarestorage.com
//! - Bucket: dx-forge-production
//!
//! Uses S3-compatible API for all operations

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::ui::theme::Theme;

pub mod cache;
pub mod client;
pub mod sync;

/// R2 storage commands
#[derive(Args, Debug)]
pub struct R2Args {
    #[command(subcommand)]
    pub command: R2Commands,
}

#[derive(Subcommand, Debug)]
pub enum R2Commands {
    /// Sync files to R2
    Push(PushArgs),

    /// Download files from R2
    Pull(PullArgs),

    /// List objects in bucket
    List(ListArgs),

    /// Delete objects from R2
    Delete(DeleteArgs),

    /// Show sync status
    Status(StatusArgs),

    /// Configure R2 credentials
    Config(ConfigArgs),
}

#[derive(Args, Debug)]
pub struct PushArgs {
    /// Files or directories to push
    pub paths: Vec<PathBuf>,

    /// Remote prefix/path
    #[arg(long)]
    pub prefix: Option<String>,

    /// Include patterns
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude patterns
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Delete remote files not in source
    #[arg(long)]
    pub delete: bool,

    /// Dry run without making changes
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Force overwrite existing files
    #[arg(long, short)]
    pub force: bool,

    /// Compress files before upload
    #[arg(long)]
    pub compress: bool,

    /// Show progress
    #[arg(long)]
    pub progress: bool,
}

#[derive(Args, Debug)]
pub struct PullArgs {
    /// Remote prefix/path to download
    pub remote: String,

    /// Local destination
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Include patterns
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude patterns  
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Delete local files not in remote
    #[arg(long)]
    pub delete: bool,

    /// Dry run without making changes
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Force overwrite existing files
    #[arg(long, short)]
    pub force: bool,

    /// Show progress
    #[arg(long)]
    pub progress: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Prefix to list
    pub prefix: Option<String>,

    /// Show detailed info
    #[arg(long, short)]
    pub long: bool,

    /// Recursive listing
    #[arg(long, short)]
    pub recursive: bool,

    /// Output format
    #[arg(long, default_value = "human")]
    pub format: OutputFormat,

    /// Maximum results
    #[arg(long)]
    pub max: Option<usize>,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Objects to delete (keys or prefixes)
    pub keys: Vec<String>,

    /// Delete all objects matching prefix
    #[arg(long)]
    pub recursive: bool,

    /// Dry run without making changes
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Skip confirmation
    #[arg(long, short)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Show sync status for path
    pub path: Option<PathBuf>,

    /// Show detailed info
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Set access key
    #[arg(long)]
    pub access_key: Option<String>,

    /// Set secret key
    #[arg(long)]
    pub secret_key: Option<String>,

    /// Set endpoint URL
    #[arg(long)]
    pub endpoint: Option<String>,

    /// Set bucket name
    #[arg(long)]
    pub bucket: Option<String>,

    /// Show current config
    #[arg(long)]
    pub show: bool,

    /// Test connection
    #[arg(long)]
    pub test: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Sr,
    Llm,
}

/// R2 storage configuration
#[derive(Debug, Clone)]
pub struct R2Config {
    pub access_key: String,
    pub secret_key: String,
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
}

impl Default for R2Config {
    fn default() -> Self {
        Self::production()
    }
}

impl R2Config {
    /// Production R2 configuration
    pub fn production() -> Self {
        Self {
            access_key: "de8218aea33b1e1c6195107290c78448".to_string(),
            secret_key: "900629e1597e4a92f2e09fb9c6b36cde5ee9ff05aecba2d195b080c03d3e2ac6"
                .to_string(),
            endpoint: "https://2410e99bde64ed52a9d6c2395a440b0b.r2.cloudflarestorage.com"
                .to_string(),
            bucket: "dx-forge-production".to_string(),
            region: "auto".to_string(),
        }
    }

    /// Load from environment
    pub fn from_env() -> Option<Self> {
        Some(Self {
            access_key: std::env::var("DX_R2_ACCESS_KEY").ok()?,
            secret_key: std::env::var("DX_R2_SECRET_KEY").ok()?,
            endpoint: std::env::var("DX_R2_ENDPOINT").unwrap_or_else(|_| {
                "https://2410e99bde64ed52a9d6c2395a440b0b.r2.cloudflarestorage.com".to_string()
            }),
            bucket: std::env::var("DX_R2_BUCKET")
                .unwrap_or_else(|_| "dx-forge-production".to_string()),
            region: "auto".to_string(),
        })
    }
}

/// Run R2 commands
pub async fn run(args: R2Args, theme: &Theme) -> Result<()> {
    match args.command {
        R2Commands::Push(args) => run_push(args, theme).await,
        R2Commands::Pull(args) => run_pull(args, theme).await,
        R2Commands::List(args) => run_list(args, theme).await,
        R2Commands::Delete(args) => run_delete(args, theme).await,
        R2Commands::Status(args) => run_status(args, theme).await,
        R2Commands::Config(args) => run_config(args, theme).await,
    }
}

async fn run_push(args: PushArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let config = R2Config::from_env().unwrap_or_default();
    let client = client::R2Client::new(config)?;

    if args.dry_run {
        println!("{} Dry run mode - no changes will be made", "●".yellow());
    }

    let mut total_files = 0;
    let mut total_bytes = 0u64;

    for path in &args.paths {
        if path.is_file() {
            let key = args
                .prefix
                .as_ref()
                .map(|p| format!("{}/{}", p, path.file_name().unwrap().to_string_lossy()))
                .unwrap_or_else(|| path.file_name().unwrap().to_string_lossy().to_string());

            if !args.dry_run {
                let size = path.metadata()?.len();
                client.upload_file(path, &key, args.compress).await?;
                total_bytes += size;
            }

            println!("{} {}", "↑".green(), path.display());
            total_files += 1;
        } else if path.is_dir() {
            let files = collect_files(path, &args.include, &args.exclude)?;

            for file in files {
                let relative = file.strip_prefix(path).unwrap();
                let key = args
                    .prefix
                    .as_ref()
                    .map(|p| format!("{}/{}", p, relative.display()))
                    .unwrap_or_else(|| relative.display().to_string());

                if !args.dry_run {
                    let size = file.metadata()?.len();
                    client.upload_file(&file, &key, args.compress).await?;
                    total_bytes += size;
                }

                println!("{} {}", "↑".green(), file.display());
                total_files += 1;
            }
        }
    }

    println!();
    println!(
        "{} Pushed {} files ({} bytes)",
        "✓".green(),
        total_files,
        format_bytes(total_bytes)
    );

    Ok(())
}

async fn run_pull(args: PullArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let config = R2Config::from_env().unwrap_or_default();
    let client = client::R2Client::new(config)?;

    if args.dry_run {
        println!("{} Dry run mode - no changes will be made", "●".yellow());
    }

    let output = args.output.unwrap_or_else(|| std::env::current_dir().unwrap());

    let objects = client.list_objects(&args.remote, true).await?;
    let mut total_files = 0;
    let mut total_bytes = 0u64;

    for obj in objects {
        // Check include/exclude patterns
        if !should_include(&obj.key, &args.include, &args.exclude) {
            continue;
        }

        let relative_key =
            obj.key.strip_prefix(&args.remote).unwrap_or(&obj.key).trim_start_matches('/');
        let local_path = output.join(relative_key);

        if !args.dry_run {
            if let Some(parent) = local_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            client.download_file(&obj.key, &local_path).await?;
            total_bytes += obj.size;
        }

        println!("{} {}", "↓".green(), local_path.display());
        total_files += 1;
    }

    println!();
    println!(
        "{} Pulled {} files ({} bytes)",
        "✓".green(),
        total_files,
        format_bytes(total_bytes)
    );

    Ok(())
}

async fn run_list(args: ListArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let config = R2Config::from_env().unwrap_or_default();
    let client = client::R2Client::new(config)?;

    let prefix = args.prefix.unwrap_or_default();
    let objects = client.list_objects(&prefix, args.recursive).await?;

    match args.format {
        OutputFormat::Human => {
            println!("{}", "╔════════════════════════════════════════════╗".cyan());
            println!("{}", "║            R2 Object Listing               ║".cyan());
            println!("{}", "╚════════════════════════════════════════════╝".cyan());
            println!();

            for (i, obj) in objects.iter().enumerate() {
                if let Some(max) = args.max {
                    if i >= max {
                        println!("... and {} more", objects.len() - max);
                        break;
                    }
                }

                if args.long {
                    println!(
                        "{:>10}  {}  {}",
                        format_bytes(obj.size),
                        format_time(&obj.last_modified),
                        obj.key
                    );
                } else {
                    println!("{}", obj.key);
                }
            }

            println!();
            println!("Total: {} objects", objects.len());
        }
        OutputFormat::Json => {
            let output: Vec<_> = objects
                .iter()
                .map(|obj| {
                    serde_json::json!({
                        "key": obj.key,
                        "size": obj.size,
                        "last_modified": format_time(&obj.last_modified),
                        "etag": obj.etag,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Sr => {
            println!("# R2 objects");
            for obj in &objects {
                println!("{}|{}|{}", obj.key, obj.size, obj.etag);
            }
        }
        OutputFormat::Llm => {
            println!("R2_OBJECTS prefix:{}", prefix);
            for obj in &objects {
                println!("{}|{}", obj.key, format_bytes(obj.size));
            }
        }
    }

    Ok(())
}

async fn run_delete(args: DeleteArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let config = R2Config::from_env().unwrap_or_default();
    let client = client::R2Client::new(config)?;

    if args.dry_run {
        println!("{} Dry run mode - no changes will be made", "●".yellow());
    }

    for key in &args.keys {
        if args.recursive {
            let objects = client.list_objects(key, true).await?;

            if !args.force && !args.dry_run {
                println!(
                    "{} Delete {} objects under '{}'? (y/N)",
                    "?".yellow(),
                    objects.len(),
                    key
                );
                // TODO: Read confirmation
            }

            for obj in objects {
                if !args.dry_run {
                    client.delete_object(&obj.key).await?;
                }
                println!("{} {}", "✗".red(), obj.key);
            }
        } else {
            if !args.dry_run {
                client.delete_object(key).await?;
            }
            println!("{} {}", "✗".red(), key);
        }
    }

    Ok(())
}

async fn run_status(args: StatusArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let config = R2Config::from_env().unwrap_or_default();
    let client = client::R2Client::new(config.clone())?;

    println!("{}", "╔════════════════════════════════════════════╗".cyan());
    println!("{}", "║             R2 Sync Status                 ║".cyan());
    println!("{}", "╚════════════════════════════════════════════╝".cyan());
    println!();

    // Test connection
    match client.head_bucket().await {
        Ok(_) => println!("{} Connected to bucket: {}", "✓".green(), config.bucket),
        Err(e) => println!("{} Connection failed: {}", "✗".red(), e),
    }

    if let Some(path) = args.path {
        // Show sync status for specific path
        println!();
        println!("Path: {}", path.display());
        // TODO: Show detailed sync status
    }

    Ok(())
}

async fn run_config(args: ConfigArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    if args.show {
        let config = R2Config::from_env().unwrap_or_default();
        println!("{}", "R2 Configuration:".bold());
        println!("  Access Key: {}...", &config.access_key[..16]);
        println!("  Endpoint:   {}", config.endpoint);
        println!("  Bucket:     {}", config.bucket);
        println!("  Region:     {}", config.region);
        return Ok(());
    }

    if args.test {
        let config = R2Config::from_env().unwrap_or_default();
        let client = client::R2Client::new(config)?;

        print!("Testing connection... ");
        match client.head_bucket().await {
            Ok(_) => println!("{}", "OK".green()),
            Err(e) => println!("{}: {}", "FAILED".red(), e),
        }
        return Ok(());
    }

    // Update config
    // TODO: Implement config storage
    if args.access_key.is_some()
        || args.secret_key.is_some()
        || args.endpoint.is_some()
        || args.bucket.is_some()
    {
        println!("{} Set environment variables to configure R2:", "ℹ".blue());
        println!("  DX_R2_ACCESS_KEY=<access_key>");
        println!("  DX_R2_SECRET_KEY=<secret_key>");
        println!("  DX_R2_ENDPOINT=<endpoint>");
        println!("  DX_R2_BUCKET=<bucket>");
    }

    Ok(())
}

// Helper functions

fn collect_files(dir: &PathBuf, include: &[String], exclude: &[String]) -> Result<Vec<PathBuf>> {
    let mut files = vec![];

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();
            if should_include(&path.to_string_lossy(), include, exclude) {
                files.push(path);
            }
        }
    }

    Ok(files)
}

fn should_include(path: &str, include: &[String], exclude: &[String]) -> bool {
    // Check exclude first
    for pattern in exclude {
        if glob::Pattern::new(pattern).map_or(false, |p| p.matches(path)) {
            return false;
        }
    }

    // If no include patterns, include everything
    if include.is_empty() {
        return true;
    }

    // Check include patterns
    for pattern in include {
        if glob::Pattern::new(pattern).map_or(false, |p| p.matches(path)) {
            return true;
        }
    }

    false
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

fn format_time(time: &SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = (*time).into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
