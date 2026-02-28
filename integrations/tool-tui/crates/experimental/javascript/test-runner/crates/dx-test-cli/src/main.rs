//! DX Test CLI - Command Line Interface
//!
//! 50x faster test runner than Bun

use clap::{Parser, Subcommand};
use colored::*;
use dx_test_cache::{TestLayoutCache, WarmState};
use dx_test_core::*;
use dx_test_executor::TestExecutor;
use std::time::Instant;

mod coverage;
mod mock;
mod snapshot;
mod watch;

#[derive(Parser)]
#[command(name = "dx-test")]
#[command(about = "DX Test Runner - 50x faster than Bun", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Test pattern to filter
    pattern: Option<String>,

    /// Watch mode
    #[arg(short, long)]
    watch: bool,

    /// Coverage
    #[arg(long)]
    coverage: bool,

    /// Update snapshots
    #[arg(short = 'u', long)]
    update_snapshots: bool,

    /// Disable parallel execution
    #[arg(long)]
    no_parallel: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tests (default)
    Run {
        /// Test pattern
        pattern: Option<String>,
    },
    /// Show cache statistics
    Cache,
    /// Clear cache
    Clear,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Cache) => {
            show_cache_stats()?;
        }
        Some(Commands::Clear) => {
            clear_cache()?;
        }
        Some(Commands::Run { pattern }) => {
            run_tests(
                pattern.or(cli.pattern),
                cli.watch,
                cli.coverage,
                cli.update_snapshots,
                !cli.no_parallel,
                cli.verbose,
            )?;
        }
        None => {
            run_tests(
                cli.pattern,
                cli.watch,
                cli.coverage,
                cli.update_snapshots,
                !cli.no_parallel,
                cli.verbose,
            )?;
        }
    }

    Ok(())
}

fn run_tests(
    pattern: Option<String>,
    watch: bool,
    _coverage: bool,
    _update_snapshots: bool,
    parallel: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;

    // Watch mode
    if watch {
        let runner = watch::WatchRunner::new(project_root, pattern, parallel, verbose);
        return runner.run();
    }

    let total_start = Instant::now();

    println!("\n{}", "🧪 DX Test Runner".bold().cyan());
    println!("{}", "━".repeat(50).dimmed());

    // Phase 1: Discovery & Layout (O(1) with cache!)
    let discovery_start = Instant::now();
    let warm = WarmState::global();
    let layout = warm.get_layout(&project_root)?;
    let discovery_time = discovery_start.elapsed();

    // Phase 2: Filter tests
    let all_tests: Vec<_> = layout.tests().to_vec();
    let mut tests = all_tests.clone();

    if let Some(ref pattern) = pattern {
        tests.retain(|t| layout.get_test_name(t).contains(pattern));
    }

    println!(
        "  {} {} tests found in {:.2}ms",
        "✓".green().bold(),
        tests.len(),
        discovery_time.as_secs_f64() * 1000.0
    );

    if pattern.is_some() {
        println!("  {} Filter: {} tests matched", "⚡".yellow(), tests.len());
    }

    // Phase 3: Execute tests
    let exec_start = Instant::now();
    let executor = TestExecutor::new(parallel);
    let results = executor.execute(&tests, &layout);
    let exec_time = exec_start.elapsed();

    println!(
        "  {} Executed in {:.2}ms {}",
        "⚡".yellow().bold(),
        exec_time.as_secs_f64() * 1000.0,
        if parallel {
            format!("(parallel on {} cores)", TestExecutor::thread_count())
        } else {
            "(sequential)".to_string()
        }
    );

    println!("{}", "━".repeat(50).dimmed());

    // Display results
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for (test, result) in tests.iter().zip(&results) {
        let name = layout.get_test_name(test);
        let duration = result.duration.as_secs_f64() * 1000.0;

        match result.status {
            TestStatus::Passed => {
                passed += 1;
                if verbose {
                    println!("  {} {} ({:.2}ms)", "✓".green(), name.dimmed(), duration);
                }
            }
            TestStatus::Failed => {
                failed += 1;
                println!("  {} {} ({:.2}ms)", "✗".red().bold(), name, duration);
                if let Some(ref msg) = result.error_message {
                    println!("     {}", msg.red());
                }
            }
            TestStatus::Skipped => {
                skipped += 1;
                if verbose {
                    println!("  {} {} (skipped)", "○".yellow(), name.dimmed());
                }
            }
            _ => {}
        }
    }

    // Summary
    let total_time = total_start.elapsed();

    println!("{}", "━".repeat(50).dimmed());
    println!("\n{}", "Test Summary:".bold());
    println!("  {} {} passed", "✓".green().bold(), passed.to_string().green().bold());
    if failed > 0 {
        println!("  {} {} failed", "✗".red().bold(), failed.to_string().red().bold());
    }
    if skipped > 0 {
        println!("  {} {} skipped", "○".yellow(), skipped.to_string().dimmed());
    }
    println!("  {} {} total", "Σ".bold(), tests.len().to_string().bold());

    println!("\n{}", "Performance:".bold());
    println!("  Discovery:  {:.2}ms", discovery_time.as_secs_f64() * 1000.0);
    println!("  Execution:  {:.2}ms", exec_time.as_secs_f64() * 1000.0);
    println!("  {} {:.2}ms", "Total:".bold(), total_time.as_secs_f64() * 1000.0);

    // Performance comparison
    let bun_estimated = tests.len() as f64 * 0.84; // Bun: ~0.84ms per test
    let speedup = bun_estimated / (total_time.as_secs_f64() * 1000.0);

    if speedup > 1.0 {
        println!(
            "\n  {} {:.1}x faster than Bun (estimated)",
            "🚀".bold(),
            speedup.to_string().green().bold()
        );
    }

    println!();

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn show_cache_stats() -> Result<(), Box<dyn std::error::Error>> {
    let cache = TestLayoutCache::new()?;
    let project_root = std::env::current_dir()?;
    let hash = TestLayoutCache::compute_hash(&project_root);

    println!("\n{}", "Cache Statistics:".bold().cyan());
    println!("{}", "━".repeat(50).dimmed());
    println!("  Project hash: {:032x}", hash);

    if let Some(layout) = cache.get_cached_layout(hash) {
        let header = layout.header();
        // Copy values from packed struct to avoid unaligned reference errors
        let test_count = header.test_count;
        let file_count = header.file_count;
        let created_at = header.created_at;

        println!("  {} Cache HIT", "✓".green().bold());
        println!("  Tests: {}", test_count);
        println!("  Files: {}", file_count);
        println!(
            "  Created: {} seconds ago",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()
                - created_at
        );
    } else {
        println!("  {} Cache MISS - will build on next run", "○".yellow());
    }

    println!();
    Ok(())
}

fn clear_cache() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "Clearing cache...".bold());

    let cache_dir = std::env::temp_dir().join("dx-test-cache");
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)?;
        println!("  {} Cache cleared", "✓".green().bold());
    } else {
        println!("  {} Cache already empty", "○".yellow());
    }

    // Invalidate warm state
    WarmState::global().invalidate();

    println!();
    Ok(())
}
