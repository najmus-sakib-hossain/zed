//! CLI for dx-py-test-runner
//!
//! This is the main entry point for the dx-py command.

use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, Subcommand};
use colored::Colorize;
use dx_py_discovery::{ParametrizeExpander, TestScanner};
use dx_py_executor::{ExecutionSummary, ExecutorConfig, WorkStealingExecutor};

mod filter;
mod junit;
mod watch;

use filter::TestFilter;
use junit::JUnitReport;

/// High-performance Python test runner
#[derive(Parser)]
#[command(name = "dx-py")]
#[command(about = "A high-performance Python test runner built with Rust")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tests
    Test {
        /// Test pattern to filter (glob or regex)
        #[arg(default_value = "*")]
        pattern: String,

        /// Watch mode - re-run affected tests on file changes
        #[arg(short, long)]
        watch: bool,

        /// Update snapshots instead of failing on mismatch
        #[arg(long)]
        update_snapshots: bool,

        /// CI mode - output JUnit XML
        #[arg(long)]
        ci: bool,

        /// Output file for JUnit XML (default: test-results.xml)
        #[arg(long, default_value = "test-results.xml")]
        junit_output: PathBuf,

        /// Number of workers (default: number of CPUs)
        #[arg(short = 'j', long)]
        workers: Option<usize>,

        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Python executable path (default: python)
        #[arg(short, long, default_value = "python")]
        python: String,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Discover tests without running them
    Discover {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Test pattern to filter
        #[arg(default_value = "*")]
        pattern: String,
    },

    /// Show dependency graph for a file
    Graph {
        /// File to show dependencies for
        file: PathBuf,

        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Test {
            pattern,
            watch,
            update_snapshots,
            ci,
            junit_output,
            workers,
            root,
            python,
            verbose,
        } => {
            run_tests(
                &root,
                &pattern,
                watch,
                update_snapshots,
                ci,
                &junit_output,
                workers,
                &python,
                verbose,
            );
        }
        Commands::Discover { root, pattern } => {
            discover_tests(&root, &pattern);
        }
        Commands::Graph { file, root } => {
            show_graph(&root, &file);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_tests(
    root: &Path,
    pattern: &str,
    watch: bool,
    update_snapshots: bool,
    ci: bool,
    junit_output: &Path,
    workers: Option<usize>,
    python: &str,
    verbose: bool,
) {
    println!("{}", "dx-py test runner".cyan().bold());
    println!();

    let start = Instant::now();

    // Create test filter
    let filter = TestFilter::new(pattern);

    if verbose {
        println!("Root: {}", root.display());
        println!("Pattern: {}", pattern);
        println!("Workers: {}", workers.unwrap_or(num_cpus::get()));
        println!();
    }

    // Discover tests using tree-sitter
    let discovery_start = Instant::now();
    let mut scanner = match TestScanner::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", "Discovery error".red(), e);
            return;
        }
    };

    let mut all_tests = Vec::new();

    // Scan for Python test files
    if let Ok(entries) = walkdir(root) {
        for entry in entries {
            if entry.extension().is_some_and(|e| e == "py") {
                if let Ok(tests) = scanner.scan_file(&entry) {
                    for test in tests {
                        if filter.matches(&test.name) {
                            all_tests.push(test);
                        }
                    }
                }
            }
        }
    }

    // Expand parametrized tests
    let expander = ParametrizeExpander::new();
    let expanded_tests: Vec<_> = all_tests
        .iter()
        .flat_map(|test| {
            let expanded = expander.expand(test);
            expanded.into_iter().map(|exp| exp.to_test_case())
        })
        .collect();
    
    all_tests = expanded_tests;

    let discovery_time = discovery_start.elapsed();

    if verbose {
        println!(
            "Discovery: {:.2}ms ({} tests found)",
            discovery_time.as_secs_f64() * 1000.0,
            all_tests.len()
        );
    }

    if watch {
        // Build dependency graph
        let mut graph = dx_py_graph::DependencyGraph::new();

        // Register test files and their tests
        for test in &all_tests {
            graph.add_file(&test.file_path, true);
            graph.set_file_tests(&test.file_path, vec![test.id]);
        }

        // Run watch mode
        let executor_config = ExecutorConfig::default()
            .with_workers(workers.unwrap_or(num_cpus::get()))
            .with_python(python);

        match watch::run_watch_mode(root, &filter, all_tests, graph, executor_config, verbose) {
            Ok(result) => {
                println!("\nWatch mode summary:");
                println!("  Total runs: {}", result.total_runs);
                println!("  Total passed: {}", result.total_passed);
                println!("  Total failed: {}", result.total_failed);
            }
            Err(e) => {
                eprintln!("{}: {}", "Watch mode error".red(), e);
            }
        }
        return;
    }

    // Execute tests using work-stealing executor
    let execution_start = Instant::now();
    let config = ExecutorConfig::default()
        .with_workers(workers.unwrap_or(num_cpus::get()))
        .with_python(python);
    let executor = WorkStealingExecutor::new(config);

    if let Err(e) = executor.submit_all(all_tests.clone()) {
        eprintln!("{}: {}", "Execution error".red(), e);
        return;
    }

    let results = executor.execute();
    let execution_time = execution_start.elapsed();
    let total_time = start.elapsed();

    // Print individual test results
    for result in &results {
        let test = all_tests.iter().find(|t| t.id == result.test_id);
        let name = test.map(|t| t.full_name()).unwrap_or_else(|| "unknown".to_string());

        match &result.status {
            dx_py_core::TestStatus::Pass => {
                println!(
                    "  {} {} ({:.2}ms)",
                    "✓".green(),
                    name,
                    result.duration.as_secs_f64() * 1000.0
                );
            }
            dx_py_core::TestStatus::Fail => {
                println!(
                    "  {} {} ({:.2}ms)",
                    "✗".red(),
                    name,
                    result.duration.as_secs_f64() * 1000.0
                );
                if let Some(tb) = &result.traceback {
                    println!("    {}", tb.red());
                }
            }
            dx_py_core::TestStatus::Skip { reason } => {
                println!("  {} {} (skipped: {})", "○".yellow(), name, reason);
            }
            dx_py_core::TestStatus::Error { message } => {
                println!("  {} {} (error: {})", "!".red().bold(), name, message);
            }
        }
    }

    // Calculate summary
    let summary = ExecutionSummary::from_results(&results, executor.panics());

    println!();
    if verbose {
        println!("Discovery: {:.2}ms", discovery_time.as_secs_f64() * 1000.0);
        println!("Execution: {:.2}ms", execution_time.as_secs_f64() * 1000.0);
    }
    print_summary(summary.passed, summary.failed, summary.skipped, total_time);

    if ci {
        // Generate JUnit XML
        let mut report = JUnitReport::new("dx-py-tests");
        for result in &results {
            let test = all_tests.iter().find(|t| t.id == result.test_id);
            let name = test.map(|t| t.full_name()).unwrap_or_else(|| "unknown".to_string());
            report.add_result(&name, result.clone());
        }
        if let Err(e) = report.write_to_file(junit_output) {
            eprintln!("{}: {}", "Error writing JUnit XML".red(), e);
        } else if verbose {
            println!("JUnit XML written to: {}", junit_output.display());
        }
    }

    if update_snapshots {
        println!("{}", "Snapshots updated".green());
    }
}

/// Walk directory recursively and collect Python files
fn walkdir(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk_recursive(root, &mut files)?;
    Ok(files)
}

fn walk_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_file() {
        // If it's a file, check if it's a Python test file
        if dir.extension().is_some_and(|e| e == "py") {
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("test_") || name.ends_with("_test.py") {
                files.push(dir.to_path_buf());
            }
        }
    } else if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden directories and common non-test directories
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "__pycache__" && name != "node_modules" {
                    walk_recursive(&path, files)?;
                }
            } else if path.extension().is_some_and(|e| e == "py") {
                // Only include test files
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with("test_") || name.ends_with("_test.py") {
                    files.push(path);
                }
            }
        }
    }
    Ok(())
}

fn discover_tests(root: &Path, pattern: &str) {
    println!("{}", "Discovering tests...".cyan());
    println!("Root: {}", root.display());
    println!("Pattern: {}", pattern);
    println!();

    let start = Instant::now();
    let filter = TestFilter::new(pattern);

    // Discover tests using tree-sitter
    let mut scanner = match TestScanner::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", "Discovery error".red(), e);
            return;
        }
    };

    let mut all_tests = Vec::new();

    // Scan for Python test files
    if let Ok(entries) = walkdir(root) {
        for entry in entries {
            if let Ok(tests) = scanner.scan_file(&entry) {
                for test in tests {
                    if filter.matches(&test.name) {
                        all_tests.push((entry.clone(), test));
                    }
                }
            }
        }
    }

    // Expand parametrized tests
    let expander = ParametrizeExpander::new();
    let expanded_tests: Vec<_> = all_tests
        .iter()
        .flat_map(|(file, test)| {
            let expanded = expander.expand(test);
            expanded.into_iter().map(move |exp| (file.clone(), exp.to_test_case()))
        })
        .collect();
    
    all_tests = expanded_tests;

    let discovery_time = start.elapsed();

    // Print discovered tests
    let mut current_file: Option<PathBuf> = None;
    for (file, test) in &all_tests {
        if current_file.as_ref() != Some(file) {
            println!("\n{}", file.display().to_string().cyan());
            current_file = Some(file.clone());
        }
        println!("  {} (line {})", test.full_name().green(), test.line_number);
    }

    println!();
    println!("{}", "─".repeat(50));
    println!(
        "Discovered {} tests in {:.2}ms",
        all_tests.len().to_string().green(),
        discovery_time.as_secs_f64() * 1000.0
    );
    println!("{}", "─".repeat(50));
}

fn show_graph(root: &Path, file: &Path) {
    println!("{}", "Dependency graph".cyan());
    println!("Root: {}", root.display());
    println!("File: {}", file.display());
    println!();

    // Graph visualization requires dx-py-graph integration
    println!("{}", "Graph visualization requires dx-py-graph crate".yellow());
}

fn print_summary(passed: usize, failed: usize, skipped: usize, duration: std::time::Duration) {
    println!("{}", "─".repeat(50));

    if failed == 0 {
        println!(
            "{} {} passed, {} skipped in {:.2}s",
            "✓".green().bold(),
            passed.to_string().green(),
            skipped.to_string().yellow(),
            duration.as_secs_f64()
        );
    } else {
        println!(
            "{} {} passed, {} failed, {} skipped in {:.2}s",
            "✗".red().bold(),
            passed.to_string().green(),
            failed.to_string().red(),
            skipped.to_string().yellow(),
            duration.as_secs_f64()
        );
    }

    println!("{}", "─".repeat(50));
}
