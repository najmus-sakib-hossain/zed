//! Per-core state for thread-per-core architecture.

use crate::io::ReactorConfig;
use crossbeam_deque::{Injector, Stealer, Worker};
use std::sync::atomic::{AtomicBool, Ordering};

/// Task type for the work queue.
pub type Task = Box<dyn FnOnce() + Send + 'static>;

/// Per-CPU-core state.
///
/// Each core has its own local work queue with no shared locks.
/// Work-stealing is only used when a core's queue is empty.
pub struct CoreState {
    /// Core ID (0-indexed).
    id: usize,

    /// Reactor configuration.
    config: ReactorConfig,

    /// Local work queue (LIFO for cache locality).
    local_queue: Worker<Task>,

    /// Stealer for other cores to steal from.
    stealer: Stealer<Task>,

    /// Global injector for external task submission.
    injector: Injector<Task>,

    /// Whether this core is running.
    running: AtomicBool,
}

impl CoreState {
    /// Create a new core state.
    pub fn new(id: usize, config: ReactorConfig) -> Self {
        let local_queue = Worker::new_lifo();
        let stealer = local_queue.stealer();

        Self {
            id,
            config,
            local_queue,
            stealer,
            injector: Injector::new(),
            running: AtomicBool::new(false),
        }
    }

    /// Get the core ID.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the reactor configuration.
    pub fn config(&self) -> &ReactorConfig {
        &self.config
    }

    /// Check if this core is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Push a task to the local queue.
    pub fn push(&self, task: Task) {
        self.local_queue.push(task);
    }

    /// Pop a task from the local queue.
    pub fn pop(&self) -> Option<Task> {
        self.local_queue.pop()
    }

    /// Get a stealer for work-stealing.
    pub fn stealer(&self) -> &Stealer<Task> {
        &self.stealer
    }

    /// Inject a task from outside (thread-safe).
    pub fn inject(&self, task: Task) {
        self.injector.push(task);
    }

    /// Try to steal a task from the injector.
    pub fn steal_from_injector(&self) -> Option<Task> {
        loop {
            match self.injector.steal() {
                crossbeam_deque::Steal::Success(task) => return Some(task),
                crossbeam_deque::Steal::Empty => return None,
                crossbeam_deque::Steal::Retry => continue,
            }
        }
    }

    /// Try to steal a task from another core.
    pub fn steal_from(&self, other: &Stealer<Task>) -> Option<Task> {
        loop {
            match other.steal() {
                crossbeam_deque::Steal::Success(task) => return Some(task),
                crossbeam_deque::Steal::Empty => return None,
                crossbeam_deque::Steal::Retry => continue,
            }
        }
    }

    /// Run the event loop for this core.
    ///
    /// This should be called from a thread pinned to this core.
    pub fn run_event_loop(&self, stealers: &[Stealer<Task>]) {
        self.running.store(true, Ordering::Relaxed);

        while self.running.load(Ordering::Relaxed) {
            // 1. Try local queue first (best cache locality)
            if let Some(task) = self.pop() {
                task();
                continue;
            }

            // 2. Try the injector
            if let Some(task) = self.steal_from_injector() {
                task();
                continue;
            }

            // 3. Try stealing from other cores
            for stealer in stealers {
                if let Some(task) = self.steal_from(stealer) {
                    task();
                    break;
                }
            }

            // 4. No work available - could yield or park here
            std::hint::spin_loop();
        }
    }

    /// Stop the event loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
