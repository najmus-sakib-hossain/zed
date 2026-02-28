//! Watch Mode - Re-run tests on file changes
//!
//! Uses notify crate for efficient file system watching.

#![allow(dead_code)]

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// File watcher for test files
pub struct TestWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    rx: Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
    watched_paths: Vec<PathBuf>,
}

impl TestWatcher {
    /// Create a new test watcher for the given project root
    pub fn new(project_root: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        // Create debounced watcher (100ms debounce to batch rapid changes)
        let mut debouncer = new_debouncer(Duration::from_millis(100), tx)?;

        // Watch the project root recursively
        debouncer.watcher().watch(project_root, RecursiveMode::Recursive)?;

        Ok(Self {
            _debouncer: debouncer,
            rx,
            watched_paths: vec![project_root.to_path_buf()],
        })
    }

    /// Wait for file changes and return affected paths
    pub fn wait_for_changes(&self) -> Vec<PathBuf> {
        match self.rx.recv() {
            Ok(Ok(events)) => events
                .into_iter()
                .filter(|e| matches!(e.kind, DebouncedEventKind::Any))
                .filter(|e| Self::is_relevant_file(&e.path))
                .map(|e| e.path)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Check if a file change is relevant (test file or source file)
    fn is_relevant_file(path: &Path) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Test files
        if name.ends_with(".test.ts")
            || name.ends_with(".test.js")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.js")
            || name.ends_with("_test.ts")
            || name.ends_with("_test.js")
        {
            return true;
        }

        // Source files (may affect tests)
        if name.ends_with(".ts")
            || name.ends_with(".js")
            || name.ends_with(".tsx")
            || name.ends_with(".jsx")
        {
            return true;
        }

        // Config files
        if name == "package.json" || name == "tsconfig.json" || name == "dx.toml" {
            return true;
        }

        false
    }

    /// Get tests affected by changed files
    pub fn get_affected_tests(changed_files: &[PathBuf], all_tests: &[String]) -> Vec<String> {
        let mut affected = Vec::new();

        for changed in changed_files {
            let changed_str = changed.to_string_lossy();

            // If a test file changed, include it directly
            if Self::is_test_file(changed) {
                // Find tests from this file
                for test in all_tests {
                    if test.contains(&*changed_str) || Self::test_matches_file(test, changed) {
                        affected.push(test.clone());
                    }
                }
            } else {
                // Source file changed - find tests that might import it
                // For now, re-run all tests (could be optimized with dependency graph)
                return all_tests.to_vec();
            }
        }

        affected.sort();
        affected.dedup();
        affected
    }

    fn is_test_file(path: &Path) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name.ends_with(".test.ts")
            || name.ends_with(".test.js")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.js")
            || name.ends_with("_test.ts")
            || name.ends_with("_test.js")
    }

    fn test_matches_file(test_name: &str, file_path: &Path) -> bool {
        // Extract base name from file path
        if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
            // Remove .test or .spec suffix
            let base = stem
                .strip_suffix(".test")
                .or_else(|| stem.strip_suffix(".spec"))
                .or_else(|| stem.strip_suffix("_test"))
                .unwrap_or(stem);

            test_name.to_lowercase().contains(&base.to_lowercase())
        } else {
            false
        }
    }
}

/// Watch mode runner
pub struct WatchRunner {
    project_root: PathBuf,
    pattern: Option<String>,
    parallel: bool,
    verbose: bool,
}

impl WatchRunner {
    pub fn new(
        project_root: PathBuf,
        pattern: Option<String>,
        parallel: bool,
        verbose: bool,
    ) -> Self {
        Self {
            project_root,
            pattern,
            parallel,
            verbose,
        }
    }

    /// Run watch mode loop
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        use colored::*;

        println!("\n{}", "👀 Watch mode enabled".bold().cyan());
        println!("{}", "━".repeat(50).dimmed());
        println!("  Watching for file changes...");
        println!("  Press Ctrl+C to exit\n");

        let watcher = TestWatcher::new(&self.project_root)?;

        // Initial run
        self.run_tests()?;

        // Watch loop
        loop {
            let changed = watcher.wait_for_changes();

            if !changed.is_empty() {
                println!("\n{}", "📝 Files changed:".yellow());
                for path in &changed {
                    println!("  • {}", path.display());
                }
                println!();

                // Invalidate cache and re-run
                dx_test_cache::WarmState::global().invalidate();

                if let Err(e) = self.run_tests() {
                    eprintln!("{}: {}", "Error".red().bold(), e);
                }
            }
        }
    }

    fn run_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        use colored::*;
        use std::time::Instant;

        let total_start = Instant::now();

        println!("{}", "🧪 Running tests...".bold());

        // Get layout
        let warm = dx_test_cache::WarmState::global();
        let layout = warm.get_layout(&self.project_root)?;

        // Filter tests
        let all_tests: Vec<_> = layout.tests().to_vec();
        let mut tests = all_tests.clone();

        if let Some(ref pattern) = self.pattern {
            tests.retain(|t| layout.get_test_name(t).contains(pattern));
        }

        // Execute
        let executor = dx_test_executor::TestExecutor::new(self.parallel);
        let results = executor.execute(&tests, &layout);

        // Count results
        let passed = results
            .iter()
            .filter(|r| matches!(r.status, dx_test_core::TestStatus::Passed))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.status, dx_test_core::TestStatus::Failed))
            .count();

        let total_time = total_start.elapsed();

        // Summary
        if failed == 0 {
            println!(
                "  {} {} tests passed in {:.2}ms",
                "✓".green().bold(),
                passed,
                total_time.as_secs_f64() * 1000.0
            );
        } else {
            println!(
                "  {} {} passed, {} {} failed in {:.2}ms",
                "✓".green(),
                passed,
                "✗".red().bold(),
                failed,
                total_time.as_secs_f64() * 1000.0
            );

            // Show failures
            for (test, result) in tests.iter().zip(&results) {
                if matches!(result.status, dx_test_core::TestStatus::Failed) {
                    let name = layout.get_test_name(test);
                    println!("    {} {}", "✗".red(), name);
                    if let Some(ref msg) = result.error_message {
                        println!("      {}", msg.red());
                    }
                }
            }
        }

        println!();
        Ok(())
    }
}
