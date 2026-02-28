//! Cross-platform reactor trait

use crate::completion::Completion;
use crate::error::Result;
use crate::io_buffer::IoBuffer;
use crate::operation::IoOperation;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::io::RawFd;

/// Cross-platform reactor trait for async I/O operations.
///
/// This trait abstracts over platform-specific async I/O mechanisms:
/// - Linux: io_uring
/// - macOS: kqueue
/// - Windows: IOCP
///
/// # Example
///
/// ```ignore
/// use dx_py_reactor::{create_reactor, IoOperation, IoBuffer};
///
/// let mut reactor = create_reactor(0)?;
///
/// // Submit a read operation
/// let buf = IoBuffer::new(4096);
/// let op = IoOperation::Read {
///     fd: file_fd,
///     buf,
///     offset: 0,
///     user_data: 1,
/// };
/// reactor.submit(op)?;
///
/// // Wait for completion
/// let completions = reactor.wait(Duration::from_secs(1))?;
/// for completion in completions {
///     println!("Read {} bytes", completion.bytes());
/// }
/// ```
pub trait Reactor: Send + Sync {
    /// Submit a single I/O operation.
    ///
    /// Returns the user_data that will identify this operation in completions.
    fn submit(&mut self, op: IoOperation) -> Result<u64>;

    /// Submit multiple I/O operations in a single syscall (batched).
    ///
    /// This is more efficient than calling `submit` multiple times
    /// as it minimizes syscall overhead.
    ///
    /// Returns a vector of user_data values for each submitted operation.
    fn submit_batch(&mut self, ops: Vec<IoOperation>) -> Result<Vec<u64>>;

    /// Poll for completions without blocking.
    ///
    /// Returns all currently available completions.
    fn poll(&mut self) -> Vec<Completion>;

    /// Wait for completions with a timeout.
    ///
    /// Blocks until at least one completion is available or the timeout expires.
    fn wait(&mut self, timeout: Duration) -> Result<Vec<Completion>>;

    /// Wait for a specific number of completions.
    ///
    /// Blocks until at least `min_completions` are available or the timeout expires.
    fn wait_for(&mut self, min_completions: usize, timeout: Duration) -> Result<Vec<Completion>> {
        let mut all_completions = Vec::new();
        let start = std::time::Instant::now();

        while all_completions.len() < min_completions {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            let completions = self.wait(remaining)?;
            all_completions.extend(completions);
        }

        Ok(all_completions)
    }

    /// Register file descriptors for zero-copy operations.
    ///
    /// On Linux with io_uring, this enables the use of registered file
    /// descriptors which can improve performance by avoiding fd lookup
    /// in the kernel.
    ///
    /// On other platforms, this may be a no-op.
    #[cfg(unix)]
    fn register_files(&mut self, fds: &[RawFd]) -> Result<()>;

    /// Register buffers for zero-copy I/O.
    ///
    /// On Linux with io_uring, this enables the kernel to directly
    /// access these buffers without copying.
    ///
    /// On other platforms, this may be a no-op.
    fn register_buffers(&mut self, buffers: &[IoBuffer]) -> Result<()>;

    /// Unregister previously registered file descriptors.
    #[cfg(unix)]
    fn unregister_files(&mut self) -> Result<()> {
        Ok(()) // Default no-op
    }

    /// Unregister previously registered buffers.
    fn unregister_buffers(&mut self) -> Result<()> {
        Ok(()) // Default no-op
    }

    /// Get the number of pending operations.
    fn pending_count(&self) -> usize;

    /// Check if the reactor has pending operations.
    fn has_pending(&self) -> bool {
        self.pending_count() > 0
    }

    /// Wake up the reactor if it's blocked in wait().
    ///
    /// This is useful for signaling the reactor from another thread.
    fn wake(&self) -> Result<()>;

    /// Shutdown the reactor, cancelling all pending operations.
    fn shutdown(&mut self) -> Result<()>;

    /// Check if the reactor supports a specific feature.
    fn supports(&self, feature: ReactorFeature) -> bool;

    /// Get reactor statistics.
    fn stats(&self) -> ReactorStats {
        ReactorStats::default()
    }
}

/// Features that may or may not be supported by a reactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactorFeature {
    /// Zero-syscall submission (io_uring SQPOLL)
    ZeroSyscallSubmit,
    /// Multi-shot accept
    MultishotAccept,
    /// Zero-copy send
    ZeroCopySend,
    /// Registered file descriptors
    RegisteredFds,
    /// Registered buffers
    RegisteredBuffers,
    /// Buffer selection (kernel provides buffer)
    BufferSelection,
    /// Linked operations (chain of dependent ops)
    LinkedOperations,
    /// Timeout operations
    Timeouts,
    /// Operation cancellation
    Cancellation,
}

/// Statistics about reactor operations.
#[derive(Debug, Clone, Default)]
pub struct ReactorStats {
    /// Total operations submitted
    pub ops_submitted: u64,
    /// Total operations completed
    pub ops_completed: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Number of syscalls made
    pub syscalls: u64,
    /// Number of times poll returned empty
    pub empty_polls: u64,
}

impl ReactorStats {
    /// Calculate the average bytes per operation.
    pub fn avg_bytes_per_op(&self) -> f64 {
        if self.ops_completed == 0 {
            0.0
        } else {
            (self.bytes_read + self.bytes_written) as f64 / self.ops_completed as f64
        }
    }

    /// Calculate the average operations per syscall.
    pub fn ops_per_syscall(&self) -> f64 {
        if self.syscalls == 0 {
            0.0
        } else {
            self.ops_submitted as f64 / self.syscalls as f64
        }
    }
}

/// A reactor that wraps another reactor and collects statistics.
pub struct StatsReactor<R: Reactor> {
    inner: R,
    stats: ReactorStats,
}

impl<R: Reactor> StatsReactor<R> {
    /// Create a new stats-collecting reactor wrapper.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            stats: ReactorStats::default(),
        }
    }

    /// Get the inner reactor.
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Get the inner reactor mutably.
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Get the collected statistics.
    pub fn stats(&self) -> &ReactorStats {
        &self.stats
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ReactorStats::default();
    }
}

impl<R: Reactor> Reactor for StatsReactor<R> {
    fn submit(&mut self, op: IoOperation) -> Result<u64> {
        self.stats.ops_submitted += 1;
        self.stats.syscalls += 1;
        self.inner.submit(op)
    }

    fn submit_batch(&mut self, ops: Vec<IoOperation>) -> Result<Vec<u64>> {
        self.stats.ops_submitted += ops.len() as u64;
        self.stats.syscalls += 1;
        self.inner.submit_batch(ops)
    }

    fn poll(&mut self) -> Vec<Completion> {
        let completions = self.inner.poll();
        if completions.is_empty() {
            self.stats.empty_polls += 1;
        }
        for c in &completions {
            self.stats.ops_completed += 1;
            if let Ok(bytes) = c.result {
                // We don't know if it was read or write, so we just track total
                self.stats.bytes_read += bytes as u64;
            }
        }
        completions
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<Completion>> {
        self.stats.syscalls += 1;
        let completions = self.inner.wait(timeout)?;
        for c in &completions {
            self.stats.ops_completed += 1;
            if let Ok(bytes) = c.result {
                self.stats.bytes_read += bytes as u64;
            }
        }
        Ok(completions)
    }

    #[cfg(unix)]
    fn register_files(&mut self, fds: &[RawFd]) -> Result<()> {
        self.inner.register_files(fds)
    }

    fn register_buffers(&mut self, buffers: &[IoBuffer]) -> Result<()> {
        self.inner.register_buffers(buffers)
    }

    fn pending_count(&self) -> usize {
        self.inner.pending_count()
    }

    fn wake(&self) -> Result<()> {
        self.inner.wake()
    }

    fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown()
    }

    fn supports(&self, feature: ReactorFeature) -> bool {
        self.inner.supports(feature)
    }

    fn stats(&self) -> ReactorStats {
        self.stats.clone()
    }
}
