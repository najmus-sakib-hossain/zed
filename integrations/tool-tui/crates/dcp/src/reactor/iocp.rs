//! IOCP reactor implementation for Windows.
//!
//! Uses Windows I/O Completion Ports for async I/O.
//! Note: Full IOCP implementation requires the windows-sys crate.
//! This module provides the interface and a stub implementation.

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use super::{Completion, Event, Interest, RawFd, Reactor, Token};

/// IOCP reactor for Windows
///
/// Uses Windows I/O Completion Ports for true async I/O.
///
/// Note: This is a stub implementation. For full IOCP support,
/// add the `windows-sys` crate as a dependency.
pub struct IocpReactor {
    /// Maximum events per poll
    max_events: usize,
    /// Registered file descriptors/handles
    registrations: HashMap<Token, Registration>,
    /// Next token to assign
    next_token: usize,
    /// Pending completions
    pending: Vec<PendingOp>,
}

/// Registration state for a handle
struct Registration {
    fd: RawFd,
    interest: Interest,
}

/// Pending async operation
#[allow(dead_code)]
struct PendingOp {
    token: Token,
    op_type: OpType,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum OpType {
    Read,
    Write,
}

impl IocpReactor {
    /// Create a new IOCP reactor
    ///
    /// On Windows, this creates an I/O Completion Port.
    /// On other platforms, this creates a stub implementation.
    pub fn new(max_events: usize) -> io::Result<Self> {
        #[cfg(target_os = "windows")]
        {
            // Note: Full IOCP implementation would use:
            // CreateIoCompletionPort(INVALID_HANDLE_VALUE, null, 0, 0)
            // For now, we provide a stub that can be extended
            // when windows-sys is added as a dependency.

            Ok(Self {
                max_events,
                registrations: HashMap::new(),
                next_token: 1,
                pending: Vec::new(),
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Stub implementation for non-Windows platforms
            Ok(Self {
                max_events,
                registrations: HashMap::new(),
                next_token: 1,
                pending: Vec::new(),
            })
        }
    }

    /// Check if IOCP is available on this system
    pub fn is_available() -> bool {
        cfg!(target_os = "windows")
    }
}

impl Reactor for IocpReactor {
    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<Event>> {
        #[cfg(target_os = "windows")]
        {
            // Stub implementation for Windows
            // Full implementation would use GetQueuedCompletionStatusEx

            // Simulate timeout
            if let Some(duration) = timeout {
                std::thread::sleep(duration.min(Duration::from_millis(100)));
            }

            // Return events for any pending operations (simulated)
            let events: Vec<Event> = self
                .pending
                .iter()
                .filter_map(|op| {
                    if self.registrations.contains_key(&op.token) {
                        Some(Event::new(op.token).with_readable())
                    } else {
                        None
                    }
                })
                .collect();

            // Clear pending after returning
            self.pending.clear();

            Ok(events)
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Stub for non-Windows: just sleep and return empty
            if let Some(duration) = timeout {
                std::thread::sleep(duration.min(Duration::from_millis(100)));
            }
            Ok(Vec::new())
        }
    }

    fn register(&mut self, fd: RawFd, interest: Interest) -> io::Result<Token> {
        let token = Token(self.next_token);
        self.next_token += 1;

        #[cfg(target_os = "windows")]
        {
            // Full implementation would associate handle with completion port:
            // CreateIoCompletionPort(fd, self.port, token.0, 0)
        }

        self.registrations.insert(token, Registration { fd, interest });

        Ok(token)
    }

    fn modify(&mut self, token: Token, interest: Interest) -> io::Result<()> {
        if let Some(reg) = self.registrations.get_mut(&token) {
            reg.interest = interest;
            // IOCP doesn't have a modify operation - interest is per-operation
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Token not registered"))
        }
    }

    fn deregister(&mut self, token: Token) -> io::Result<()> {
        self.registrations.remove(&token);
        self.pending.retain(|op| op.token != token);
        // IOCP handles are automatically disassociated when closed
        Ok(())
    }

    fn submit_read(&mut self, token: Token, _buf: &mut [u8]) -> io::Result<Option<Completion>> {
        if !self.registrations.contains_key(&token) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Token not registered"));
        }

        // In full implementation:
        // 1. Create OVERLAPPED structure
        // 2. Call ReadFile with overlapped
        // 3. Completion comes via GetQueuedCompletionStatus

        self.pending.push(PendingOp {
            token,
            op_type: OpType::Read,
        });

        Ok(None)
    }

    fn submit_write(&mut self, token: Token, _buf: &[u8]) -> io::Result<Option<Completion>> {
        if !self.registrations.contains_key(&token) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Token not registered"));
        }

        // In full implementation:
        // 1. Create OVERLAPPED structure
        // 2. Call WriteFile with overlapped
        // 3. Completion comes via GetQueuedCompletionStatus

        self.pending.push(PendingOp {
            token,
            op_type: OpType::Write,
        });

        Ok(None)
    }

    fn supports_async_io(&self) -> bool {
        // True async I/O is supported on Windows via IOCP
        cfg!(target_os = "windows")
    }

    fn name(&self) -> &'static str {
        "iocp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iocp_creation() {
        let result = IocpReactor::new(256);
        assert!(result.is_ok());
    }

    #[test]
    fn test_iocp_name() {
        let reactor = IocpReactor::new(256).unwrap();
        assert_eq!(reactor.name(), "iocp");
    }

    #[test]
    fn test_iocp_register_deregister() {
        let mut reactor = IocpReactor::new(256).unwrap();

        // Register a fake fd
        let token = reactor.register(42, Interest::READABLE).unwrap();
        assert_eq!(token, Token(1));

        // Modify interest
        assert!(reactor.modify(token, Interest::BOTH).is_ok());

        // Deregister
        assert!(reactor.deregister(token).is_ok());

        // Modify after deregister should fail
        assert!(reactor.modify(token, Interest::READABLE).is_err());
    }

    #[test]
    fn test_iocp_submit_operations() {
        let mut reactor = IocpReactor::new(256).unwrap();

        let token = reactor.register(42, Interest::BOTH).unwrap();

        // Submit read
        let mut buf = [0u8; 1024];
        let result = reactor.submit_read(token, &mut buf);
        assert!(result.is_ok());

        // Submit write
        let data = b"test data";
        let result = reactor.submit_write(token, data);
        assert!(result.is_ok());

        // Submit to unregistered token should fail
        let bad_token = Token(999);
        assert!(reactor.submit_read(bad_token, &mut buf).is_err());
    }

    #[test]
    fn test_iocp_availability() {
        let available = IocpReactor::is_available();
        #[cfg(target_os = "windows")]
        assert!(available);
        #[cfg(not(target_os = "windows"))]
        assert!(!available);
    }
}
