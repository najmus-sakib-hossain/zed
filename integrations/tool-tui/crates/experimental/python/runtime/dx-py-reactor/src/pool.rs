//! Reactor pool for thread-per-core architecture

use crate::error::Result;
use crate::reactor::Reactor;
use crate::{create_reactor, Completion, IoOperation};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

/// A pool of reactors, one per CPU core.
///
/// This enables the thread-per-core architecture where each worker thread
/// has its own reactor, eliminating contention and maximizing cache locality.
///
/// # Example
///
/// ```ignore
/// use dx_py_reactor::ReactorPool;
///
/// let pool = ReactorPool::new()?;
///
/// // Get reactor for current thread's core
/// let reactor = pool.get_reactor(0);
/// let mut reactor = reactor.lock();
///
/// // Submit operations
/// reactor.submit(op)?;
/// ```
pub struct ReactorPool {
    /// One reactor per core
    reactors: Vec<Arc<Mutex<Box<dyn Reactor>>>>,
    /// Number of cores
    num_cores: usize,
}

impl ReactorPool {
    /// Create a new reactor pool with one reactor per physical CPU core.
    pub fn new() -> Result<Self> {
        let num_cores = num_cpus::get_physical();
        Self::with_cores(num_cores)
    }

    /// Create a reactor pool with a specific number of reactors.
    pub fn with_cores(num_cores: usize) -> Result<Self> {
        let mut reactors = Vec::with_capacity(num_cores);

        for core_id in 0..num_cores {
            let reactor = create_reactor(core_id)?;
            reactors.push(Arc::new(Mutex::new(reactor)));
        }

        Ok(Self {
            reactors,
            num_cores,
        })
    }

    /// Get the reactor for a specific core.
    ///
    /// The core_id is taken modulo the number of cores, so any value is valid.
    pub fn get_reactor(&self, core_id: usize) -> Arc<Mutex<Box<dyn Reactor>>> {
        Arc::clone(&self.reactors[core_id % self.num_cores])
    }

    /// Get the reactor for the current thread.
    ///
    /// Uses the thread ID to select a reactor, distributing threads across cores.
    pub fn get_current_reactor(&self) -> Arc<Mutex<Box<dyn Reactor>>> {
        let thread_id = std::thread::current().id();
        let hash = format!("{:?}", thread_id)
            .bytes()
            .fold(0usize, |acc, b| acc.wrapping_add(b as usize));
        self.get_reactor(hash)
    }

    /// Get the number of reactors in the pool.
    pub fn num_reactors(&self) -> usize {
        self.num_cores
    }

    /// Submit an operation to a specific reactor.
    pub fn submit(&self, core_id: usize, op: IoOperation) -> Result<u64> {
        let reactor = self.get_reactor(core_id);
        let mut reactor = reactor.lock();
        reactor.submit(op)
    }

    /// Submit a batch of operations to a specific reactor.
    pub fn submit_batch(&self, core_id: usize, ops: Vec<IoOperation>) -> Result<Vec<u64>> {
        let reactor = self.get_reactor(core_id);
        let mut reactor = reactor.lock();
        reactor.submit_batch(ops)
    }

    /// Poll all reactors for completions.
    pub fn poll_all(&self) -> Vec<(usize, Vec<Completion>)> {
        self.reactors
            .iter()
            .enumerate()
            .map(|(i, reactor)| {
                let mut reactor = reactor.lock();
                (i, reactor.poll())
            })
            .filter(|(_, completions)| !completions.is_empty())
            .collect()
    }

    /// Shutdown all reactors.
    pub fn shutdown(&self) -> Result<()> {
        for reactor in &self.reactors {
            let mut reactor = reactor.lock();
            reactor.shutdown()?;
        }
        Ok(())
    }
}

/// A handle to a reactor in the pool.
///
/// This provides a more ergonomic interface for working with a single reactor.
pub struct ReactorHandle {
    reactor: Arc<Mutex<Box<dyn Reactor>>>,
    core_id: usize,
}

impl ReactorHandle {
    /// Create a new handle from a pool and core ID.
    pub fn new(pool: &ReactorPool, core_id: usize) -> Self {
        Self {
            reactor: pool.get_reactor(core_id),
            core_id,
        }
    }

    /// Get the core ID for this handle.
    pub fn core_id(&self) -> usize {
        self.core_id
    }

    /// Submit an operation.
    pub fn submit(&self, op: IoOperation) -> Result<u64> {
        let mut reactor = self.reactor.lock();
        reactor.submit(op)
    }

    /// Submit a batch of operations.
    pub fn submit_batch(&self, ops: Vec<IoOperation>) -> Result<Vec<u64>> {
        let mut reactor = self.reactor.lock();
        reactor.submit_batch(ops)
    }

    /// Poll for completions.
    pub fn poll(&self) -> Vec<Completion> {
        let mut reactor = self.reactor.lock();
        reactor.poll()
    }

    /// Wait for completions.
    pub fn wait(&self, timeout: Duration) -> Result<Vec<Completion>> {
        let mut reactor = self.reactor.lock();
        reactor.wait(timeout)
    }

    /// Get the number of pending operations.
    pub fn pending_count(&self) -> usize {
        let reactor = self.reactor.lock();
        reactor.pending_count()
    }
}

/// Builder for creating a ReactorPool with custom configuration.
pub struct ReactorPoolBuilder {
    num_cores: Option<usize>,
    use_sqpoll: bool,
}

impl ReactorPoolBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            num_cores: None,
            use_sqpoll: true,
        }
    }

    /// Set the number of cores (reactors) to create.
    pub fn num_cores(mut self, num_cores: usize) -> Self {
        self.num_cores = Some(num_cores);
        self
    }

    /// Enable or disable SQPOLL mode (Linux only).
    pub fn sqpoll(mut self, enabled: bool) -> Self {
        self.use_sqpoll = enabled;
        self
    }

    /// Build the reactor pool.
    pub fn build(self) -> Result<ReactorPool> {
        let num_cores = self.num_cores.unwrap_or_else(num_cpus::get_physical);

        if self.use_sqpoll {
            ReactorPool::with_cores(num_cores)
        } else {
            // Create basic reactors without SQPOLL
            let mut reactors = Vec::with_capacity(num_cores);
            for _ in 0..num_cores {
                let reactor = crate::create_basic_reactor()?;
                reactors.push(Arc::new(Mutex::new(reactor)));
            }
            Ok(ReactorPool {
                reactors,
                num_cores,
            })
        }
    }
}

impl Default for ReactorPoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactor_pool_creation() {
        // This test may fail on systems without io_uring/kqueue/IOCP support
        if let Ok(pool) = ReactorPool::with_cores(2) {
            assert_eq!(pool.num_reactors(), 2);
        }
    }

    #[test]
    fn test_reactor_handle() {
        if let Ok(pool) = ReactorPool::with_cores(2) {
            let handle = ReactorHandle::new(&pool, 0);
            assert_eq!(handle.core_id(), 0);
        }
    }

    #[test]
    fn test_reactor_pool_builder() {
        let builder = ReactorPoolBuilder::new().num_cores(4).sqpoll(false);

        if let Ok(pool) = builder.build() {
            assert_eq!(pool.num_reactors(), 4);
        }
    }
}
