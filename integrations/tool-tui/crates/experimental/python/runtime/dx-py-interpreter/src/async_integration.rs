//! Async Integration for the Interpreter
//!
//! This module wires the interpreter to the async reactor, enabling
//! async/await support in Python code.

use dx_py_reactor::{IoOperation, ReactorPool, ReactorStats};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Async runtime integration for the interpreter
pub struct AsyncRuntime {
    /// The reactor pool for async I/O
    pool: Option<Arc<ReactorPool>>,
    /// Pending futures tracking
    pending: Mutex<Vec<PendingOperation>>,
    /// Whether async is enabled
    enabled: bool,
    /// Event loop running flag
    running: Mutex<bool>,
    /// Next operation ID
    next_id: AtomicU64,
}

/// A pending operation waiting for completion
struct PendingOperation {
    /// Operation ID
    id: u64,
    /// User data returned from reactor submit
    user_data: u64,
    /// Core ID where the operation was submitted
    core_id: usize,
    /// Callback to invoke on completion (reserved for future use)
    #[allow(dead_code)]
    callback: Option<Box<dyn FnOnce(FutureResult) + Send>>,
}

/// Result of a future completion
#[derive(Debug)]
pub enum FutureResult {
    /// Successful completion with value
    Ok(Vec<u8>),
    /// Error completion
    Err(AsyncError),
    /// Cancelled
    Cancelled,
}

impl AsyncRuntime {
    /// Create a new async runtime
    pub fn new() -> Self {
        Self {
            pool: None,
            pending: Mutex::new(Vec::new()),
            enabled: false,
            running: Mutex::new(false),
            next_id: AtomicU64::new(1),
        }
    }

    /// Generate a new operation ID
    fn next_op_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Initialize the reactor pool
    pub fn init(&mut self) -> Result<(), AsyncError> {
        if self.pool.is_some() {
            return Err(AsyncError::AlreadyInitialized);
        }

        // ReactorPool::new() creates one reactor per physical CPU core
        let pool = ReactorPool::new().map_err(|e| AsyncError::InitFailed(e.to_string()))?;

        self.pool = Some(Arc::new(pool));
        self.enabled = true;
        Ok(())
    }

    /// Initialize with a specific number of reactors
    pub fn init_with_cores(&mut self, num_cores: usize) -> Result<(), AsyncError> {
        if self.pool.is_some() {
            return Err(AsyncError::AlreadyInitialized);
        }

        let pool = ReactorPool::with_cores(num_cores)
            .map_err(|e| AsyncError::InitFailed(e.to_string()))?;

        self.pool = Some(Arc::new(pool));
        self.enabled = true;
        Ok(())
    }

    /// Check if async is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start the event loop
    pub fn start(&self) -> Result<(), AsyncError> {
        if !self.enabled {
            return Err(AsyncError::NotInitialized);
        }

        let mut running = self.running.lock();
        if *running {
            return Err(AsyncError::AlreadyRunning);
        }

        *running = true;
        Ok(())
    }

    /// Stop the event loop
    pub fn stop(&self) {
        let mut running = self.running.lock();
        *running = false;
    }

    /// Check if event loop is running
    pub fn is_running(&self) -> bool {
        *self.running.lock()
    }

    /// Submit an async file read operation
    ///
    /// Note: This is a simplified implementation. The actual file read
    /// requires opening the file first and using the file descriptor.
    /// For now, this returns NotImplemented as the ReactorPool doesn't
    /// have high-level file read methods - it works with IoOperation.
    pub fn read_file(&self, _path: &str) -> Result<u64, AsyncError> {
        let _pool = self.pool.as_ref().ok_or(AsyncError::NotInitialized)?;

        // ReactorPool works with IoOperation which requires file descriptors.
        // High-level file operations would need to:
        // 1. Open the file to get a file descriptor
        // 2. Create an IoBuffer
        // 3. Submit an IoOperation::Read
        //
        // For now, return NotImplemented as this requires platform-specific
        // file handling that's beyond the scope of this compilation fix.
        Err(AsyncError::NotImplemented(
            "read_file requires file descriptor handling".to_string(),
        ))
    }

    /// Submit an async file write operation
    ///
    /// Note: This is a simplified implementation. See read_file for details.
    pub fn write_file(&self, _path: &str, _data: &[u8]) -> Result<u64, AsyncError> {
        let _pool = self.pool.as_ref().ok_or(AsyncError::NotInitialized)?;

        // Same as read_file - requires file descriptor handling
        Err(AsyncError::NotImplemented(
            "write_file requires file descriptor handling".to_string(),
        ))
    }

    /// Submit a raw I/O operation to the reactor pool
    pub fn submit_operation(&self, core_id: usize, op: IoOperation) -> Result<u64, AsyncError> {
        let pool = self.pool.as_ref().ok_or(AsyncError::NotInitialized)?;

        let op_id = self.next_op_id();

        let user_data =
            pool.submit(core_id, op).map_err(|e| AsyncError::SubmitFailed(e.to_string()))?;

        self.pending.lock().push(PendingOperation {
            id: op_id,
            user_data,
            core_id,
            callback: None,
        });

        Ok(op_id)
    }

    /// Submit a batch of I/O operations
    pub fn submit_batch(
        &self,
        core_id: usize,
        ops: Vec<IoOperation>,
    ) -> Result<Vec<u64>, AsyncError> {
        let pool = self.pool.as_ref().ok_or(AsyncError::NotInitialized)?;

        let user_datas = pool
            .submit_batch(core_id, ops)
            .map_err(|e| AsyncError::SubmitFailed(e.to_string()))?;

        let mut op_ids = Vec::with_capacity(user_datas.len());
        let mut pending = self.pending.lock();

        for user_data in user_datas {
            let op_id = self.next_op_id();
            pending.push(PendingOperation {
                id: op_id,
                user_data,
                core_id,
                callback: None,
            });
            op_ids.push(op_id);
        }

        Ok(op_ids)
    }

    /// Poll for completed operations
    pub fn poll(&self) -> Vec<(u64, FutureResult)> {
        let pool = match self.pool.as_ref() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Poll all reactors for completions
        let all_completions = pool.poll_all();

        let mut pending = self.pending.lock();
        let mut completed = Vec::new();

        for (core_id, completions) in all_completions {
            for completion in completions {
                // Find the pending operation by user_data and core_id
                if let Some(pos) = pending
                    .iter()
                    .position(|p| p.user_data == completion.user_data && p.core_id == core_id)
                {
                    let pending_op = pending.remove(pos);
                    let result = match completion.result {
                        Ok(bytes_transferred) => {
                            // For now, return the byte count as a simple result
                            // In a full implementation, we'd return the actual data
                            FutureResult::Ok(bytes_transferred.to_le_bytes().to_vec())
                        }
                        Err(e) => FutureResult::Err(AsyncError::IoError(e.to_string())),
                    };
                    completed.push((pending_op.id, result));
                }
            }
        }

        completed
    }

    /// Wait for a specific operation to complete
    pub fn wait(&self, op_id: u64) -> Result<FutureResult, AsyncError> {
        loop {
            let completed = self.poll();
            for (id, result) in completed {
                if id == op_id {
                    return Ok(result);
                }
            }

            // Small sleep to avoid busy waiting
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    }

    /// Cancel a pending operation
    ///
    /// Note: Actual cancellation depends on reactor support.
    /// This removes the operation from our tracking.
    pub fn cancel(&self, op_id: u64) -> bool {
        let mut pending = self.pending.lock();
        if let Some(pos) = pending.iter().position(|p| p.id == op_id) {
            pending.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get the number of pending operations
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }

    /// Get reactor statistics
    ///
    /// Returns aggregated stats from all reactors in the pool.
    pub fn stats(&self) -> Option<ReactorStats> {
        let pool = self.pool.as_ref()?;

        // Aggregate stats from all reactors
        let mut total_stats = ReactorStats::default();

        for i in 0..pool.num_reactors() {
            let reactor = pool.get_reactor(i);
            let reactor_guard = reactor.lock();
            let stats = reactor_guard.stats();

            total_stats.ops_submitted += stats.ops_submitted;
            total_stats.ops_completed += stats.ops_completed;
            total_stats.bytes_read += stats.bytes_read;
            total_stats.bytes_written += stats.bytes_written;
            total_stats.syscalls += stats.syscalls;
            total_stats.empty_polls += stats.empty_polls;
        }

        Some(total_stats)
    }

    /// Shutdown the async runtime
    pub fn shutdown(&mut self) {
        self.stop();

        // Clear all pending operations
        self.pending.lock().clear();

        // Shutdown the reactor pool
        if let Some(pool) = self.pool.take() {
            let _ = pool.shutdown();
        }

        self.enabled = false;
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AsyncRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Async runtime errors
#[derive(Debug, thiserror::Error)]
pub enum AsyncError {
    #[error("Async runtime not initialized")]
    NotInitialized,

    #[error("Async runtime already initialized")]
    AlreadyInitialized,

    #[error("Event loop already running")]
    AlreadyRunning,

    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Submit failed: {0}")]
    SubmitFailed(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Future not found")]
    FutureNotFound,

    #[error("Timeout")]
    Timeout,

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_runtime_creation() {
        let runtime = AsyncRuntime::new();
        assert!(!runtime.is_enabled());
        assert!(!runtime.is_running());
    }

    #[test]
    fn test_not_initialized_error() {
        let runtime = AsyncRuntime::new();
        assert!(matches!(runtime.read_file("test.txt"), Err(AsyncError::NotInitialized)));
    }

    #[test]
    fn test_pending_count() {
        let runtime = AsyncRuntime::new();
        assert_eq!(runtime.pending_count(), 0);
    }

    #[test]
    fn test_start_without_init() {
        let runtime = AsyncRuntime::new();
        assert!(matches!(runtime.start(), Err(AsyncError::NotInitialized)));
    }

    #[test]
    fn test_cancel_nonexistent() {
        let runtime = AsyncRuntime::new();
        assert!(!runtime.cancel(999));
    }
}
