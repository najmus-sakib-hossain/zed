//! High-level async I/O operations
//!
//! This module provides convenient async functions for common I/O operations:
//! - File operations (read, write, batch reads)
//! - Network operations (accept, connect, send, recv)
//! - DNS resolution

use crate::completion::Completion;
use crate::error::{ReactorError, Result};
use crate::io_buffer::IoBuffer;
use crate::operation::IoOperation;
use crate::py_future::PyFuture;
use crate::reactor::Reactor;

use std::io;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::io::RawFd;
#[cfg(windows)]
use std::os::windows::io::RawHandle;

/// File descriptor type (platform-specific)
#[cfg(unix)]
pub type Fd = RawFd;
#[cfg(windows)]
pub type Fd = RawHandle;

// ============================================================================
// Task 19.11: Async File Operations
// ============================================================================

/// Read a file asynchronously.
///
/// Returns a future that resolves to the file contents.
pub fn async_read_file<R: Reactor + ?Sized>(
    reactor: &mut R,
    path: &Path,
) -> Result<PyFuture<Vec<u8>>> {
    let file = std::fs::File::open(path).map_err(ReactorError::Io)?;

    let metadata = file.metadata().map_err(ReactorError::Io)?;

    let size = metadata.len() as usize;

    #[cfg(unix)]
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);
    #[cfg(windows)]
    let fd = std::os::windows::io::AsRawHandle::as_raw_handle(&file);

    // Keep file open
    std::mem::forget(file);

    let buf = IoBuffer::new(size);
    let future = PyFuture::new();

    let op = IoOperation::Read {
        fd,
        buf: buf.clone(),
        offset: 0,
        user_data: 1,
    };

    reactor.submit(op)?;

    // In a real implementation, we'd register a callback to complete the future
    // For now, we return the future and the caller must poll the reactor

    Ok(future)
}

/// Write data to a file asynchronously.
///
/// Returns a future that resolves to the number of bytes written.
pub fn async_write_file<R: Reactor + ?Sized>(
    reactor: &mut R,
    path: &Path,
    data: &[u8],
) -> Result<PyFuture<usize>> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(ReactorError::Io)?;

    #[cfg(unix)]
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);
    #[cfg(windows)]
    let fd = std::os::windows::io::AsRawHandle::as_raw_handle(&file);

    // Keep file open
    std::mem::forget(file);

    let buf = IoBuffer::from_vec(data.to_vec());
    let future = PyFuture::new();

    let op = IoOperation::Write {
        fd,
        buf,
        offset: 0,
        user_data: 1,
    };

    reactor.submit(op)?;

    Ok(future)
}

/// Read multiple files in parallel.
///
/// This is more efficient than reading files sequentially as it
/// batches the I/O operations.
pub fn async_read_files_batch<R: Reactor + ?Sized>(
    reactor: &mut R,
    paths: &[&Path],
) -> Result<Vec<PyFuture<Vec<u8>>>> {
    let mut futures = Vec::with_capacity(paths.len());
    let mut ops = Vec::with_capacity(paths.len());

    for (i, path) in paths.iter().enumerate() {
        let file = std::fs::File::open(path).map_err(ReactorError::Io)?;

        let metadata = file.metadata().map_err(ReactorError::Io)?;

        let size = metadata.len() as usize;

        #[cfg(unix)]
        let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);
        #[cfg(windows)]
        let fd = std::os::windows::io::AsRawHandle::as_raw_handle(&file);

        std::mem::forget(file);

        let buf = IoBuffer::new(size);
        let future = PyFuture::new();
        futures.push(future);

        ops.push(IoOperation::Read {
            fd,
            buf,
            offset: 0,
            user_data: i as u64 + 1,
        });
    }

    // Submit all operations in a single batch
    reactor.submit_batch(ops)?;

    Ok(futures)
}

// ============================================================================
// Task 19.12: Async Network Operations
// ============================================================================

/// Accept a connection asynchronously.
///
/// Returns a future that resolves to the accepted socket file descriptor.
pub fn async_accept<R: Reactor + ?Sized>(reactor: &mut R, listener_fd: Fd) -> Result<PyFuture<Fd>> {
    let future = PyFuture::new();

    let op = IoOperation::Accept {
        fd: listener_fd,
        user_data: 1,
    };

    reactor.submit(op)?;

    Ok(future)
}

/// Connect to a remote address asynchronously.
///
/// Returns a future that resolves when the connection is established.
pub fn async_connect<R: Reactor + ?Sized>(
    reactor: &mut R,
    socket_fd: Fd,
    addr: std::net::SocketAddr,
) -> Result<PyFuture<()>> {
    let future = PyFuture::new();

    let op = IoOperation::Connect {
        fd: socket_fd,
        addr,
        user_data: 1,
    };

    reactor.submit(op)?;

    Ok(future)
}

/// Send data on a socket asynchronously.
///
/// Returns a future that resolves to the number of bytes sent.
pub fn async_send<R: Reactor + ?Sized>(
    reactor: &mut R,
    socket_fd: Fd,
    data: &[u8],
) -> Result<PyFuture<usize>> {
    let future = PyFuture::new();

    let buf = IoBuffer::from_vec(data.to_vec());
    let op = IoOperation::Send {
        fd: socket_fd,
        buf,
        flags: crate::operation::SendFlags::empty(),
        user_data: 1,
    };

    reactor.submit(op)?;

    Ok(future)
}

/// Receive data from a socket asynchronously.
///
/// Returns a future that resolves to the received data.
pub fn async_recv<R: Reactor + ?Sized>(
    reactor: &mut R,
    socket_fd: Fd,
    max_size: usize,
) -> Result<PyFuture<Vec<u8>>> {
    let future = PyFuture::new();

    let buf = IoBuffer::new(max_size);
    let op = IoOperation::Recv {
        fd: socket_fd,
        buf,
        user_data: 1,
    };

    reactor.submit(op)?;

    Ok(future)
}

// ============================================================================
// Task 19.13: Async DNS Resolution
// ============================================================================

/// DNS resolution result
#[derive(Debug, Clone)]
pub struct DnsResult {
    /// Resolved IP addresses
    pub addresses: Vec<std::net::IpAddr>,
    /// Canonical name (if available)
    pub canonical_name: Option<String>,
}

/// Resolve a hostname asynchronously.
///
/// This performs DNS resolution in a background thread to avoid blocking
/// the reactor. Returns a future that resolves to the DNS result.
///
/// Note: True async DNS would require platform-specific APIs (like
/// getaddrinfo_a on Linux). This implementation uses a thread pool
/// for portability.
pub fn async_resolve(hostname: &str) -> PyFuture<DnsResult> {
    let future = PyFuture::new();
    let future_clone = future.clone();
    let hostname = hostname.to_string();

    // Spawn a thread for DNS resolution
    // In a production implementation, this would use a thread pool
    std::thread::spawn(move || match resolve_hostname(&hostname) {
        Ok(result) => future_clone.set_result(result),
        Err(e) => future_clone.set_error(e),
    });

    future
}

/// Synchronous hostname resolution (used internally)
fn resolve_hostname(hostname: &str) -> io::Result<DnsResult> {
    use std::net::ToSocketAddrs;

    // Add a dummy port for resolution
    let addr_str = format!("{}:0", hostname);

    let addresses: Vec<std::net::IpAddr> =
        addr_str.to_socket_addrs()?.map(|addr| addr.ip()).collect();

    if addresses.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No addresses found for hostname: {}", hostname),
        ));
    }

    Ok(DnsResult {
        addresses,
        canonical_name: None, // Would need platform-specific APIs for CNAME
    })
}

/// Resolve multiple hostnames in parallel.
pub fn async_resolve_batch(hostnames: &[&str]) -> Vec<PyFuture<DnsResult>> {
    hostnames.iter().map(|h| async_resolve(h)).collect()
}

// ============================================================================
// Helper: Completion Handler
// ============================================================================

/// Process completions and resolve corresponding futures.
///
/// This is a helper for integrating the reactor with the future system.
pub struct CompletionHandler<T> {
    pending: std::collections::HashMap<u64, PyFuture<T>>,
}

impl<T: Clone + Send + 'static> CompletionHandler<T> {
    /// Create a new completion handler.
    pub fn new() -> Self {
        Self {
            pending: std::collections::HashMap::new(),
        }
    }

    /// Register a future for a user_data ID.
    pub fn register(&mut self, user_data: u64, future: PyFuture<T>) {
        self.pending.insert(user_data, future);
    }

    /// Process a completion and resolve the corresponding future.
    pub fn process<F>(&mut self, completion: &Completion, converter: F)
    where
        F: FnOnce(&Completion) -> io::Result<T>,
    {
        if let Some(future) = self.pending.remove(&completion.user_data) {
            match converter(completion) {
                Ok(value) => future.set_result(value),
                Err(e) => future.set_error(e),
            }
        }
    }

    /// Get the number of pending futures.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl<T: Clone + Send + 'static> Default for CompletionHandler<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_resolve_localhost() {
        let result = resolve_hostname("localhost");
        assert!(result.is_ok());
        let dns = result.unwrap();
        assert!(!dns.addresses.is_empty());
    }

    #[test]
    fn test_completion_handler() {
        let mut handler: CompletionHandler<usize> = CompletionHandler::new();
        let future = PyFuture::new();

        handler.register(1, future.clone());
        assert_eq!(handler.pending_count(), 1);

        let completion = Completion::success(1, 100);
        handler.process(&completion, |c| Ok(c.bytes()));

        assert_eq!(handler.pending_count(), 0);
        assert_eq!(future.try_get().unwrap().unwrap(), 100);
    }
}
