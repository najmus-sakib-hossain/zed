//! epoll reactor implementation for Linux.
//!
//! Provides async I/O using Linux's epoll interface.
//! This is the fallback when io_uring is not available.

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use super::{Event, Interest, RawFd, Reactor, Token};

/// epoll reactor for Linux
///
/// Uses Linux's epoll interface for event-driven I/O.
/// This is a poll-based reactor (not true async I/O).
pub struct EpollReactor {
    /// Maximum events per poll
    max_events: usize,
    /// Registered file descriptors
    registrations: HashMap<Token, Registration>,
    /// Reverse mapping from fd to token
    fd_to_token: HashMap<RawFd, Token>,
    /// Next token to assign
    next_token: usize,
    /// epoll file descriptor (on Linux)
    #[cfg(target_os = "linux")]
    epfd: RawFd,
}

/// Registration state for a file descriptor
struct Registration {
    fd: RawFd,
    interest: Interest,
}

impl EpollReactor {
    /// Create a new epoll reactor
    pub fn new(max_events: usize) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            // Create epoll instance
            let epfd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
            if epfd < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self {
                max_events,
                registrations: HashMap::new(),
                fd_to_token: HashMap::new(),
                next_token: 1,
                epfd,
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Stub implementation for non-Linux platforms
            Ok(Self {
                max_events,
                registrations: HashMap::new(),
                fd_to_token: HashMap::new(),
                next_token: 1,
            })
        }
    }

    /// Convert Interest to epoll events
    #[cfg(target_os = "linux")]
    fn interest_to_epoll(interest: Interest) -> u32 {
        let mut events = libc::EPOLLET as u32; // Edge-triggered
        if interest.readable {
            events |= libc::EPOLLIN as u32;
        }
        if interest.writable {
            events |= libc::EPOLLOUT as u32;
        }
        events
    }

    /// Convert epoll events to Event
    #[cfg(target_os = "linux")]
    fn epoll_to_event(epoll_event: &libc::epoll_event, token: Token) -> Event {
        let events = epoll_event.events;
        Event {
            token,
            readable: (events & libc::EPOLLIN as u32) != 0,
            writable: (events & libc::EPOLLOUT as u32) != 0,
            error: (events & libc::EPOLLERR as u32) != 0,
            closed: (events & (libc::EPOLLHUP | libc::EPOLLRDHUP) as u32) != 0,
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for EpollReactor {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.epfd);
        }
    }
}

impl Reactor for EpollReactor {
    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<Event>> {
        #[cfg(target_os = "linux")]
        {
            let timeout_ms = timeout.map(|d| d.as_millis() as i32).unwrap_or(-1);

            let mut epoll_events: Vec<libc::epoll_event> =
                vec![unsafe { std::mem::zeroed() }; self.max_events];

            let n = unsafe {
                libc::epoll_wait(
                    self.epfd,
                    epoll_events.as_mut_ptr(),
                    self.max_events as i32,
                    timeout_ms,
                )
            };

            if n < 0 {
                let err = io::Error::last_os_error();
                // EINTR is not an error, just retry
                if err.kind() == io::ErrorKind::Interrupted {
                    return Ok(Vec::new());
                }
                return Err(err);
            }

            let mut events = Vec::with_capacity(n as usize);
            for i in 0..n as usize {
                let epoll_event = &epoll_events[i];
                let fd = epoll_event.u64 as RawFd;

                if let Some(&token) = self.fd_to_token.get(&fd) {
                    events.push(Self::epoll_to_event(epoll_event, token));
                }
            }

            Ok(events)
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Stub for non-Linux: just sleep and return empty
            if let Some(duration) = timeout {
                std::thread::sleep(duration.min(Duration::from_millis(100)));
            }
            Ok(Vec::new())
        }
    }

    fn register(&mut self, fd: RawFd, interest: Interest) -> io::Result<Token> {
        let token = Token(self.next_token);
        self.next_token += 1;

        #[cfg(target_os = "linux")]
        {
            let mut event = libc::epoll_event {
                events: Self::interest_to_epoll(interest),
                u64: fd as u64,
            };

            let result = unsafe { libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_ADD, fd, &mut event) };

            if result < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        self.registrations.insert(token, Registration { fd, interest });
        self.fd_to_token.insert(fd, token);

        Ok(token)
    }

    fn modify(&mut self, token: Token, interest: Interest) -> io::Result<()> {
        let reg = self
            .registrations
            .get_mut(&token)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Token not registered"))?;

        #[cfg(target_os = "linux")]
        {
            let mut event = libc::epoll_event {
                events: Self::interest_to_epoll(interest),
                u64: reg.fd as u64,
            };

            let result =
                unsafe { libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_MOD, reg.fd, &mut event) };

            if result < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        reg.interest = interest;
        Ok(())
    }

    fn deregister(&mut self, token: Token) -> io::Result<()> {
        let reg = self
            .registrations
            .remove(&token)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Token not registered"))?;

        self.fd_to_token.remove(&reg.fd);

        #[cfg(target_os = "linux")]
        {
            let result = unsafe {
                libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_DEL, reg.fd, std::ptr::null_mut())
            };

            if result < 0 {
                // Ignore ENOENT - fd might already be closed
                let err = io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::ENOENT) {
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    fn supports_async_io(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "epoll"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoll_creation() {
        let result = EpollReactor::new(256);
        assert!(result.is_ok());
    }

    #[test]
    fn test_epoll_name() {
        let reactor = EpollReactor::new(256).unwrap();
        assert_eq!(reactor.name(), "epoll");
    }

    #[test]
    fn test_epoll_no_async_io() {
        let reactor = EpollReactor::new(256).unwrap();
        assert!(!reactor.supports_async_io());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_interest_to_epoll() {
        let readable = EpollReactor::interest_to_epoll(Interest::READABLE);
        assert!((readable & libc::EPOLLIN as u32) != 0);
        assert!((readable & libc::EPOLLOUT as u32) == 0);

        let writable = EpollReactor::interest_to_epoll(Interest::WRITABLE);
        assert!((writable & libc::EPOLLIN as u32) == 0);
        assert!((writable & libc::EPOLLOUT as u32) != 0);

        let both = EpollReactor::interest_to_epoll(Interest::BOTH);
        assert!((both & libc::EPOLLIN as u32) != 0);
        assert!((both & libc::EPOLLOUT as u32) != 0);
    }
}
