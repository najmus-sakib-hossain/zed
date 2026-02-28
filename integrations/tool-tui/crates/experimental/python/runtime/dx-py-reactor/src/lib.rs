//! Platform-Native Async I/O Reactor for DX-Py-Runtime
//!
//! This crate provides high-performance async I/O using platform-native APIs
//! for the DX-Py runtime.
//!
//! ## Platform Support
//!
//! | Platform | Backend | Features |
//! |----------|---------|----------|
//! | Linux | io_uring | SQPOLL mode, zero-syscall submissions |
//! | macOS | kqueue | Efficient event notification |
//! | Windows | IOCP | I/O Completion Ports |
//!
//! ## Performance Targets
//!
//! - Single file read: <2μs (vs 50μs for asyncio)
//! - 100 parallel file reads: <100μs (vs 5ms for asyncio)
//! - Accept throughput: 2M+ connections/sec
//! - HTTP throughput: 500K+ requests/sec
//!
//! ## Features
//!
//! - [`Reactor`]: Platform-specific async I/O reactor trait
//! - [`ReactorPool`]: Pool of reactors for multi-core scaling
//! - [`PyFuture`]: Python-compatible future for async/await
//! - [`IoBuffer`]: Efficient I/O buffer management
//! - [`IoOperation`]: Async I/O operation types
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_py_reactor::{create_reactor, IoOperation, IoBuffer};
//!
//! // Create a platform-appropriate reactor
//! let reactor = create_reactor(0)?;
//!
//! // Submit an async read operation
//! let buffer = IoBuffer::new(4096);
//! let op = IoOperation::Read {
//!     fd: file_fd,
//!     buffer,
//!     offset: 0,
//! };
//! reactor.submit(op)?;
//!
//! // Poll for completions
//! let completions = reactor.poll()?;
//! ```
//!
//! ## Async Operations
//!
//! High-level async operations are provided for common use cases:
//!
//! - [`async_read_file`]: Read a file asynchronously
//! - [`async_write_file`]: Write a file asynchronously
//! - [`async_read_files_batch`]: Read multiple files in parallel
//! - [`async_accept`]: Accept incoming connections
//! - [`async_connect`]: Connect to a remote host
//! - [`async_resolve`]: DNS resolution

pub mod async_ops;
pub mod completion;
pub mod error;
pub mod io_buffer;
pub mod operation;
pub mod pool;
pub mod py_future;
pub mod reactor;

// Platform-specific implementations
#[cfg(target_os = "linux")]
pub mod io_uring;

#[cfg(target_os = "macos")]
pub mod kqueue;

#[cfg(target_os = "windows")]
pub mod iocp;

// Re-exports
pub use async_ops::{
    async_accept, async_connect, async_read_file, async_read_files_batch, async_recv,
    async_resolve, async_resolve_batch, async_send, async_write_file, CompletionHandler, DnsResult,
};
pub use completion::{Completion, CompletionFlags};
pub use error::{ReactorError, Result};
pub use io_buffer::IoBuffer;
pub use operation::{IoOperation, SendFlags};
pub use pool::ReactorPool;
pub use py_future::PyFuture;
pub use reactor::{Reactor, ReactorFeature, ReactorStats};

/// Create a platform-appropriate reactor for the given core ID.
///
/// On Linux, this creates an io_uring reactor with SQPOLL mode if available,
/// falling back to basic io_uring if SQPOLL is not supported.
///
/// On macOS, this creates a kqueue reactor.
///
/// On Windows, this creates an IOCP reactor.
pub fn create_reactor(_core_id: usize) -> Result<Box<dyn Reactor>> {
    #[cfg(target_os = "linux")]
    {
        // Try io_uring with SQPOLL first
        match io_uring::IoUringReactor::new_sqpoll(core_id) {
            Ok(reactor) => return Ok(Box::new(reactor)),
            Err(_) => {
                // Fall back to basic io_uring without SQPOLL
                return Ok(Box::new(io_uring::IoUringReactor::new()?));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(Box::new(kqueue::KqueueReactor::new()?));
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(iocp::IocpReactor::new()?))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(ReactorError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "Platform not supported",
        )))
    }
}

/// Create a basic reactor without platform-specific optimizations.
/// This is useful for testing or when advanced features are not needed.
pub fn create_basic_reactor() -> Result<Box<dyn Reactor>> {
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(io_uring::IoUringReactor::new()?));
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(Box::new(kqueue::KqueueReactor::new()?));
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(iocp::IocpReactor::new()?))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(ReactorError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "Platform not supported",
        )))
    }
}
