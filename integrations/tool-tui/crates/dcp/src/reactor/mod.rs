//! Platform-native reactor abstraction for async I/O.
//!
//! Provides a unified API for platform-specific I/O mechanisms:
//! - Linux: io_uring (preferred) or epoll (fallback)
//! - macOS: kqueue
//! - Windows: IOCP
//!
//! The reactor trait abstracts over these implementations to provide
//! a consistent interface for high-performance async I/O operations.

use std::io;
use std::time::Duration;

mod epoll;
mod iocp;
mod iouring;
mod kqueue;

pub use epoll::EpollReactor;
pub use iocp::IocpReactor;
pub use iouring::IoUringReactor;
pub use kqueue::KqueueReactor;

/// Token for identifying registered file descriptors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token(pub usize);

/// Interest flags for I/O events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interest {
    /// Interested in read events
    pub readable: bool,
    /// Interested in write events
    pub writable: bool,
}

impl Interest {
    /// Interest in readable events only
    pub const READABLE: Interest = Interest {
        readable: true,
        writable: false,
    };

    /// Interest in writable events only
    pub const WRITABLE: Interest = Interest {
        readable: false,
        writable: true,
    };

    /// Interest in both readable and writable events
    pub const BOTH: Interest = Interest {
        readable: true,
        writable: true,
    };
}

/// I/O event returned by poll
#[derive(Debug, Clone)]
pub struct Event {
    /// Token identifying the source
    pub token: Token,
    /// Whether the source is readable
    pub readable: bool,
    /// Whether the source is writable
    pub writable: bool,
    /// Whether an error occurred
    pub error: bool,
    /// Whether the connection was closed
    pub closed: bool,
}

impl Event {
    /// Create a new event
    pub fn new(token: Token) -> Self {
        Self {
            token,
            readable: false,
            writable: false,
            error: false,
            closed: false,
        }
    }

    /// Set readable flag
    pub fn with_readable(mut self) -> Self {
        self.readable = true;
        self
    }

    /// Set writable flag
    pub fn with_writable(mut self) -> Self {
        self.writable = true;
        self
    }

    /// Set error flag
    pub fn with_error(mut self) -> Self {
        self.error = true;
        self
    }

    /// Set closed flag
    pub fn with_closed(mut self) -> Self {
        self.closed = true;
        self
    }
}

/// Completion result for async operations
#[derive(Debug)]
pub struct Completion {
    /// Token identifying the operation
    pub token: Token,
    /// Result of the operation (bytes transferred or error)
    pub result: io::Result<usize>,
}

/// Platform-agnostic reactor trait
///
/// Provides a unified API for async I/O across different platforms.
/// Implementations use platform-specific mechanisms for optimal performance.
pub trait Reactor: Send + Sync {
    /// Poll for I/O events
    ///
    /// Blocks until events are available or timeout expires.
    /// Returns a list of events that occurred.
    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<Event>>;

    /// Register interest in a file descriptor
    ///
    /// Returns a token that identifies this registration.
    fn register(&mut self, fd: RawFd, interest: Interest) -> io::Result<Token>;

    /// Modify interest for a registered file descriptor
    fn modify(&mut self, token: Token, interest: Interest) -> io::Result<()>;

    /// Deregister a file descriptor
    fn deregister(&mut self, token: Token) -> io::Result<()>;

    /// Submit an async read operation (for io_uring/IOCP)
    ///
    /// For poll-based reactors, this is a no-op and reads should be
    /// performed after receiving a readable event.
    fn submit_read(&mut self, _token: Token, _buf: &mut [u8]) -> io::Result<Option<Completion>> {
        Ok(None)
    }

    /// Submit an async write operation (for io_uring/IOCP)
    ///
    /// For poll-based reactors, this is a no-op and writes should be
    /// performed after receiving a writable event.
    fn submit_write(&mut self, _token: Token, _buf: &[u8]) -> io::Result<Option<Completion>> {
        Ok(None)
    }

    /// Check if this reactor supports true async I/O (io_uring/IOCP)
    ///
    /// If true, submit_read/submit_write can be used for zero-copy I/O.
    /// If false, use poll() and perform I/O after receiving events.
    fn supports_async_io(&self) -> bool {
        false
    }

    /// Get the reactor type name for debugging
    fn name(&self) -> &'static str;
}

/// Raw file descriptor type (platform-specific)
#[cfg(unix)]
pub type RawFd = std::os::unix::io::RawFd;

#[cfg(windows)]
pub type RawFd = std::os::windows::io::RawSocket;

/// Reactor configuration
#[derive(Debug, Clone)]
pub struct ReactorConfig {
    /// Maximum number of events to return per poll
    pub max_events: usize,
    /// Whether to prefer io_uring on Linux (if available)
    pub prefer_io_uring: bool,
}

impl Default for ReactorConfig {
    fn default() -> Self {
        Self {
            max_events: 1024,
            prefer_io_uring: true,
        }
    }
}

/// Create a platform-appropriate reactor
///
/// On Linux: tries io_uring first, falls back to epoll
/// On macOS: uses kqueue
/// On Windows: uses IOCP
pub fn create_reactor(config: ReactorConfig) -> io::Result<Box<dyn Reactor>> {
    #[cfg(target_os = "linux")]
    {
        if config.prefer_io_uring {
            match IoUringReactor::new(config.max_events) {
                Ok(reactor) => return Ok(Box::new(reactor)),
                Err(_) => {
                    // Fall back to epoll
                }
            }
        }
        Ok(Box::new(EpollReactor::new(config.max_events)?))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(KqueueReactor::new(config.max_events)?))
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(IocpReactor::new(config.max_events)?))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No reactor implementation for this platform",
        ))
    }
}

/// Create a reactor with default configuration
pub fn create_default_reactor() -> io::Result<Box<dyn Reactor>> {
    create_reactor(ReactorConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interest_constants() {
        assert!(Interest::READABLE.readable);
        assert!(!Interest::READABLE.writable);

        assert!(!Interest::WRITABLE.readable);
        assert!(Interest::WRITABLE.writable);

        assert!(Interest::BOTH.readable);
        assert!(Interest::BOTH.writable);
    }

    #[test]
    fn test_event_builder() {
        let event = Event::new(Token(42)).with_readable().with_writable();

        assert_eq!(event.token, Token(42));
        assert!(event.readable);
        assert!(event.writable);
        assert!(!event.error);
        assert!(!event.closed);
    }

    #[test]
    fn test_token_equality() {
        assert_eq!(Token(1), Token(1));
        assert_ne!(Token(1), Token(2));
    }

    #[test]
    fn test_reactor_config_default() {
        let config = ReactorConfig::default();
        assert_eq!(config.max_events, 1024);
        assert!(config.prefer_io_uring);
    }

    #[test]
    fn test_create_default_reactor() {
        // This should succeed on all platforms
        let result = create_default_reactor();
        // On Windows, we get IOCP stub; on Linux, epoll; on macOS, kqueue
        // All should succeed
        assert!(result.is_ok());

        let reactor = result.unwrap();
        // Verify we got a valid reactor
        let name = reactor.name();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_create_reactor_with_config() {
        let config = ReactorConfig {
            max_events: 512,
            prefer_io_uring: false, // Force epoll on Linux
        };

        let result = create_reactor(config);
        assert!(result.is_ok());
    }
}
