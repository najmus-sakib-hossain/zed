//! Work-stealing parallel executor
//!
//! This crate implements a work-stealing executor that distributes
//! tests across workers with dynamic load balancing and real Python execution.

pub use dx_py_core::{
    AssertionFailure, AssertionStats, ExecutionError, TestCase, TestId, TestResult, TestStatus,
};
pub use dx_py_daemon::{DaemonConfig, DaemonError, DaemonPool};

use crossbeam::deque::{Injector, Stealer, Worker};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Configuration for the executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Number of worker threads
    pub num_workers: usize,
    /// Whether to continue on worker panic
    pub fault_tolerant: bool,
    /// Python executable path
    pub python_path: String,
    /// Timeout for test execution
    pub timeout: Duration,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            num_workers: num_cpus::get(),
            fault_tolerant: true,
            python_path: "python".to_string(),
            timeout: Duration::from_secs(60),
        }
    }
}

impl ExecutorConfig {
    pub fn with_workers(mut self, n: usize) -> Self {
        self.num_workers = n;
        self
    }

    pub fn with_fault_tolerance(mut self, enabled: bool) -> Self {
        self.fault_tolerant = enabled;
        self
    }

    pub fn with_python(mut self, path: impl Into<String>) -> Self {
        self.python_path = path.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Work-stealing executor for parallel test execution with real Python execution
pub struct WorkStealingExecutor {
    config: ExecutorConfig,
    global_queue: Arc<Injector<TestCase>>,
    results: Arc<Mutex<HashMap<TestId, TestResult>>>,
    pending_count: Arc<AtomicUsize>,
    completed_count: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
    worker_panics: Arc<AtomicUsize>,
}

impl WorkStealingExecutor {
    /// Create a new executor with the given configuration
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            config,
            global_queue: Arc::new(Injector::new()),
            results: Arc::new(Mutex::new(HashMap::new())),
            pending_count: Arc::new(AtomicUsize::new(0)),
            completed_count: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            worker_panics: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Submit a test for execution
    pub fn submit(&self, test: TestCase) -> Result<(), ExecutionError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(ExecutionError::QueueFull);
        }
        self.pending_count.fetch_add(1, Ordering::Release);
        self.global_queue.push(test);
        Ok(())
    }

    /// Submit multiple tests for execution
    pub fn submit_all(&self, tests: Vec<TestCase>) -> Result<(), ExecutionError> {
        for test in tests {
            self.submit(test)?;
        }
        Ok(())
    }

    /// Get the number of pending tests
    pub fn pending(&self) -> usize {
        self.pending_count.load(Ordering::Acquire)
    }

    /// Get the number of completed tests
    pub fn completed(&self) -> usize {
        self.completed_count.load(Ordering::Acquire)
    }

    /// Get the number of worker panics
    pub fn panics(&self) -> usize {
        self.worker_panics.load(Ordering::Acquire)
    }

    /// Execute all submitted tests and return results
    pub fn execute(&self) -> Vec<TestResult> {
        let num_workers = self.config.num_workers;
        let fault_tolerant = self.config.fault_tolerant;

        // Create daemon pool for Python execution
        let daemon_config = DaemonConfig::default()
            .with_pool_size(num_workers)
            .with_python(self.config.python_path.clone())
            .with_timeout(self.config.timeout);

        let daemon_pool = match DaemonPool::new(daemon_config) {
            Ok(pool) => Arc::new(pool),
            Err(e) => {
                // If we can't create the daemon pool, return error results for all tests
                eprintln!("Failed to create daemon pool: {}", e);
                let mut results = Vec::new();
                loop {
                    match self.global_queue.steal() {
                        crossbeam::deque::Steal::Success(test) => {
                            results.push(TestResult::error(
                                test.id,
                                format!("Daemon pool creation failed: {}", e),
                            ));
                        }
                        crossbeam::deque::Steal::Empty => break,
                        crossbeam::deque::Steal::Retry => continue,
                    }
                }
                return results;
            }
        };

        // Create per-worker local queues
        let workers: Vec<Worker<TestCase>> = (0..num_workers).map(|_| Worker::new_fifo()).collect();

        let stealers: Vec<Stealer<TestCase>> = workers.iter().map(|w| w.stealer()).collect();

        let stealers = Arc::new(stealers);

        // Spawn worker threads
        let handles: Vec<_> = workers
            .into_iter()
            .enumerate()
            .map(|(id, local)| {
                let global = Arc::clone(&self.global_queue);
                let stealers = Arc::clone(&stealers);
                let results = Arc::clone(&self.results);
                let pending = Arc::clone(&self.pending_count);
                let completed = Arc::clone(&self.completed_count);
                let shutdown = Arc::clone(&self.shutdown);
                let panics = Arc::clone(&self.worker_panics);
                let shutdown_on_panic = Arc::clone(&self.shutdown);
                let daemon_pool = Arc::clone(&daemon_pool);
                let timeout = self.config.timeout;

                thread::spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        worker_loop(
                            id,
                            local,
                            global,
                            stealers,
                            results,
                            pending,
                            completed,
                            shutdown,
                            daemon_pool,
                            timeout,
                        )
                    }));

                    if result.is_err() {
                        panics.fetch_add(1, Ordering::Release);
                        if !fault_tolerant {
                            shutdown_on_panic.store(true, Ordering::Release);
                        }
                    }
                })
            })
            .collect();

        // Wait for all workers to complete
        for handle in handles {
            let _ = handle.join();
        }

        // Collect results
        let results = self.results.lock().unwrap();
        results.values().cloned().collect()
    }

    /// Shutdown the executor
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// Check if all tests have been processed
    pub fn is_complete(&self) -> bool {
        self.pending() == 0 && self.global_queue.is_empty()
    }

    /// Get a result by test ID
    pub fn get_result(&self, test_id: TestId) -> Option<TestResult> {
        let results = self.results.lock().unwrap();
        results.get(&test_id).cloned()
    }
}

#[allow(clippy::too_many_arguments)]
fn worker_loop(
    id: usize,
    local: Worker<TestCase>,
    global: Arc<Injector<TestCase>>,
    stealers: Arc<Vec<Stealer<TestCase>>>,
    results: Arc<Mutex<HashMap<TestId, TestResult>>>,
    pending: Arc<AtomicUsize>,
    completed: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
    daemon_pool: Arc<DaemonPool>,
    timeout: Duration,
) {
    loop {
        if shutdown.load(Ordering::Acquire) {
            break;
        }

        // Try to get work: local queue first, then global, then steal
        let test = local.pop().or_else(|| {
            // Try global queue
            loop {
                match global.steal() {
                    crossbeam::deque::Steal::Success(t) => return Some(t),
                    crossbeam::deque::Steal::Empty => break,
                    crossbeam::deque::Steal::Retry => continue,
                }
            }

            // Try stealing from other workers
            for stealer in stealers.iter() {
                loop {
                    match stealer.steal() {
                        crossbeam::deque::Steal::Success(t) => return Some(t),
                        crossbeam::deque::Steal::Empty => break,
                        crossbeam::deque::Steal::Retry => continue,
                    }
                }
            }

            None
        });

        match test {
            Some(test) => {
                // Execute the test using the daemon pool
                let result = execute_test_with_daemon(&daemon_pool, &test, id, timeout);

                let test_id = result.test_id;
                {
                    let mut results = results.lock().unwrap();
                    results.insert(test_id, result);
                }

                pending.fetch_sub(1, Ordering::Release);
                completed.fetch_add(1, Ordering::Release);
            }
            None => {
                // No work available
                if pending.load(Ordering::Acquire) == 0 {
                    break;
                }
                // Brief sleep to avoid busy-waiting
                thread::sleep(Duration::from_micros(100));
            }
        }
    }
}

/// Execute a single test using the daemon pool with crash recovery
fn execute_test_with_daemon(
    daemon_pool: &DaemonPool,
    test: &TestCase,
    _worker_id: usize,
    _timeout: Duration,
) -> TestResult {
    let start = Instant::now();

    // Try to acquire a worker from the daemon pool
    let daemon_worker_id = match daemon_pool.acquire_worker() {
        Ok(id) => id,
        Err(DaemonError::NoWorkerAvailable) => {
            // Wait and retry with exponential backoff
            let mut acquired = None;
            for attempt in 0..10 {
                let delay = Duration::from_millis(50 * (1 << attempt.min(4)));
                thread::sleep(delay);
                if let Ok(id) = daemon_pool.acquire_worker() {
                    acquired = Some(id);
                    break;
                }
            }
            match acquired {
                Some(id) => id,
                None => {
                    return TestResult::error(
                        test.id,
                        "Failed to acquire worker: no workers available after retries",
                    );
                }
            }
        }
        Err(e) => {
            return TestResult::error(test.id, format!("Failed to acquire worker: {}", e));
        }
    };

    // Execute the test with automatic crash recovery
    let result = match daemon_pool.execute_test_with_recovery(daemon_worker_id, test) {
        Ok(result) => result,
        Err(DaemonError::Timeout(duration)) => {
            // Handle timeout - restart the worker
            let _ = daemon_pool.handle_worker_crash(daemon_worker_id);
            TestResult {
                test_id: test.id,
                status: TestStatus::Error {
                    message: format!("Test timed out after {:?}", duration),
                },
                duration: start.elapsed(),
                stdout: String::new(),
                stderr: String::new(),
                traceback: Some(format!(
                    "Test '{}' exceeded timeout of {:?}",
                    test.full_name(),
                    duration
                )),
                assertions: AssertionStats::default(),
                assertion_failure: None,
            }
        }
        Err(DaemonError::WorkerCrash(msg)) => {
            // Handle crash - restart the worker
            let _ = daemon_pool.handle_worker_crash(daemon_worker_id);
            TestResult {
                test_id: test.id,
                status: TestStatus::Error {
                    message: format!("Worker crashed: {}", msg),
                },
                duration: start.elapsed(),
                stdout: String::new(),
                stderr: String::new(),
                traceback: Some(format!(
                    "Worker crashed while executing test '{}': {}",
                    test.full_name(),
                    msg
                )),
                assertions: AssertionStats::default(),
                assertion_failure: None,
            }
        }
        Err(e) => TestResult::error(test.id, format!("Execution error: {}", e)),
    };

    // Release the worker back to the pool
    let _ = daemon_pool.release_worker(daemon_worker_id);

    result
}

/// Summary of execution results
#[derive(Debug, Clone, Default)]
pub struct ExecutionSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub panics: usize,
    pub duration: Duration,
}

impl ExecutionSummary {
    pub fn from_results(results: &[TestResult], panics: usize) -> Self {
        let mut summary = Self {
            panics,
            ..Default::default()
        };

        summary.total = results.len();
        for result in results {
            summary.duration += result.duration;
            match &result.status {
                TestStatus::Pass => summary.passed += 1,
                TestStatus::Fail => summary.failed += 1,
                TestStatus::Skip { .. } => summary.skipped += 1,
                TestStatus::Error { .. } => summary.errors += 1,
            }
        }

        summary
    }

    pub fn is_success(&self) -> bool {
        self.failed == 0 && self.errors == 0 && self.panics == 0
    }
}

#[cfg(test)]
mod tests;
