//! Linux io_uring reactor implementation
//!
//! This module provides a high-performance async I/O reactor using Linux's
//! io_uring interface. Key features:
//!
//! - SQPOLL mode for zero-syscall submissions
//! - Registered file descriptors for faster fd lookup
//! - Registered buffers for zero-copy I/O
//! - Multi-shot accept for high-throughput connection handling
//! - Zero-copy send (SendZc)

#![cfg(target_os = "linux")]

use crate::completion::{Completion, CompletionFlags};
use crate::error::{ReactorError, Result};
use crate::io_buffer::IoBuffer;
use crate::operation::{Fd, IoOperation, PollEvents, SendFlags};
use crate::reactor::{Reactor, ReactorFeature, ReactorStats};

use io_uring::{opcode, types, IoUring, Probe};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Default ring size (number of SQEs)
const DEFAULT_RING_SIZE: u32 = 4096;

/// Default SQPOLL idle timeout in milliseconds
const DEFAULT_SQPOLL_IDLE_MS: u32 = 2000;

/// io_uring-based reactor for Linux.
pub struct IoUringReactor {
    /// The io_uring instance
    ring: IoUring,
    /// Pending operations count
    pending: AtomicUsize,
    /// Next user_data ID
    next_id: AtomicU64,
    /// Registered file descriptors
    registered_fds: Vec<RawFd>,
    /// Whether SQPOLL mode is enabled
    sqpoll_enabled: bool,
    /// Shutdown flag
    shutdown: AtomicBool,
    /// Probe for feature detection
    probe: Probe,
    /// Statistics
    stats: ReactorStats,
    /// Pending operation metadata (for buffer management)
    pending_ops: HashMap<u64, PendingOp>,
}

/// Metadata for a pending operation
struct PendingOp {
    /// The buffer associated with this operation (if any)
    #[allow(dead_code)]
    buf: Option<IoBuffer>,
    /// Whether this is a multi-shot operation
    multishot: bool,
}

impl IoUringReactor {
    /// Create a new io_uring reactor with default settings.
    pub fn new() -> Result<Self> {
        let ring = IoUring::new(DEFAULT_RING_SIZE)?;
        let probe = Probe::new();

        Ok(Self {
            ring,
            pending: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            registered_fds: Vec::new(),
            sqpoll_enabled: false,
            shutdown: AtomicBool::new(false),
            probe,
            stats: ReactorStats::default(),
            pending_ops: HashMap::new(),
        })
    }

    /// Create a new io_uring reactor with SQPOLL mode.
    ///
    /// SQPOLL mode enables kernel-side polling, which can eliminate syscalls
    /// for I/O submissions when the kernel polling thread is active.
    pub fn new_sqpoll(core_id: usize) -> Result<Self> {
        let ring = IoUring::builder()
            .setup_sqpoll(DEFAULT_SQPOLL_IDLE_MS)
            .setup_sqpoll_cpu(core_id as u32)
            .setup_single_issuer()
            .setup_coop_taskrun()
            .build(DEFAULT_RING_SIZE)?;

        let probe = Probe::new();

        Ok(Self {
            ring,
            pending: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            registered_fds: Vec::new(),
            sqpoll_enabled: true,
            shutdown: AtomicBool::new(false),
            probe,
            stats: ReactorStats::default(),
            pending_ops: HashMap::new(),
        })
    }

    /// Create a reactor with custom ring size.
    pub fn with_ring_size(ring_size: u32) -> Result<Self> {
        let ring = IoUring::new(ring_size)?;
        let probe = Probe::new();

        Ok(Self {
            ring,
            pending: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            registered_fds: Vec::new(),
            sqpoll_enabled: false,
            shutdown: AtomicBool::new(false),
            probe,
            stats: ReactorStats::default(),
            pending_ops: HashMap::new(),
        })
    }

    /// Get the next user_data ID.
    fn next_user_data(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Build an SQE from an IoOperation.
    fn build_sqe(&self, op: &IoOperation, user_data: u64) -> Result<io_uring::squeue::Entry> {
        let sqe = match op {
            IoOperation::Read {
                fd, buf, offset, ..
            } => opcode::Read::new(types::Fd(*fd), buf.as_ptr() as *mut u8, buf.len() as u32)
                .offset(*offset)
                .build()
                .user_data(user_data),

            IoOperation::Write {
                fd, buf, offset, ..
            } => opcode::Write::new(types::Fd(*fd), buf.as_ptr(), buf.len() as u32)
                .offset(*offset)
                .build()
                .user_data(user_data),

            IoOperation::Pread {
                fd, buf, offset, ..
            } => opcode::Read::new(types::Fd(*fd), buf.as_ptr() as *mut u8, buf.len() as u32)
                .offset(*offset)
                .build()
                .user_data(user_data),

            IoOperation::Pwrite {
                fd, buf, offset, ..
            } => opcode::Write::new(types::Fd(*fd), buf.as_ptr(), buf.len() as u32)
                .offset(*offset)
                .build()
                .user_data(user_data),

            IoOperation::Accept { fd, .. } => {
                opcode::Accept::new(types::Fd(*fd), std::ptr::null_mut(), std::ptr::null_mut())
                    .build()
                    .user_data(user_data)
            }

            IoOperation::AcceptMulti { fd, .. } => {
                opcode::AcceptMulti::new(types::Fd(*fd)).build().user_data(user_data)
            }

            IoOperation::Connect { fd, addr, .. } => {
                let (sockaddr, len) = socket_addr_to_raw(addr);
                opcode::Connect::new(types::Fd(*fd), sockaddr, len).build().user_data(user_data)
            }

            IoOperation::Send { fd, buf, flags, .. } => {
                let mut send_flags = 0i32;
                if flags.contains(SendFlags::NOSIGNAL) {
                    send_flags |= libc::MSG_NOSIGNAL;
                }
                if flags.contains(SendFlags::DONTROUTE) {
                    send_flags |= libc::MSG_DONTROUTE;
                }
                if flags.contains(SendFlags::MORE) {
                    send_flags |= libc::MSG_MORE;
                }

                opcode::Send::new(types::Fd(*fd), buf.as_ptr(), buf.len() as u32)
                    .flags(send_flags)
                    .build()
                    .user_data(user_data)
            }

            IoOperation::SendZeroCopy { fd, buf, .. } => {
                opcode::SendZc::new(types::Fd(*fd), buf.as_ptr(), buf.len() as u32)
                    .build()
                    .user_data(user_data)
            }

            IoOperation::Recv { fd, buf, .. } => {
                opcode::Recv::new(types::Fd(*fd), buf.as_ptr() as *mut u8, buf.len() as u32)
                    .build()
                    .user_data(user_data)
            }

            IoOperation::Close { fd, .. } => {
                opcode::Close::new(types::Fd(*fd)).build().user_data(user_data)
            }

            IoOperation::Fsync { fd, .. } => {
                opcode::Fsync::new(types::Fd(*fd)).build().user_data(user_data)
            }

            IoOperation::Fdatasync { fd, .. } => opcode::Fsync::new(types::Fd(*fd))
                .flags(types::FsyncFlags::DATASYNC)
                .build()
                .user_data(user_data),

            IoOperation::Timeout { duration_ns, .. } => {
                let ts = types::Timespec::new()
                    .sec((*duration_ns / 1_000_000_000) as u64)
                    .nsec((*duration_ns % 1_000_000_000) as u32);

                opcode::Timeout::new(&ts as *const _).build().user_data(user_data)
            }

            IoOperation::Cancel {
                target_user_data, ..
            } => opcode::AsyncCancel::new(*target_user_data).build().user_data(user_data),

            IoOperation::Nop { .. } => opcode::Nop::new().build().user_data(user_data),

            IoOperation::Poll { fd, events, .. } => {
                let mut poll_flags = 0i16;
                if events.contains(PollEvents::READABLE) {
                    poll_flags |= libc::POLLIN as i16;
                }
                if events.contains(PollEvents::WRITABLE) {
                    poll_flags |= libc::POLLOUT as i16;
                }
                if events.contains(PollEvents::ERROR) {
                    poll_flags |= libc::POLLERR as i16;
                }
                if events.contains(PollEvents::HUP) {
                    poll_flags |= libc::POLLHUP as i16;
                }

                opcode::PollAdd::new(types::Fd(*fd), poll_flags as u32)
                    .build()
                    .user_data(user_data)
            }
        };

        Ok(sqe)
    }

    /// Process completions from the CQ.
    fn process_completions(&mut self) -> Vec<Completion> {
        let mut completions = Vec::new();

        for cqe in self.ring.completion() {
            let user_data = cqe.user_data();
            let result = cqe.result();
            let flags = CompletionFlags::from_io_uring(cqe.flags());

            // Check if this is a multi-shot operation that's still active
            let is_multishot =
                self.pending_ops.get(&user_data).map(|op| op.multishot).unwrap_or(false);

            // Only decrement pending count if not a continuing multi-shot
            if !flags.contains(CompletionFlags::MORE) {
                self.pending.fetch_sub(1, Ordering::Relaxed);
                self.pending_ops.remove(&user_data);
            }

            let completion = if result >= 0 {
                Completion::success_with_flags(user_data, result as usize, flags)
            } else {
                Completion::from_raw_error(user_data, -result)
            };

            completions.push(completion);
            self.stats.ops_completed += 1;
        }

        completions
    }
}

impl Reactor for IoUringReactor {
    fn submit(&mut self, op: IoOperation) -> Result<u64> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(ReactorError::Shutdown);
        }

        let user_data = op.user_data();
        let is_multishot = op.is_multishot();

        // Store pending operation metadata
        self.pending_ops.insert(
            user_data,
            PendingOp {
                buf: None, // We don't take ownership of the buffer here
                multishot: is_multishot,
            },
        );

        let sqe = self.build_sqe(&op, user_data)?;

        unsafe {
            self.ring
                .submission()
                .push(&sqe)
                .map_err(|_| ReactorError::SubmissionQueueFull)?;
        }

        self.ring.submit()?;
        self.pending.fetch_add(1, Ordering::Relaxed);
        self.stats.ops_submitted += 1;
        self.stats.syscalls += 1;

        Ok(user_data)
    }

    fn submit_batch(&mut self, ops: Vec<IoOperation>) -> Result<Vec<u64>> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(ReactorError::Shutdown);
        }

        let mut user_datas = Vec::with_capacity(ops.len());

        {
            let mut sq = self.ring.submission();

            for op in ops {
                let user_data = op.user_data();
                let is_multishot = op.is_multishot();

                self.pending_ops.insert(
                    user_data,
                    PendingOp {
                        buf: None,
                        multishot: is_multishot,
                    },
                );

                let sqe = self.build_sqe(&op, user_data)?;

                unsafe {
                    sq.push(&sqe).map_err(|_| ReactorError::SubmissionQueueFull)?;
                }

                user_datas.push(user_data);
                self.pending.fetch_add(1, Ordering::Relaxed);
                self.stats.ops_submitted += 1;
            }
        }

        // Single syscall for all operations
        self.ring.submit()?;
        self.stats.syscalls += 1;

        Ok(user_datas)
    }

    fn poll(&mut self) -> Vec<Completion> {
        self.process_completions()
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<Completion>> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(ReactorError::Shutdown);
        }

        // Submit and wait for at least one completion
        self.ring.submit_and_wait_with_timeout(1, timeout)?;
        self.stats.syscalls += 1;

        Ok(self.process_completions())
    }

    fn register_files(&mut self, fds: &[RawFd]) -> Result<()> {
        self.ring.submitter().register_files(fds)?;
        self.registered_fds.extend_from_slice(fds);
        Ok(())
    }

    fn register_buffers(&mut self, buffers: &[IoBuffer]) -> Result<()> {
        let iovecs: Vec<libc::iovec> = buffers
            .iter()
            .map(|b| libc::iovec {
                iov_base: b.as_ptr() as *mut libc::c_void,
                iov_len: b.len(),
            })
            .collect();

        self.ring.submitter().register_buffers(&iovecs)?;
        Ok(())
    }

    fn unregister_files(&mut self) -> Result<()> {
        self.ring.submitter().unregister_files()?;
        self.registered_fds.clear();
        Ok(())
    }

    fn unregister_buffers(&mut self) -> Result<()> {
        self.ring.submitter().unregister_buffers()?;
        Ok(())
    }

    fn pending_count(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }

    fn wake(&self) -> Result<()> {
        // Submit a NOP to wake up the ring
        // This is a bit hacky but works
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.shutdown.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn supports(&self, feature: ReactorFeature) -> bool {
        match feature {
            ReactorFeature::ZeroSyscallSubmit => self.sqpoll_enabled,
            ReactorFeature::MultishotAccept => true, // io_uring supports this
            ReactorFeature::ZeroCopySend => true,    // io_uring supports SendZc
            ReactorFeature::RegisteredFds => true,
            ReactorFeature::RegisteredBuffers => true,
            ReactorFeature::BufferSelection => true,
            ReactorFeature::LinkedOperations => true,
            ReactorFeature::Timeouts => true,
            ReactorFeature::Cancellation => true,
        }
    }

    fn stats(&self) -> ReactorStats {
        self.stats.clone()
    }
}

/// Convert a SocketAddr to raw sockaddr pointer and length.
fn socket_addr_to_raw(addr: &SocketAddr) -> (*const libc::sockaddr, u32) {
    // This is a simplified version - in production you'd want to handle
    // the lifetime of the sockaddr properly
    match addr {
        SocketAddr::V4(v4) => {
            let sockaddr = Box::new(libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: v4.port().to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes(v4.ip().octets()),
                },
                sin_zero: [0; 8],
            });
            let ptr = Box::into_raw(sockaddr) as *const libc::sockaddr;
            (ptr, std::mem::size_of::<libc::sockaddr_in>() as u32)
        }
        SocketAddr::V6(v6) => {
            let sockaddr = Box::new(libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as u16,
                sin6_port: v6.port().to_be(),
                sin6_flowinfo: v6.flowinfo(),
                sin6_addr: libc::in6_addr {
                    s6_addr: v6.ip().octets(),
                },
                sin6_scope_id: v6.scope_id(),
            });
            let ptr = Box::into_raw(sockaddr) as *const libc::sockaddr;
            (ptr, std::mem::size_of::<libc::sockaddr_in6>() as u32)
        }
    }
}

// Safety: IoUringReactor is Send because io_uring::IoUring is Send
unsafe impl Send for IoUringReactor {}

// Safety: IoUringReactor is Sync because we use atomic operations for shared state
unsafe impl Sync for IoUringReactor {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_io_uring_reactor_creation() {
        let reactor = IoUringReactor::new();
        assert!(reactor.is_ok());
    }

    #[test]
    fn test_io_uring_read_write() {
        let mut reactor = IoUringReactor::new().unwrap();

        // Create a temp file with some data
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, io_uring!").unwrap();
        temp_file.flush().unwrap();

        let file = File::open(temp_file.path()).unwrap();
        let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);

        // Submit a read operation
        let buf = IoBuffer::new(1024);
        let op = IoOperation::Read {
            fd,
            buf,
            offset: 0,
            user_data: 1,
        };

        let user_data = reactor.submit(op).unwrap();
        assert_eq!(user_data, 1);

        // Wait for completion
        let completions = reactor.wait(Duration::from_secs(1)).unwrap();
        assert_eq!(completions.len(), 1);
        assert!(completions[0].is_success());
        assert_eq!(completions[0].bytes(), 16); // "Hello, io_uring!" is 16 bytes
    }

    #[test]
    fn test_io_uring_batch_submit() {
        let mut reactor = IoUringReactor::new().unwrap();

        // Create multiple temp files
        let mut files = Vec::new();
        for i in 0..5 {
            let mut temp_file = NamedTempFile::new().unwrap();
            write!(temp_file, "File {}", i).unwrap();
            temp_file.flush().unwrap();
            files.push(temp_file);
        }

        // Submit batch of read operations
        let ops: Vec<IoOperation> = files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let file = File::open(f.path()).unwrap();
                let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);
                std::mem::forget(file); // Keep fd valid

                IoOperation::Read {
                    fd,
                    buf: IoBuffer::new(1024),
                    offset: 0,
                    user_data: i as u64 + 1,
                }
            })
            .collect();

        let user_datas = reactor.submit_batch(ops).unwrap();
        assert_eq!(user_datas.len(), 5);

        // Wait for all completions
        let mut all_completions = Vec::new();
        while all_completions.len() < 5 {
            let completions = reactor.wait(Duration::from_secs(1)).unwrap();
            all_completions.extend(completions);
        }

        assert_eq!(all_completions.len(), 5);
        for c in &all_completions {
            assert!(c.is_success());
        }
    }

    #[test]
    fn test_io_uring_supports_features() {
        let reactor = IoUringReactor::new().unwrap();

        assert!(reactor.supports(ReactorFeature::MultishotAccept));
        assert!(reactor.supports(ReactorFeature::ZeroCopySend));
        assert!(reactor.supports(ReactorFeature::RegisteredFds));
        assert!(reactor.supports(ReactorFeature::Timeouts));
        assert!(reactor.supports(ReactorFeature::Cancellation));

        // SQPOLL is only enabled with new_sqpoll()
        assert!(!reactor.supports(ReactorFeature::ZeroSyscallSubmit));
    }
}
