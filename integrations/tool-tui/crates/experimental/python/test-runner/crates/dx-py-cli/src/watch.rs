//! Watch mode for file system notifications
//!
//! Re-runs affected tests when files change.
//!
//! This module implements:
//! - File system monitoring using the notify crate
//! - Debouncing of rapid file changes
//! - Integration with the dependency graph for affected test detection
//! - Graceful shutdown handling

// Allow dead code as this module contains public API that may not be used yet
#![allow(dead_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::filter::TestFilter;
use dx_py_core::{TestCase, TestId};
use dx_py_graph::DependencyGraph;

/// Watch mode configuration
pub struct WatchConfig {
    /// Root directory to watch
    pub root: PathBuf,
    /// File extensions to watch
    pub extensions: Vec<String>,
    /// Debounce duration - wait this long after last change before triggering
    pub debounce: Duration,
    /// Minimum time between test runs
    pub min_interval: Duration,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            extensions: vec!["py".to_string()],
            debounce: Duration::from_millis(100),
            min_interval: Duration::from_millis(500),
        }
    }
}

impl WatchConfig {
    /// Create a new watch config with the given root
    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    /// Set the debounce duration
    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    /// Set the minimum interval between test runs
    pub fn with_min_interval(mut self, interval: Duration) -> Self {
        self.min_interval = interval;
        self
    }

    /// Add file extensions to watch
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }
}

/// Represents a batch of file changes
#[derive(Debug, Clone)]
pub struct FileChangeBatch {
    /// Changed files
    pub files: Vec<PathBuf>,
    /// When the first change was detected
    pub first_change: Instant,
    /// When the last change was detected
    pub last_change: Instant,
}

impl FileChangeBatch {
    /// Create a new empty batch
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            files: Vec::new(),
            first_change: now,
            last_change: now,
        }
    }

    /// Add a file to the batch
    pub fn add(&mut self, path: PathBuf) {
        if !self.files.contains(&path) {
            self.files.push(path);
        }
        self.last_change = Instant::now();
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get the number of files in the batch
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if enough time has passed since the last change (debounce complete)
    pub fn is_debounce_complete(&self, debounce: Duration) -> bool {
        self.last_change.elapsed() >= debounce
    }
}

impl Default for FileChangeBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// File watcher for watch mode with debouncing
pub struct FileWatcher {
    /// The underlying watcher
    _watcher: RecommendedWatcher,
    /// Receiver for file events
    receiver: Receiver<Result<Event, notify::Error>>,
    /// Configuration
    config: WatchConfig,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Current batch of changes being collected
    current_batch: FileChangeBatch,
    /// Last time tests were run
    last_run: Option<Instant>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatchConfig) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(config.debounce),
        )?;

        watcher.watch(&config.root, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
            current_batch: FileChangeBatch::new(),
            last_run: None,
        })
    }

    /// Get a handle to the shutdown flag
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Get the next batch of changed files (non-blocking)
    pub fn get_changed_files(&mut self) -> Vec<PathBuf> {
        self.collect_pending_events();

        if self.current_batch.is_empty() {
            return Vec::new();
        }

        // Check if debounce period has passed
        if !self.current_batch.is_debounce_complete(self.config.debounce) {
            return Vec::new();
        }

        // Check minimum interval since last run
        if let Some(last_run) = self.last_run {
            if last_run.elapsed() < self.config.min_interval {
                return Vec::new();
            }
        }

        // Return the batch and reset
        let batch = std::mem::take(&mut self.current_batch);
        self.last_run = Some(Instant::now());
        batch.files
    }

    /// Wait for the next file change with debouncing
    pub fn wait_for_change(&mut self) -> Vec<PathBuf> {
        loop {
            if self.is_shutdown() {
                return Vec::new();
            }

            // Try to receive events with a timeout
            match self.receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(event)) => {
                    self.process_event(event);
                }
                Ok(Err(_)) => {
                    // Watcher error, continue
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Check if we have a complete batch
                    if !self.current_batch.is_empty()
                        && self.current_batch.is_debounce_complete(self.config.debounce)
                    {
                        // Check minimum interval
                        if let Some(last_run) = self.last_run {
                            if last_run.elapsed() < self.config.min_interval {
                                continue;
                            }
                        }

                        let batch = std::mem::take(&mut self.current_batch);
                        self.last_run = Some(Instant::now());
                        return batch.files;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Vec::new();
                }
            }
        }
    }

    /// Collect all pending events without blocking
    fn collect_pending_events(&mut self) {
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(event)) => {
                    self.process_event(event);
                }
                Ok(Err(_)) => {
                    // Watcher error, continue
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    /// Process a file system event
    fn process_event(&mut self, event: Event) {
        // Only process modify, create, and remove events
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                for path in event.paths {
                    if self.should_watch(&path) {
                        // Initialize batch if empty
                        if self.current_batch.is_empty() {
                            self.current_batch = FileChangeBatch::new();
                        }
                        self.current_batch.add(path);
                    }
                }
            }
            _ => {}
        }
    }

    /// Check if a file should be watched
    fn should_watch(&self, path: &Path) -> bool {
        // Skip hidden files and directories
        if path.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| s.starts_with('.') || s == "__pycache__" || s == "node_modules")
                .unwrap_or(false)
        }) {
            return false;
        }

        // Check extension
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            return self.config.extensions.iter().any(|e| e == &ext_str);
        }
        false
    }

    /// Get the root directory being watched
    pub fn root(&self) -> &Path {
        &self.config.root
    }

    /// Get the debounce duration
    pub fn debounce(&self) -> Duration {
        self.config.debounce
    }
}

/// Calculator for determining which tests are affected by file changes
pub struct AffectedTestCalculator {
    /// The dependency graph
    graph: DependencyGraph,
    /// All known tests indexed by file
    tests_by_file: std::collections::HashMap<PathBuf, Vec<TestCase>>,
}

impl AffectedTestCalculator {
    /// Create a new calculator with the given dependency graph
    pub fn new(graph: DependencyGraph) -> Self {
        Self {
            graph,
            tests_by_file: std::collections::HashMap::new(),
        }
    }

    /// Register tests for a file
    pub fn register_tests(&mut self, file: &Path, tests: Vec<TestCase>) {
        let test_ids: Vec<TestId> = tests.iter().map(|t| t.id).collect();
        self.graph.set_file_tests(file, test_ids);
        self.tests_by_file.insert(file.to_owned(), tests);
    }

    /// Get all tests affected by changes to the given files
    ///
    /// This includes:
    /// - Tests in the changed files themselves (if they are test files)
    /// - Tests in files that import the changed files (transitively)
    pub fn get_affected_tests(&self, changed_files: &[PathBuf]) -> Vec<TestCase> {
        let mut affected_test_ids = HashSet::new();
        let mut affected_tests = Vec::new();

        for changed_file in changed_files {
            // Get tests directly affected by this file change
            let test_ids = self.graph.get_affected_tests(changed_file);

            for test_id in test_ids {
                if !affected_test_ids.contains(&test_id) {
                    affected_test_ids.insert(test_id);

                    // Find the actual test case
                    for tests in self.tests_by_file.values() {
                        if let Some(test) = tests.iter().find(|t| t.id == test_id) {
                            affected_tests.push(test.clone());
                            break;
                        }
                    }
                }
            }
        }

        affected_tests
    }

    /// Get affected test files (not individual tests)
    pub fn get_affected_test_files(&self, changed_files: &[PathBuf]) -> Vec<PathBuf> {
        let mut affected_files = HashSet::new();

        for changed_file in changed_files {
            // Get test files affected by this change
            let test_files = self.graph.get_affected_test_files(changed_file);
            affected_files.extend(test_files);
        }

        affected_files.into_iter().collect()
    }

    /// Get the dependency graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Get mutable reference to the dependency graph
    pub fn graph_mut(&mut self) -> &mut DependencyGraph {
        &mut self.graph
    }

    /// Get all registered tests
    pub fn all_tests(&self) -> Vec<TestCase> {
        self.tests_by_file.values().flatten().cloned().collect()
    }

    /// Get tests for a specific file
    pub fn tests_for_file(&self, file: &Path) -> Option<&Vec<TestCase>> {
        self.tests_by_file.get(file)
    }
}

/// Result of analyzing file changes
#[derive(Debug, Clone)]
pub struct ChangeAnalysis {
    /// Changed test files
    pub changed_test_files: Vec<PathBuf>,
    /// Changed source files
    pub changed_source_files: Vec<PathBuf>,
    /// All affected tests
    pub affected_tests: Vec<TestId>,
    /// Affected test files (including transitive dependencies)
    pub affected_test_files: Vec<PathBuf>,
}

impl ChangeAnalysis {
    /// Create a new change analysis
    pub fn new() -> Self {
        Self {
            changed_test_files: Vec::new(),
            changed_source_files: Vec::new(),
            affected_tests: Vec::new(),
            affected_test_files: Vec::new(),
        }
    }

    /// Check if there are any affected tests
    pub fn has_affected_tests(&self) -> bool {
        !self.affected_tests.is_empty()
    }

    /// Get the total number of affected tests
    pub fn affected_test_count(&self) -> usize {
        self.affected_tests.len()
    }
}

impl Default for ChangeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyze file changes and determine affected tests
pub fn analyze_changes(changed_files: &[PathBuf], graph: &DependencyGraph) -> ChangeAnalysis {
    let mut analysis = ChangeAnalysis::new();

    // Categorize changed files
    let (test_files, source_files) = categorize_changed_files(changed_files);
    analysis.changed_test_files = test_files;
    analysis.changed_source_files = source_files;

    // Get affected tests from all changed files
    let mut affected_test_ids = HashSet::new();
    let mut affected_test_files = HashSet::new();

    for file in changed_files {
        // Get tests affected by this file
        let tests = graph.get_affected_tests(file);
        affected_test_ids.extend(tests);

        // Get test files affected by this file
        let test_files = graph.get_affected_test_files(file);
        affected_test_files.extend(test_files);
    }

    analysis.affected_tests = affected_test_ids.into_iter().collect();
    analysis.affected_test_files = affected_test_files.into_iter().collect();

    analysis
}

/// Watch mode runner that monitors files and re-runs affected tests
pub struct WatchRunner {
    /// File watcher
    watcher: FileWatcher,
    /// Affected test calculator
    calculator: AffectedTestCalculator,
    /// Test filter
    filter: TestFilter,
    /// All discovered tests
    all_tests: Vec<TestCase>,
    /// Executor configuration
    executor_config: dx_py_executor::ExecutorConfig,
    /// Whether to show verbose output
    verbose: bool,
}

impl WatchRunner {
    /// Create a new watch runner
    pub fn new(
        root: PathBuf,
        filter: TestFilter,
        all_tests: Vec<TestCase>,
        graph: DependencyGraph,
        executor_config: dx_py_executor::ExecutorConfig,
        verbose: bool,
    ) -> Result<Self, notify::Error> {
        let config = WatchConfig::default().with_root(root);

        let watcher = FileWatcher::new(config)?;
        let mut calculator = AffectedTestCalculator::new(graph);

        // Register all tests with the calculator
        let mut tests_by_file: std::collections::HashMap<PathBuf, Vec<TestCase>> =
            std::collections::HashMap::new();
        for test in &all_tests {
            tests_by_file.entry(test.file_path.clone()).or_default().push(test.clone());
        }
        for (file, tests) in tests_by_file {
            calculator.register_tests(&file, tests);
        }

        Ok(Self {
            watcher,
            calculator,
            filter,
            all_tests,
            executor_config,
            verbose,
        })
    }

    /// Get a handle to the shutdown flag
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        self.watcher.shutdown_handle()
    }

    /// Run watch mode - blocks until shutdown
    pub fn run(&mut self) -> WatchResult {
        use colored::Colorize;

        println!("{}", "Watch mode started".cyan().bold());
        println!("Watching for changes in: {}", self.watcher.root().display());
        println!("Press {} to exit\n", "Ctrl+C".yellow());

        let mut total_runs = 0;
        let mut total_passed = 0;
        let mut total_failed = 0;

        loop {
            if self.watcher.is_shutdown() {
                println!("\n{}", "Watch mode stopped".yellow());
                break;
            }

            // Wait for file changes
            let changed_files = self.watcher.wait_for_change();

            if changed_files.is_empty() {
                continue;
            }

            // Analyze changes
            let _analysis = analyze_changes(&changed_files, self.calculator.graph());

            if self.verbose {
                println!("\n{}", "─".repeat(50));
                println!("{}", "Files changed:".cyan());
                for file in &changed_files {
                    println!("  {}", file.display());
                }
            }

            // Get affected tests
            let affected_tests = self.calculator.get_affected_tests(&changed_files);

            if affected_tests.is_empty() {
                if self.verbose {
                    println!("{}", "No tests affected by these changes".yellow());
                }
                continue;
            }

            // Filter tests
            let tests_to_run: Vec<TestCase> =
                affected_tests.into_iter().filter(|t| self.filter.matches(&t.name)).collect();

            if tests_to_run.is_empty() {
                if self.verbose {
                    println!("{}", "No matching tests to run".yellow());
                }
                continue;
            }

            println!("\n{}", "─".repeat(50));
            println!("{} {} affected test(s)...", "Running".green().bold(), tests_to_run.len());

            // Execute tests
            let results = self.execute_tests(tests_to_run.clone());
            total_runs += 1;

            // Print results
            let mut passed = 0;
            let mut failed = 0;

            for result in &results {
                let test = tests_to_run.iter().find(|t| t.id == result.test_id);
                let name = test.map(|t| t.full_name()).unwrap_or_else(|| "unknown".to_string());

                match &result.status {
                    dx_py_core::TestStatus::Pass => {
                        passed += 1;
                        println!(
                            "  {} {} ({:.2}ms)",
                            "✓".green(),
                            name,
                            result.duration.as_secs_f64() * 1000.0
                        );
                    }
                    dx_py_core::TestStatus::Fail => {
                        failed += 1;
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
                        failed += 1;
                        println!("  {} {} (error: {})", "!".red().bold(), name, message);
                    }
                }
            }

            total_passed += passed;
            total_failed += failed;

            // Print summary
            println!("{}", "─".repeat(50));
            if failed == 0 {
                println!("{} {} passed", "✓".green().bold(), passed.to_string().green());
            } else {
                println!(
                    "{} {} passed, {} failed",
                    "✗".red().bold(),
                    passed.to_string().green(),
                    failed.to_string().red()
                );
            }
            println!("{}", "─".repeat(50));
            println!("\n{}", "Watching for changes...".cyan());
        }

        WatchResult {
            total_runs,
            total_passed,
            total_failed,
        }
    }

    /// Execute a set of tests
    fn execute_tests(&self, tests: Vec<TestCase>) -> Vec<dx_py_core::TestResult> {
        let executor = dx_py_executor::WorkStealingExecutor::new(self.executor_config.clone());

        if let Err(e) = executor.submit_all(tests) {
            eprintln!("Failed to submit tests: {}", e);
            return Vec::new();
        }

        executor.execute()
    }
}

/// Result of a watch mode session
#[derive(Debug, Clone)]
pub struct WatchResult {
    /// Total number of test runs
    pub total_runs: usize,
    /// Total tests passed across all runs
    pub total_passed: usize,
    /// Total tests failed across all runs
    pub total_failed: usize,
}

impl WatchResult {
    /// Check if all tests passed
    pub fn is_success(&self) -> bool {
        self.total_failed == 0
    }
}

/// Run watch mode with the given configuration
pub fn run_watch_mode(
    root: &Path,
    filter: &TestFilter,
    all_tests: Vec<TestCase>,
    graph: DependencyGraph,
    executor_config: dx_py_executor::ExecutorConfig,
    verbose: bool,
) -> Result<WatchResult, notify::Error> {
    let mut runner = WatchRunner::new(
        root.to_path_buf(),
        TestFilter::new(filter.pattern()),
        all_tests,
        graph,
        executor_config,
        verbose,
    )?;

    // Set up Ctrl+C handler
    let shutdown = runner.shutdown_handle();
    ctrlc_handler(shutdown);

    Ok(runner.run())
}

/// Set up Ctrl+C handler for graceful shutdown
fn ctrlc_handler(shutdown: Arc<AtomicBool>) {
    // Note: In a real implementation, we'd use the ctrlc crate
    // For now, we rely on the user pressing Ctrl+C which will terminate the process
    // The shutdown flag can be used by other parts of the code to check for shutdown
    let _ = shutdown; // Suppress unused warning
}

/// Filter changed files to only those matching the test filter
pub fn filter_changed_files(changed: &[PathBuf], filter: &TestFilter) -> Vec<PathBuf> {
    if filter.matches_all() {
        return changed.to_vec();
    }

    changed
        .iter()
        .filter(|p| {
            if let Some(name) = p.file_stem() {
                filter.matches(&name.to_string_lossy())
            } else {
                false
            }
        })
        .cloned()
        .collect()
}

/// Categorize changed files into test files and source files
pub fn categorize_changed_files(changed: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut test_files = Vec::new();
    let mut source_files = Vec::new();

    for path in changed {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("test_") || name.ends_with("_test.py") {
                test_files.push(path.clone());
            } else {
                source_files.push(path.clone());
            }
        }
    }

    (test_files, source_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert_eq!(config.extensions, vec!["py"]);
        assert_eq!(config.debounce, Duration::from_millis(100));
    }

    #[test]
    fn test_watch_config_builder() {
        let config = WatchConfig::default()
            .with_root("/tmp/project")
            .with_debounce(Duration::from_millis(200))
            .with_min_interval(Duration::from_secs(1))
            .with_extensions(vec!["py".to_string(), "pyi".to_string()]);

        assert_eq!(config.root, PathBuf::from("/tmp/project"));
        assert_eq!(config.debounce, Duration::from_millis(200));
        assert_eq!(config.min_interval, Duration::from_secs(1));
        assert_eq!(config.extensions.len(), 2);
    }

    #[test]
    fn test_file_change_batch() {
        let mut batch = FileChangeBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);

        batch.add(PathBuf::from("test_a.py"));
        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 1);

        // Adding same file shouldn't duplicate
        batch.add(PathBuf::from("test_a.py"));
        assert_eq!(batch.len(), 1);

        batch.add(PathBuf::from("test_b.py"));
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_filter_changed_files_match_all() {
        let filter = TestFilter::new("*");
        let changed = vec![PathBuf::from("test_foo.py"), PathBuf::from("test_bar.py")];

        let filtered = filter_changed_files(&changed, &filter);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_changed_files_pattern() {
        let filter = TestFilter::new("test_auth*");
        let changed = vec![
            PathBuf::from("test_auth_login.py"),
            PathBuf::from("test_auth_logout.py"),
            PathBuf::from("test_user.py"),
        ];

        let filtered = filter_changed_files(&changed, &filter);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_categorize_changed_files() {
        let changed = vec![
            PathBuf::from("test_auth.py"),
            PathBuf::from("utils.py"),
            PathBuf::from("models_test.py"),
            PathBuf::from("config.py"),
        ];

        let (test_files, source_files) = categorize_changed_files(&changed);

        assert_eq!(test_files.len(), 2);
        assert!(test_files.contains(&PathBuf::from("test_auth.py")));
        assert!(test_files.contains(&PathBuf::from("models_test.py")));

        assert_eq!(source_files.len(), 2);
        assert!(source_files.contains(&PathBuf::from("utils.py")));
        assert!(source_files.contains(&PathBuf::from("config.py")));
    }

    #[test]
    fn test_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatchConfig {
            root: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let watcher = FileWatcher::new(config);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatchConfig {
            root: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let watcher = FileWatcher::new(config).unwrap();
        assert!(!watcher.is_shutdown());

        watcher.shutdown();
        assert!(watcher.is_shutdown());
    }

    #[test]
    fn test_debounce_complete() {
        let mut batch = FileChangeBatch::new();
        batch.add(PathBuf::from("test.py"));

        // Immediately after adding, debounce should not be complete
        assert!(!batch.is_debounce_complete(Duration::from_millis(100)));

        // After waiting, it should be complete
        std::thread::sleep(Duration::from_millis(150));
        assert!(batch.is_debounce_complete(Duration::from_millis(100)));
    }
}

// Property tests
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: dx-py-production-ready, Property 6: Watch Mode Affected Test Detection
    // Validates: Requirements 7.2, 7.3
    //
    // For any file change, the set of tests re-run SHALL include all tests
    // that directly or transitively depend on the changed file.
    proptest! {
        /// Property 6: Watch Mode Affected Test Detection
        /// For any file change, the set of tests re-run SHALL include all tests
        /// that directly or transitively depend on the changed file.
        /// **Validates: Requirements 7.2, 7.3**
        #[test]
        fn prop_affected_test_detection(
            num_source_files in 1usize..=5usize,
            num_test_files in 1usize..=5usize,
            num_imports in 0usize..=10usize,
        ) {
            let mut graph = DependencyGraph::new();

            // Create source files
            let source_files: Vec<PathBuf> = (0..num_source_files)
                .map(|i| PathBuf::from(format!("src/module_{}.py", i)))
                .collect();

            // Create test files
            let test_files: Vec<PathBuf> = (0..num_test_files)
                .map(|i| PathBuf::from(format!("tests/test_module_{}.py", i)))
                .collect();

            // Add all files to graph
            for file in &source_files {
                graph.add_file(file, false);
            }
            for file in &test_files {
                graph.add_file(file, true);
            }

            // Create test IDs for each test file
            let mut test_ids_by_file: std::collections::HashMap<PathBuf, Vec<TestId>> =
                std::collections::HashMap::new();
            let mut all_test_ids = Vec::new();

            for (i, test_file) in test_files.iter().enumerate() {
                let test_id = TestId((i as u64) * 1000);
                test_ids_by_file.insert(test_file.clone(), vec![test_id]);
                all_test_ids.push((test_file.clone(), test_id));
                graph.set_file_tests(test_file, vec![test_id]);
            }

            // Create import relationships (test files import source files)
            let mut imports: Vec<(PathBuf, PathBuf)> = Vec::new();
            for i in 0..num_imports.min(num_test_files * num_source_files) {
                let test_idx = i % num_test_files;
                let source_idx = i % num_source_files;
                let test_file = &test_files[test_idx];
                let source_file = &source_files[source_idx];

                graph.add_import(test_file, source_file);
                imports.push((test_file.clone(), source_file.clone()));
            }

            // Test: When a source file changes, all tests that import it should be affected
            for source_file in &source_files {
                let affected_tests = graph.get_affected_tests(source_file);
                let affected_test_files = graph.get_affected_test_files(source_file);

                // Find all test files that import this source file
                let expected_test_files: HashSet<PathBuf> = imports
                    .iter()
                    .filter(|(_, imported)| imported == source_file)
                    .map(|(importer, _)| importer.clone())
                    .collect();

                // All expected test files should be in affected test files
                for expected in &expected_test_files {
                    prop_assert!(
                        affected_test_files.contains(expected),
                        "Test file {} should be affected by change to {}",
                        expected.display(),
                        source_file.display()
                    );
                }

                // All tests from expected test files should be in affected tests
                for expected_file in &expected_test_files {
                    if let Some(test_ids) = test_ids_by_file.get(expected_file) {
                        for test_id in test_ids {
                            prop_assert!(
                                affected_tests.contains(test_id),
                                "Test {:?} from {} should be affected by change to {}",
                                test_id,
                                expected_file.display(),
                                source_file.display()
                            );
                        }
                    }
                }
            }

            // Test: When a test file changes, its own tests should be affected
            for test_file in &test_files {
                let affected_tests = graph.get_affected_tests(test_file);

                if let Some(test_ids) = test_ids_by_file.get(test_file) {
                    for test_id in test_ids {
                        prop_assert!(
                            affected_tests.contains(test_id),
                            "Test {:?} should be affected by change to its own file {}",
                            test_id,
                            test_file.display()
                        );
                    }
                }
            }
        }

        /// Property: Transitive dependency detection
        /// If A imports B and B imports C, changing C should affect tests in A
        /// **Validates: Requirements 7.3**
        #[test]
        fn prop_transitive_dependency_detection(
            chain_length in 2usize..=5usize,
        ) {
            let mut graph = DependencyGraph::new();

            // Create a chain: test.py -> module_0.py -> module_1.py -> ... -> module_n.py
            let test_file = PathBuf::from("test_chain.py");
            let modules: Vec<PathBuf> = (0..chain_length)
                .map(|i| PathBuf::from(format!("module_{}.py", i)))
                .collect();

            // Add files
            graph.add_file(&test_file, true);
            for module in &modules {
                graph.add_file(module, false);
            }

            // Set up test
            let test_id = TestId(42);
            graph.set_file_tests(&test_file, vec![test_id]);

            // Create chain: test imports module_0, module_0 imports module_1, etc.
            graph.add_import(&test_file, &modules[0]);
            for i in 0..modules.len() - 1 {
                graph.add_import(&modules[i], &modules[i + 1]);
            }

            // Changing any module in the chain should affect the test
            for module in &modules {
                let affected = graph.get_affected_tests(module);
                prop_assert!(
                    affected.contains(&test_id),
                    "Test should be affected by change to {} in the import chain",
                    module.display()
                );
            }
        }

        /// Property: Watch mode filtering preserves matching tests
        #[test]
        fn prop_watch_mode_filtering(
            prefix in "[a-z]{3,5}",
            files in prop::collection::vec("[a-z_]{5,15}", 1..10),
        ) {
            let pattern = format!("{}*", prefix);
            let filter = TestFilter::new(&pattern);

            let changed: Vec<PathBuf> = files
                .iter()
                .map(|f| PathBuf::from(format!("{}.py", f)))
                .collect();

            let filtered = filter_changed_files(&changed, &filter);

            // Verify filtered files match the pattern
            for path in &filtered {
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy();
                    prop_assert!(filter.matches(&name),
                        "Filtered file '{}' should match pattern '{}'",
                        name, pattern);
                }
            }

            // Verify non-filtered files don't match
            for path in &changed {
                if !filtered.contains(path) {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy();
                        prop_assert!(!filter.matches(&name) || filter.matches_all(),
                            "Non-filtered file '{}' should not match pattern '{}'",
                            name, pattern);
                    }
                }
            }
        }

        #[test]
        fn prop_match_all_includes_everything(
            files in prop::collection::vec("[a-z_]{5,15}", 1..10),
        ) {
            let filter = TestFilter::new("*");
            let changed: Vec<PathBuf> = files
                .iter()
                .map(|f| PathBuf::from(format!("{}.py", f)))
                .collect();

            let filtered = filter_changed_files(&changed, &filter);
            prop_assert_eq!(filtered.len(), changed.len());
        }

        #[test]
        fn prop_categorize_preserves_all_files(
            test_files in prop::collection::vec("test_[a-z]{3,10}", 0..5),
            source_files in prop::collection::vec("[a-z]{3,10}", 0..5),
        ) {
            let mut all_files: Vec<PathBuf> = test_files
                .iter()
                .map(|f| PathBuf::from(format!("{}.py", f)))
                .collect();
            all_files.extend(
                source_files
                    .iter()
                    .map(|f| PathBuf::from(format!("{}.py", f)))
            );

            let (categorized_tests, categorized_sources) = categorize_changed_files(&all_files);

            // Total should equal original
            prop_assert_eq!(
                categorized_tests.len() + categorized_sources.len(),
                all_files.len()
            );
        }

        #[test]
        fn prop_file_change_batch_no_duplicates(
            files in prop::collection::vec("[a-z_]{5,15}", 1..20),
        ) {
            let mut batch = FileChangeBatch::new();

            // Add each file twice
            for file in &files {
                let path = PathBuf::from(format!("{}.py", file));
                batch.add(path.clone());
                batch.add(path);
            }

            // Should have no duplicates
            let unique_files: HashSet<_> = files.iter().collect();
            prop_assert_eq!(batch.len(), unique_files.len());
        }
    }
}
