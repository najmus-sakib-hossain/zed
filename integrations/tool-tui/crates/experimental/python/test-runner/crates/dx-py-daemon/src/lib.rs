//! Daemon pool for pre-warmed Python interpreters
//!
//! This crate manages a pool of pre-warmed Python interpreters
//! for fast test execution with real Python code execution.

pub mod worker;

pub use dx_py_core::{
    AssertionFailure, AssertionStats, DaemonError, TestCase, TestId, TestResult, TestStatus,
};
pub use worker::{TestWorker, WorkerRequest, WorkerResponse, WORKER_SCRIPT};

use crossbeam::channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Configuration for the daemon pool
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Number of workers in the pool
    pub pool_size: usize,
    /// Modules to pre-import in workers
    pub preload_modules: Vec<String>,
    /// Python executable path
    pub python_path: String,
    /// Timeout for test execution
    pub timeout: Duration,
    /// Maximum number of restart attempts for crashed workers
    pub max_restart_attempts: u32,
    /// Delay between restart attempts
    pub restart_delay: Duration,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            pool_size: num_cpus::get(),
            preload_modules: vec![],
            python_path: "python".to_string(),
            timeout: Duration::from_secs(60),
            max_restart_attempts: 3,
            restart_delay: Duration::from_millis(100),
        }
    }
}

impl DaemonConfig {
    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    pub fn with_preload(mut self, modules: Vec<String>) -> Self {
        self.preload_modules = modules;
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

    pub fn with_max_restart_attempts(mut self, attempts: u32) -> Self {
        self.max_restart_attempts = attempts;
        self
    }

    pub fn with_restart_delay(mut self, delay: Duration) -> Self {
        self.restart_delay = delay;
        self
    }
}

/// State of a worker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Idle,
    Busy,
    Crashed,
}

/// Statistics about worker crashes
#[derive(Debug, Clone)]
pub struct WorkerCrashStats {
    /// Worker ID
    pub worker_id: usize,
    /// Number of times this worker has been restarted
    pub restart_count: u32,
    /// Last crash reason (if any)
    pub last_crash_reason: Option<String>,
    /// Current worker state
    pub state: WorkerState,
}

/// Pool of pre-warmed Python workers
pub struct DaemonPool {
    config: DaemonConfig,
    workers: Arc<Mutex<Vec<TestWorker>>>,
    available_count: AtomicUsize,
    shutdown: AtomicBool,
    test_queue: Sender<TestCase>,
    test_receiver: Receiver<TestCase>,
}

impl DaemonPool {
    /// Create a new daemon pool with the given configuration
    pub fn new(config: DaemonConfig) -> Result<Self, DaemonError> {
        let (tx, rx) = bounded(config.pool_size * 10);
        let mut workers = Vec::with_capacity(config.pool_size);

        for i in 0..config.pool_size {
            let mut worker = TestWorker::new(i);
            worker.spawn(&config)?;
            workers.push(worker);
        }

        Ok(Self {
            available_count: AtomicUsize::new(config.pool_size),
            config,
            workers: Arc::new(Mutex::new(workers)),
            shutdown: AtomicBool::new(false),
            test_queue: tx,
            test_receiver: rx,
        })
    }

    /// Get the pool size
    pub fn pool_size(&self) -> usize {
        self.config.pool_size
    }

    /// Get the number of available workers
    pub fn available_workers(&self) -> usize {
        self.available_count.load(Ordering::Acquire)
    }

    /// Get the number of busy workers
    pub fn busy_workers(&self) -> usize {
        self.pool_size() - self.available_workers()
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Acquire a worker for test execution
    pub fn acquire_worker(&self) -> Result<usize, DaemonError> {
        if self.is_shutdown() {
            return Err(DaemonError::ShutdownError("Pool is shutting down".into()));
        }

        let mut workers = self.workers.lock().unwrap();
        for (i, worker) in workers.iter_mut().enumerate() {
            if worker.is_available() {
                worker.mark_busy();
                self.available_count.fetch_sub(1, Ordering::Release);
                return Ok(i);
            }
        }

        Err(DaemonError::NoWorkerAvailable)
    }

    /// Release a worker back to the pool
    pub fn release_worker(&self, worker_id: usize) -> Result<(), DaemonError> {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return Err(DaemonError::WorkerCrash(format!(
                "Invalid worker id: {}",
                worker_id
            )));
        }

        workers[worker_id].mark_idle();
        self.available_count.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Execute a test on a specific worker
    pub fn execute_test(
        &self,
        worker_id: usize,
        test: &TestCase,
    ) -> Result<TestResult, DaemonError> {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return Err(DaemonError::WorkerCrash(format!(
                "Invalid worker id: {}",
                worker_id
            )));
        }

        workers[worker_id].execute_test(test, self.config.timeout)
    }

    /// Mark a worker as crashed and attempt to restart it
    pub fn handle_worker_crash(&self, worker_id: usize) -> Result<(), DaemonError> {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return Err(DaemonError::WorkerCrash(format!(
                "Invalid worker id: {}",
                worker_id
            )));
        }

        workers[worker_id].mark_crashed();
        let _ = workers[worker_id].terminate();
        workers[worker_id].spawn(&self.config)?;
        self.available_count.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Handle a worker crash with automatic restart and retry logic
    /// Returns the crash reason if restart fails after max attempts
    pub fn handle_worker_crash_with_retry(
        &self,
        worker_id: usize,
        crash_reason: &str,
    ) -> Result<(), DaemonError> {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return Err(DaemonError::WorkerCrash(format!(
                "Invalid worker id: {}",
                worker_id
            )));
        }

        // Record the crash
        workers[worker_id].record_crash(crash_reason.to_string());
        let restart_count = workers[worker_id].get_restart_count();

        // Check if we've exceeded max restart attempts
        if restart_count > self.config.max_restart_attempts {
            return Err(DaemonError::WorkerCrash(format!(
                "Worker {} crashed {} times (max: {}). Last crash reason: {}",
                worker_id, restart_count, self.config.max_restart_attempts, crash_reason
            )));
        }

        // Terminate the crashed worker
        if let Err(e) = workers[worker_id].terminate() {
            #[cfg(debug_assertions)]
            eprintln!(
                "[DAEMON WARN] Failed to terminate crashed worker {}: {}",
                worker_id, e
            );
        }

        // Wait before restarting (helps avoid rapid restart loops)
        drop(workers); // Release lock during sleep
        std::thread::sleep(self.config.restart_delay);
        let mut workers = self.workers.lock().unwrap();

        // Attempt to restart
        match workers[worker_id].spawn(&self.config) {
            Ok(()) => {
                self.available_count.fetch_add(1, Ordering::Release);
                #[cfg(debug_assertions)]
                eprintln!(
                    "[DAEMON INFO] Worker {} restarted successfully (attempt {}/{})",
                    worker_id, restart_count, self.config.max_restart_attempts
                );
                Ok(())
            }
            Err(e) => Err(DaemonError::WorkerCrash(format!(
                "Failed to restart worker {} after crash: {}. Original crash reason: {}",
                worker_id, e, crash_reason
            ))),
        }
    }

    /// Execute a test with automatic crash recovery
    /// If the worker crashes, it will be restarted and the test will be retried
    pub fn execute_test_with_recovery(
        &self,
        worker_id: usize,
        test: &TestCase,
    ) -> Result<TestResult, DaemonError> {
        match self.execute_test(worker_id, test) {
            Ok(result) => Ok(result),
            Err(DaemonError::WorkerCrash(reason)) => {
                // Worker crashed - attempt recovery
                #[cfg(debug_assertions)]
                eprintln!(
                    "[DAEMON WARN] Worker {} crashed during test '{}': {}",
                    worker_id,
                    test.full_name(),
                    reason
                );

                // Try to restart the worker
                self.handle_worker_crash_with_retry(worker_id, &reason)?;

                // Return an error result for this test (don't retry the test itself)
                Ok(TestResult {
                    test_id: test.id,
                    status: TestStatus::Error {
                        message: format!("Worker crashed during execution: {}", reason),
                    },
                    duration: Duration::ZERO,
                    stdout: String::new(),
                    stderr: String::new(),
                    traceback: Some(format!(
                        "Worker {} crashed and was restarted. Crash reason: {}",
                        worker_id, reason
                    )),
                    assertions: AssertionStats::default(),
                    assertion_failure: None,
                })
            }
            Err(DaemonError::Timeout(duration)) => {
                // Timeout - worker might be stuck, restart it
                #[cfg(debug_assertions)]
                eprintln!(
                    "[DAEMON WARN] Worker {} timed out after {:?} during test '{}'",
                    worker_id,
                    duration,
                    test.full_name()
                );

                let reason = format!("Test execution timed out after {:?}", duration);
                self.handle_worker_crash_with_retry(worker_id, &reason)?;

                Ok(TestResult {
                    test_id: test.id,
                    status: TestStatus::Error {
                        message: format!("Test timed out after {:?}", duration),
                    },
                    duration,
                    stdout: String::new(),
                    stderr: String::new(),
                    traceback: Some(format!(
                        "Worker {} was restarted due to timeout. Test: {}",
                        worker_id,
                        test.full_name()
                    )),
                    assertions: AssertionStats::default(),
                    assertion_failure: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Check if a worker is alive
    pub fn is_worker_alive(&self, worker_id: usize) -> bool {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return false;
        }
        workers[worker_id].is_process_alive()
    }

    /// Check worker health and restart if needed
    /// Returns Ok(()) if worker is healthy or was successfully restarted
    pub fn ensure_worker_healthy(&self, worker_id: usize) -> Result<(), DaemonError> {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return Err(DaemonError::WorkerCrash(format!(
                "Invalid worker id: {}",
                worker_id
            )));
        }

        // Check if worker is alive
        if !workers[worker_id].is_process_alive() {
            let crash_reason = workers[worker_id]
                .get_last_crash_reason()
                .unwrap_or("Worker process died unexpectedly")
                .to_string();

            // Try to restart
            drop(workers); // Release lock
            return self.handle_worker_crash_with_retry(worker_id, &crash_reason);
        }

        Ok(())
    }

    /// Ping a worker to check if it's responsive
    pub fn ping_worker(&self, worker_id: usize) -> bool {
        let mut workers = self.workers.lock().unwrap();
        if worker_id >= workers.len() {
            return false;
        }
        workers[worker_id].ping(Duration::from_secs(5))
    }

    /// Get crash statistics for all workers
    pub fn get_crash_stats(&self) -> Vec<WorkerCrashStats> {
        let workers = self.workers.lock().unwrap();
        workers
            .iter()
            .enumerate()
            .map(|(id, worker)| WorkerCrashStats {
                worker_id: id,
                restart_count: worker.get_restart_count(),
                last_crash_reason: worker.get_last_crash_reason().map(|s| s.to_string()),
                state: worker.state,
            })
            .collect()
    }

    /// Queue a test for execution
    pub fn queue_test(&self, test: TestCase) -> Result<(), DaemonError> {
        if self.is_shutdown() {
            return Err(DaemonError::ShutdownError("Pool is shutting down".into()));
        }

        self.test_queue
            .send(test)
            .map_err(|e| DaemonError::WorkerCrash(e.to_string()))
    }

    /// Get the next queued test
    pub fn next_queued_test(&self) -> Option<TestCase> {
        self.test_receiver.try_recv().ok()
    }

    /// Get the number of queued tests
    pub fn queued_tests(&self) -> usize {
        self.test_receiver.len()
    }

    /// Gracefully shutdown the pool
    pub fn shutdown(&self) -> Result<(), DaemonError> {
        self.shutdown.store(true, Ordering::Release);

        let mut workers = self.workers.lock().unwrap();
        let mut errors = Vec::new();

        for worker in workers.iter_mut() {
            if let Err(e) = worker.terminate() {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(DaemonError::ShutdownError(format!(
                "Failed to terminate {} workers",
                errors.len()
            )))
        }
    }
}

impl Drop for DaemonPool {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests;
