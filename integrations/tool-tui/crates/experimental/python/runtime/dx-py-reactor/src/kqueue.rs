//! macOS/BSD kqueue reactor implementation
//!
//! This module provides an async I/O reactor using BSD's kqueue interface.
//! While not as feature-rich as io_uring, kqueue provides efficient event
//! notification for file descriptors.

#![cfg(target_os = "macos")]

use crate::completion::{Completion, CompletionFlags};
use crate::error::{ReactorError, Result};
use crate::io_buffer::IoBuffer;
use crate::operation::{Fd, IoOperation, PollEvents};
use crate::reactor::{Reactor, ReactorFeature, ReactorStats};

use std::collections::HashMap;
use std::io;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Default event buffer size
const DEFAULT_EVENT_BUFFER_SIZE: usize = 1024;

/// kqueue-based reactor for macOS/BSD.
pub struct KqueueReactor {
    /// kqueue file descriptor
    kq: RawFd,
    /// Event buffer for kevent calls
    events: Vec<libc::kevent>,
    /// Pending operations count
    pending: AtomicUsize,
    /// Next user_data ID
    next_id: AtomicU64,
    /// Shutdown flag
    shutdown: AtomicBool,
    /// Statistics
    stats: ReactorStats,
    /// Pending operation metadata
    pending_ops: HashMap<u64, PendingKqueueOp>,
}

/// Metadata for a pending kqueue operation
struct PendingKqueueOp {
    /// File descriptor
    fd: RawFd,
    /// Operation type
    op_type: KqueueOpType,
    /// Buffer for read/write operations
    #[allow(dead_code)]
    buf: Option<IoBuffer>,
    /// Offset for read/write operations
    offset: u64,
}

#[derive(Clone, Copy)]
enum KqueueOpType {
    Read,
    Write,
    Accept,
    Connect,
    Poll,
}

impl KqueueReactor {
    /// Create a new kqueue reactor.
    pub fn new() -> Result<Self> {
        let kq = unsafe { libc::kqueue() };
        if kq < 0 {
            return Err(ReactorError::Io(io::Error::last_os_error()));
        }

        Ok(Self {
            kq,
            events: vec![unsafe { std::mem::zeroed() }; DEFAULT_EVENT_BUFFER_SIZE],
            pending: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            shutdown: AtomicBool::new(false),
            stats: ReactorStats::default(),
            pending_ops: HashMap::new(),
        })
    }

    /// Get the next user_data ID.
    fn next_user_data(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register an event with kqueue.
    fn register_event(&self, fd: RawFd, filter: i16, flags: u16, user_data: u64) -> Result<()> {
        let changelist = [libc::kevent {
            ident: fd as usize,
            filter,
            flags,
            fflags: 0,
            data: 0,
            udata: user_data as *mut libc::c_void,
        }];

        let ret = unsafe {
            libc::kevent(self.kq, changelist.as_ptr(), 1, std::ptr::null_mut(), 0, std::ptr::null())
        };

        if ret < 0 {
            return Err(ReactorError::Io(io::Error::last_os_error()));
        }

        Ok(())
    }

    /// Process a completion for a read operation.
    fn complete_read(&mut self, user_data: u64, op: &PendingKqueueOp) -> Completion {
        // For kqueue, we need to perform the actual read here
        // since kqueue only notifies us that the fd is readable
        if let Some(ref buf) = op.buf {
            let result = unsafe {
                if op.offset > 0 {
                    libc::pread(
                        op.fd,
                        buf.as_ptr() as *mut libc::c_void,
                        buf.len(),
                        op.offset as i64,
                    )
                } else {
                    libc::read(op.fd, buf.as_ptr() as *mut libc::c_void, buf.len())
                }
            };

            if result >= 0 {
                Completion::success(user_data, result as usize)
            } else {
                Completion::error(user_data, io::Error::last_os_error())
            }
        } else {
            Completion::error(
                user_data,
                io::Error::new(io::ErrorKind::InvalidInput, "No buffer for read"),
            )
        }
    }

    /// Process a completion for a write operation.
    fn complete_write(&mut self, user_data: u64, op: &PendingKqueueOp) -> Completion {
        if let Some(ref buf) = op.buf {
            let result = unsafe {
                if op.offset > 0 {
                    libc::pwrite(
                        op.fd,
                        buf.as_ptr() as *const libc::c_void,
                        buf.len(),
                        op.offset as i64,
                    )
                } else {
                    libc::write(op.fd, buf.as_ptr() as *const libc::c_void, buf.len())
                }
            };

            if result >= 0 {
                Completion::success(user_data, result as usize)
            } else {
                Completion::error(user_data, io::Error::last_os_error())
            }
        } else {
            Completion::error(
                user_data,
                io::Error::new(io::ErrorKind::InvalidInput, "No buffer for write"),
            )
        }
    }

    /// Process a completion for an accept operation.
    fn complete_accept(&mut self, user_data: u64, op: &PendingKqueueOp) -> Completion {
        let result = unsafe { libc::accept(op.fd, std::ptr::null_mut(), std::ptr::null_mut()) };

        if result >= 0 {
            Completion::success(user_data, result as usize)
        } else {
            Completion::error(user_data, io::Error::last_os_error())
        }
    }
}

impl Reactor for KqueueReactor {
    fn submit(&mut self, op: IoOperation) -> Result<u64> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(ReactorError::Shutdown);
        }

        let user_data = op.user_data();

        match &op {
            IoOperation::Read {
                fd, buf, offset, ..
            } => {
                self.register_event(
                    *fd,
                    libc::EVFILT_READ,
                    (libc::EV_ADD | libc::EV_ONESHOT) as u16,
                    user_data,
                )?;
                self.pending_ops.insert(
                    user_data,
                    PendingKqueueOp {
                        fd: *fd,
                        op_type: KqueueOpType::Read,
                        buf: Some(buf.clone()),
                        offset: *offset,
                    },
                );
            }

            IoOperation::Write {
                fd, buf, offset, ..
            } => {
                self.register_event(
                    *fd,
                    libc::EVFILT_WRITE,
                    (libc::EV_ADD | libc::EV_ONESHOT) as u16,
                    user_data,
                )?;
                self.pending_ops.insert(
                    user_data,
                    PendingKqueueOp {
                        fd: *fd,
                        op_type: KqueueOpType::Write,
                        buf: Some(buf.clone()),
                        offset: *offset,
                    },
                );
            }

            IoOperation::Pread {
                fd, buf, offset, ..
            } => {
                self.register_event(
                    *fd,
                    libc::EVFILT_READ,
                    (libc::EV_ADD | libc::EV_ONESHOT) as u16,
                    user_data,
                )?;
                self.pending_ops.insert(
                    user_data,
                    PendingKqueueOp {
                        fd: *fd,
                        op_type: KqueueOpType::Read,
                        buf: Some(buf.clone()),
                        offset: *offset,
                    },
                );
            }

            IoOperation::Pwrite {
                fd, buf, offset, ..
            } => {
                self.register_event(
                    *fd,
                    libc::EVFILT_WRITE,
                    (libc::EV_ADD | libc::EV_ONESHOT) as u16,
                    user_data,
                )?;
                self.pending_ops.insert(
                    user_data,
                    PendingKqueueOp {
                        fd: *fd,
                        op_type: KqueueOpType::Write,
                        buf: Some(buf.clone()),
                        offset: *offset,
                    },
                );
            }

            IoOperation::Accept { fd, .. } => {
                self.register_event(
                    *fd,
                    libc::EVFILT_READ,
                    (libc::EV_ADD | libc::EV_ONESHOT) as u16,
                    user_data,
                )?;
                self.pending_ops.insert(
                    user_data,
                    PendingKqueueOp {
                        fd: *fd,
                        op_type: KqueueOpType::Accept,
                        buf: None,
                        offset: 0,
                    },
                );
            }

            IoOperation::Poll { fd, events, .. } => {
                let filter = if events.contains(PollEvents::READABLE) {
                    libc::EVFILT_READ
                } else {
                    libc::EVFILT_WRITE
                };

                self.register_event(
                    *fd,
                    filter,
                    (libc::EV_ADD | libc::EV_ONESHOT) as u16,
                    user_data,
                )?;
                self.pending_ops.insert(
                    user_data,
                    PendingKqueueOp {
                        fd: *fd,
                        op_type: KqueueOpType::Poll,
                        buf: None,
                        offset: 0,
                    },
                );
            }

            // Operations not directly supported by kqueue
            IoOperation::AcceptMulti { .. } => {
                return Err(ReactorError::unsupported("AcceptMulti not supported on kqueue"));
            }
            IoOperation::SendZeroCopy { .. } => {
                return Err(ReactorError::unsupported("SendZeroCopy not supported on kqueue"));
            }
            IoOperation::Close { fd, .. } => {
                // Close is synchronous
                let result = unsafe { libc::close(*fd) };
                if result < 0 {
                    return Err(ReactorError::Io(io::Error::last_os_error()));
                }
                // Return immediately with a fake completion
                return Ok(user_data);
            }
            IoOperation::Fsync { fd, .. } => {
                // Fsync is synchronous on kqueue
                let result = unsafe { libc::fsync(*fd) };
                if result < 0 {
                    return Err(ReactorError::Io(io::Error::last_os_error()));
                }
                return Ok(user_data);
            }
            _ => {
                return Err(ReactorError::unsupported("Operation not supported on kqueue"));
            }
        }

        self.pending.fetch_add(1, Ordering::Relaxed);
        self.stats.ops_submitted += 1;
        self.stats.syscalls += 1;

        Ok(user_data)
    }

    fn submit_batch(&mut self, ops: Vec<IoOperation>) -> Result<Vec<u64>> {
        let mut user_datas = Vec::with_capacity(ops.len());

        for op in ops {
            let user_data = self.submit(op)?;
            user_datas.push(user_data);
        }

        Ok(user_datas)
    }

    fn poll(&mut self) -> Vec<Completion> {
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let n = unsafe {
            libc::kevent(
                self.kq,
                std::ptr::null(),
                0,
                self.events.as_mut_ptr(),
                self.events.len() as i32,
                &timeout,
            )
        };

        if n <= 0 {
            return Vec::new();
        }

        let mut completions = Vec::with_capacity(n as usize);

        for i in 0..n as usize {
            let event = &self.events[i];
            let user_data = event.udata as u64;

            let completion = if event.flags as i32 & libc::EV_ERROR != 0 {
                Completion::from_raw_error(user_data, event.data as i32)
            } else if let Some(op) = self.pending_ops.remove(&user_data) {
                match op.op_type {
                    KqueueOpType::Read => self.complete_read(user_data, &op),
                    KqueueOpType::Write => self.complete_write(user_data, &op),
                    KqueueOpType::Accept => self.complete_accept(user_data, &op),
                    KqueueOpType::Connect | KqueueOpType::Poll => {
                        Completion::success(user_data, event.data as usize)
                    }
                }
            } else {
                Completion::success(user_data, event.data as usize)
            };

            self.pending.fetch_sub(1, Ordering::Relaxed);
            self.stats.ops_completed += 1;
            completions.push(completion);
        }

        completions
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<Completion>> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(ReactorError::Shutdown);
        }

        let timeout = libc::timespec {
            tv_sec: timeout.as_secs() as i64,
            tv_nsec: timeout.subsec_nanos() as i64,
        };

        let n = unsafe {
            libc::kevent(
                self.kq,
                std::ptr::null(),
                0,
                self.events.as_mut_ptr(),
                self.events.len() as i32,
                &timeout,
            )
        };

        self.stats.syscalls += 1;

        if n < 0 {
            return Err(ReactorError::Io(io::Error::last_os_error()));
        }

        if n == 0 {
            return Ok(Vec::new());
        }

        let mut completions = Vec::with_capacity(n as usize);

        for i in 0..n as usize {
            let event = &self.events[i];
            let user_data = event.udata as u64;

            let completion = if event.flags as i32 & libc::EV_ERROR != 0 {
                Completion::from_raw_error(user_data, event.data as i32)
            } else if let Some(op) = self.pending_ops.remove(&user_data) {
                match op.op_type {
                    KqueueOpType::Read => self.complete_read(user_data, &op),
                    KqueueOpType::Write => self.complete_write(user_data, &op),
                    KqueueOpType::Accept => self.complete_accept(user_data, &op),
                    KqueueOpType::Connect | KqueueOpType::Poll => {
                        Completion::success(user_data, event.data as usize)
                    }
                }
            } else {
                Completion::success(user_data, event.data as usize)
            };

            self.pending.fetch_sub(1, Ordering::Relaxed);
            self.stats.ops_completed += 1;
            completions.push(completion);
        }

        Ok(completions)
    }

    fn register_files(&mut self, _fds: &[RawFd]) -> Result<()> {
        // kqueue doesn't have file registration like io_uring
        Ok(())
    }

    fn register_buffers(&mut self, _buffers: &[IoBuffer]) -> Result<()> {
        // kqueue doesn't have buffer registration
        Ok(())
    }

    fn pending_count(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }

    fn wake(&self) -> Result<()> {
        // Use a user event to wake up kqueue
        let changelist = [libc::kevent {
            ident: 0,
            filter: libc::EVFILT_USER,
            flags: (libc::EV_ADD | libc::EV_ONESHOT) as u16,
            fflags: libc::NOTE_TRIGGER,
            data: 0,
            udata: std::ptr::null_mut(),
        }];

        let ret = unsafe {
            libc::kevent(self.kq, changelist.as_ptr(), 1, std::ptr::null_mut(), 0, std::ptr::null())
        };

        if ret < 0 {
            return Err(ReactorError::Io(io::Error::last_os_error()));
        }

        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.shutdown.store(true, Ordering::Relaxed);
        unsafe {
            libc::close(self.kq);
        }
        Ok(())
    }

    fn supports(&self, feature: ReactorFeature) -> bool {
        match feature {
            ReactorFeature::ZeroSyscallSubmit => false,
            ReactorFeature::MultishotAccept => false,
            ReactorFeature::ZeroCopySend => false,
            ReactorFeature::RegisteredFds => false,
            ReactorFeature::RegisteredBuffers => false,
            ReactorFeature::BufferSelection => false,
            ReactorFeature::LinkedOperations => false,
            ReactorFeature::Timeouts => true,
            ReactorFeature::Cancellation => false,
        }
    }

    fn stats(&self) -> ReactorStats {
        self.stats.clone()
    }
}

impl Drop for KqueueReactor {
    fn drop(&mut self) {
        if !self.shutdown.load(Ordering::Relaxed) {
            unsafe {
                libc::close(self.kq);
            }
        }
    }
}

// Safety: KqueueReactor is Send because kqueue fd is just an integer
unsafe impl Send for KqueueReactor {}

// Safety: KqueueReactor is Sync because we use atomic operations for shared state
unsafe impl Sync for KqueueReactor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kqueue_reactor_creation() {
        let reactor = KqueueReactor::new();
        assert!(reactor.is_ok());
    }

    #[test]
    fn test_kqueue_supports_features() {
        let reactor = KqueueReactor::new().unwrap();

        assert!(!reactor.supports(ReactorFeature::ZeroSyscallSubmit));
        assert!(!reactor.supports(ReactorFeature::MultishotAccept));
        assert!(!reactor.supports(ReactorFeature::ZeroCopySend));
        assert!(reactor.supports(ReactorFeature::Timeouts));
    }
}
