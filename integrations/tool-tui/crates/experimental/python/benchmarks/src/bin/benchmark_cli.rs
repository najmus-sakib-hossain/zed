//! CLI for the comparative benchmarking framework
//!
//! Commands:
//! - run: Execute benchmark suites
//! - list: List available benchmark suites and benchmarks
//! - reproduce: Re-run benchmarks from stored configuration
//! - compare: Compare two benchmark runs

use clap::{Parser, Subcommand};
use dx_py_benchmarks::core::{BenchmarkConfig, BenchmarkFramework, OutputFormat};
use std::path::PathBuf;
use std::process::ExitCode;

/// Comparative benchmarking framework for DX-Py components
#[derive(Parser)]
#[command(name = "benchmark")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run benchmark suites
    Run {
        /// Benchmark suites to run (runtime, package, test_runner)
        #[arg(short, long, value_delimiter = ',')]
        suite: Option<Vec<String>>,

        /// Number of warmup iterations
        #[arg(short, long, default_value = "5")]
        warmup: u32,

        /// Number of measurement iterations
        #[arg(short = 'i', long, default_value = "30")]
        iterations: u32,

        /// Timeout in seconds
        #[arg(short, long, default_value = "300")]
        timeout: u64,

        /// Output directory for results
        #[arg(short, long, default_value = "benchmark_results")]
        output: PathBuf,

        /// Output format (markdown, json, both)
        #[arg(short = 'F', long, default_value = "both")]
        format: String,

        /// Random seed for reproducibility
        #[arg(long)]
        seed: Option<u64>,

        /// Filter benchmarks by name pattern
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// List available benchmark suites and benchmarks
    List {
        /// Show detailed information about each benchmark
        #[arg(short, long)]
        verbose: bool,
    },

    /// Reproduce a previous benchmark run
    Reproduce {
        /// ID of the benchmark run to reproduce
        run_id: String,

        /// Output directory for results
        #[arg(short, long, default_value = "benchmark_results")]
        output: PathBuf,
    },

    /// Compare two benchmark runs
    Compare {
        /// ID of the first (baseline) benchmark run
        baseline_id: String,

        /// ID of the second (subject) benchmark run
        subject_id: String,

        /// Output directory for results
        #[arg(short, long, default_value = "benchmark_results")]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            suite,
            warmup,
            iterations,
            timeout,
            output,
            format,
            seed,
            filter,
        } => run_benchmarks(suite, warmup, iterations, timeout, output, format, seed, filter),

        Commands::List { verbose } => list_benchmarks(verbose),

        Commands::Reproduce { run_id, output } => reproduce_benchmark(&run_id, output),

        Commands::Compare {
            baseline_id,
            subject_id,
            output,
        } => compare_benchmarks(&baseline_id, &subject_id, output),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_benchmarks(
    suite: Option<Vec<String>>,
    warmup: u32,
    iterations: u32,
    timeout: u64,
    output: PathBuf,
    format: String,
    seed: Option<u64>,
    filter: Option<String>,
) -> ExitCode {
    let output_format = match format.to_lowercase().as_str() {
        "markdown" | "md" => OutputFormat::Markdown,
        "json" => OutputFormat::Json,
        "both" => OutputFormat::Both,
        _ => {
            eprintln!("Invalid output format: {}. Use 'markdown', 'json', or 'both'", format);
            return ExitCode::FAILURE;
        }
    };

    let config = BenchmarkConfig {
        warmup_iterations: warmup,
        measurement_iterations: iterations,
        timeout_seconds: timeout,
        output_format,
        output_dir: output,
        seed,
        suites: suite.unwrap_or_default(),
        filter,
    };

    // Validate configuration
    if iterations < 30 {
        eprintln!(
            "Warning: {} measurement iterations is below the recommended minimum of 30. \
             Results may not be statistically valid.",
            iterations
        );
    }

    let mut framework = BenchmarkFramework::new(config);

    println!("Starting benchmark run...");
    println!("  Warmup iterations: {}", warmup);
    println!("  Measurement iterations: {}", iterations);
    println!("  Timeout: {} seconds", timeout);
    if let Some(s) = seed {
        println!("  Seed: {}", s);
    }
    println!();

    match framework.run_all() {
        Ok(results) => {
            println!("Benchmark run completed successfully!");
            println!();

            for result in &results {
                println!("Suite: {}", result.suite_name);
                println!("  Benchmarks run: {}", result.results.benchmarks.len());
                if let Some(id) = &result.stored_id {
                    println!("  Stored as: {}", id);
                }

                if let Some(comparison) = &result.comparison {
                    println!("  Comparisons:");
                    for comp in &comparison.comparisons {
                        let status = if comp.is_slower {
                            "⚠️  SLOWER"
                        } else if comp.speedup > 1.0 {
                            "✅ FASTER"
                        } else {
                            "➖ SAME"
                        };
                        println!("    {} - {:.2}x {}", comp.name, comp.speedup, status);
                    }
                }
                println!();
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Benchmark run failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn list_benchmarks(verbose: bool) -> ExitCode {
    println!("Available Benchmark Suites:");
    println!();

    for suite in BenchmarkFramework::available_suites() {
        println!("  {}", suite);

        if verbose {
            match suite {
                "runtime" => {
                    println!("    Micro-benchmarks:");
                    println!("      - int_arithmetic: Integer arithmetic operations");
                    println!("      - string_operations: String manipulation");
                    println!("      - list_operations: List operations");
                    println!("      - dict_operations: Dictionary operations");
                    println!("    Macro-benchmarks:");
                    println!("      - json_parsing: JSON parsing and serialization");
                    println!("      - file_io: File I/O operations");
                    println!("      - http_handling: HTTP request/response handling");
                    println!("    Startup/Memory:");
                    println!("      - cold_startup: Cold startup time");
                    println!("      - memory_usage: Memory usage");
                }
                "package" => {
                    println!("    Resolution:");
                    println!("      - resolution_small: Small project (5 deps)");
                    println!("      - resolution_medium: Medium project (20 deps)");
                    println!("      - resolution_large: Large project (100+ deps)");
                    println!("    Installation:");
                    println!("      - install_cold_cache: Cold cache installation");
                    println!("      - install_warm_cache: Warm cache installation");
                    println!("      - lock_generation: Lock file generation");
                    println!("      - venv_creation: Virtual environment creation");
                    println!("    Real-world:");
                    println!("      - real_world_flask: Flask project");
                    println!("      - real_world_django: Django project");
                    println!("      - real_world_requests: Requests project");
                    println!("      - real_world_numpy: NumPy project");
                }
                "test_runner" => {
                    println!("    Discovery:");
                    println!("      - discovery_small: 10 tests");
                    println!("      - discovery_medium: 100 tests");
                    println!("      - discovery_large: 1000 tests");
                    println!("    Execution:");
                    println!("      - execution_simple: Simple tests");
                    println!("      - execution_fixtures: Fixture-based tests");
                    println!("      - execution_parametrized: Parametrized tests");
                    println!("      - execution_async: Async tests");
                    println!("      - parallel_execution: Parallel test execution");
                }
                _ => {}
            }
            println!();
        }
    }

    ExitCode::SUCCESS
}

fn reproduce_benchmark(run_id: &str, output: PathBuf) -> ExitCode {
    let config = BenchmarkConfig {
        output_dir: output.clone(),
        ..Default::default()
    };

    let mut framework = BenchmarkFramework::new(config);

    println!("Reproducing benchmark run: {}", run_id);
    println!();

    match framework.reproduce(run_id) {
        Ok(result) => {
            println!("Benchmark reproduced successfully!");
            println!("  Suite: {}", result.suite_name);
            println!("  Benchmarks run: {}", result.results.benchmarks.len());
            if let Some(id) = &result.stored_id {
                println!("  Stored as: {}", id);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to reproduce benchmark: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn compare_benchmarks(baseline_id: &str, subject_id: &str, output: PathBuf) -> ExitCode {
    let config = BenchmarkConfig {
        output_dir: output.clone(),
        ..Default::default()
    };

    let framework = BenchmarkFramework::new(config);

    println!("Comparing benchmark runs:");
    println!("  Baseline: {}", baseline_id);
    println!("  Subject: {}", subject_id);
    println!();

    // Load both results
    let baseline = match framework.result_store.load(baseline_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load baseline result: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let subject = match framework.result_store.load(subject_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load subject result: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Create comparison
    let comparison = framework.reporter.create_comparison(&baseline.results, &subject.results);

    // Print comparison
    println!("Comparison Results:");
    println!();
    println!(
        "{:<30} {:>12} {:>12} {:>10} {:>10}",
        "Benchmark", "Baseline", "Subject", "Speedup", "Status"
    );
    println!("{}", "-".repeat(76));

    for comp in &comparison.comparisons {
        let status = if comp.is_slower {
            "⚠️  SLOWER"
        } else if comp.speedup > 1.0 {
            "✅ FASTER"
        } else {
            "➖ SAME"
        };

        println!(
            "{:<30} {:>10.3}ms {:>10.3}ms {:>9.2}x {}",
            comp.name, comp.baseline_mean_ms, comp.subject_mean_ms, comp.speedup, status
        );
    }

    println!();

    // Generate and save comparison report
    let markdown = framework.reporter.generate_markdown(&subject.results, &comparison);

    let report_path = output.join(format!("comparison_{}_{}.md", baseline_id, subject_id));
    if let Err(e) = std::fs::write(&report_path, &markdown) {
        eprintln!("Warning: Failed to write comparison report: {}", e);
    } else {
        println!("Comparison report saved to: {}", report_path.display());
    }

    ExitCode::SUCCESS
}
