//! Worker thread implementation with work-stealing

use crossbeam::deque::{Injector, Stealer, Worker as WorkerDeque};
use parking_lot::Condvar;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use thiserror::Error;

use crate::task::Task;

/// Errors that can occur in worker operations
#[derive(Debug, Error)]
pub enum WorkerError {
    /// Failed to spawn worker thread
    #[error("Failed to spawn worker thread {id}: {reason}")]
    SpawnFailed { id: usize, reason: String },

    /// Worker is not running
    #[error("Worker {id} is not running")]
    NotRunning { id: usize },
}

/// A worker thread in the parallel executor
pub struct Worker {
    /// Worker ID (also the core ID for pinning)
    pub id: usize,
    /// Local work queue
    local_queue: WorkerDeque<Task>,
    /// Thread handle
    handle: Option<JoinHandle<()>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl Worker {
    /// Create a new worker
    pub fn new(id: usize) -> Self {
        Self {
            id,
            local_queue: WorkerDeque::new_fifo(),
            handle: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a stealer for this worker's queue
    pub fn stealer(&self) -> Stealer<Task> {
        self.local_queue.stealer()
    }

    /// Push a task to the local queue
    pub fn push(&self, task: Task) {
        self.local_queue.push(task);
    }

    /// Start the worker thread
    ///
    /// Returns an error if the thread cannot be spawned.
    pub fn start(
        &mut self,
        global_queue: Arc<Injector<Task>>,
        stealers: Vec<Stealer<Task>>,
        parker: Arc<(Mutex<bool>, Condvar)>,
    ) -> Result<(), WorkerError> {
        let id = self.id;
        let shutdown = Arc::clone(&self.shutdown);
        let local = self.local_queue.stealer();

        let handle = thread::Builder::new()
            .name(format!("dx-py-worker-{}", id))
            .spawn(move || {
                // Try to pin to core
                if let Some(core_ids) = core_affinity::get_core_ids() {
                    if id < core_ids.len() {
                        core_affinity::set_for_current(core_ids[id]);
                    }
                }

                worker_loop(id, shutdown, global_queue, local, stealers, parker);
            })
            .map_err(|e| WorkerError::SpawnFailed {
                id,
                reason: e.to_string(),
            })?;

        self.handle = Some(handle);
        Ok(())
    }

    /// Signal the worker to shut down
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Wait for the worker to finish
    pub fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// The main worker loop
fn worker_loop(
    _id: usize,
    shutdown: Arc<AtomicBool>,
    global_queue: Arc<Injector<Task>>,
    local: Stealer<Task>,
    stealers: Vec<Stealer<Task>>,
    parker: Arc<(Mutex<bool>, Condvar)>,
) {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        // Try to get work in priority order:
        // 1. Local queue
        // 2. Global queue
        // 3. Steal from other workers

        let task = find_task(&local, &global_queue, &stealers);

        match task {
            Some(task) => {
                // Execute the task
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    task.execute();
                }));
            }
            None => {
                // No work available, park the thread
                let (lock, cvar) = &*parker;
                let mut has_work = lock.lock();

                // Double-check before parking
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                if !*has_work {
                    // Wait with timeout to periodically check for shutdown
                    let result = cvar.wait_for(&mut has_work, std::time::Duration::from_millis(10));
                    if result.timed_out() {
                        continue;
                    }
                }
                *has_work = false;
            }
        }
    }
}

/// Find a task from local queue, global queue, or by stealing
fn find_task(
    local: &Stealer<Task>,
    global: &Injector<Task>,
    stealers: &[Stealer<Task>],
) -> Option<Task> {
    // Try local queue first
    if let crossbeam::deque::Steal::Success(task) = local.steal() {
        return Some(task);
    }

    // Try global queue
    loop {
        match global.steal() {
            crossbeam::deque::Steal::Success(task) => return Some(task),
            crossbeam::deque::Steal::Empty => break,
            crossbeam::deque::Steal::Retry => continue,
        }
    }

    // Try stealing from other workers
    for stealer in stealers {
        loop {
            match stealer.steal() {
                crossbeam::deque::Steal::Success(task) => return Some(task),
                crossbeam::deque::Steal::Empty => break,
                crossbeam::deque::Steal::Retry => continue,
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_creation() {
        let worker = Worker::new(0);
        assert_eq!(worker.id, 0);
    }
}
