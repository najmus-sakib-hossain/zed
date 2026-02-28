//! kqueue reactor implementation for macOS/BSD.
//!
//! Uses BSD's kqueue interface for event-driven I/O.

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use super::{Event, Interest, RawFd, Reactor, Token};

/// kqueue reactor for macOS/BSD
///
/// Uses BSD's kqueue interface for event-driven I/O.
/// This is a poll-based reactor (not true async I/O).
pub struct KqueueReactor {
    /// Maximum events per poll
    max_events: usize,
    /// Registered file descriptors
    registrations: HashMap<Token, Registration>,
    /// Reverse mapping from fd to token
    fd_to_token: HashMap<RawFd, Token>,
    /// Next token to assign
    next_token: usize,
    /// kqueue file descriptor (on macOS/BSD)
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    kq: RawFd,
}

/// Registration state for a file descriptor
struct Registration {
    fd: RawFd,
    interest: Interest,
}

impl KqueueReactor {
    /// Create a new kqueue reactor
    pub fn new(max_events: usize) -> io::Result<Self> {
        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let kq = unsafe { libc::kqueue() };
            if kq < 0 {
                return Err(io::Error::last_os_error());
            }

            // Set close-on-exec
            unsafe {
                let flags = libc::fcntl(kq, libc::F_GETFD);
                if flags >= 0 {
                    libc::fcntl(kq, libc::F_SETFD, flags | libc::FD_CLOEXEC);
                }
            }

            Ok(Self {
                max_events,
                registrations: HashMap::new(),
                fd_to_token: HashMap::new(),
                next_token: 1,
                kq,
            })
        }

        #[cfg(not(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            // Stub implementation for non-BSD platforms
            Ok(Self {
                max_events,
                registrations: HashMap::new(),
                fd_to_token: HashMap::new(),
                next_token: 1,
            })
        }
    }

    /// Register kevent changes
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn kevent_register(&self, fd: RawFd, interest: Interest, add: bool) -> io::Result<()> {
        let mut changes = Vec::new();
        let flags = if add {
            libc::EV_ADD | libc::EV_CLEAR
        } else {
            libc::EV_DELETE
        };

        if interest.readable || !add {
            changes.push(libc::kevent {
                ident: fd as usize,
                filter: libc::EVFILT_READ,
                flags: if add && interest.readable {
                    flags
                } else {
                    libc::EV_DELETE
                },
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            });
        }

        if interest.writable || !add {
            changes.push(libc::kevent {
                ident: fd as usize,
                filter: libc::EVFILT_WRITE,
                flags: if add && interest.writable {
                    flags
                } else {
                    libc::EV_DELETE
                },
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            });
        }

        if changes.is_empty() {
            return Ok(());
        }

        let result = unsafe {
            libc::kevent(
                self.kq,
                changes.as_ptr(),
                changes.len() as i32,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };

        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
impl Drop for KqueueReactor {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.kq);
        }
    }
}

impl Reactor for KqueueReactor {
    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<Event>> {
        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let timespec = timeout.map(|d| libc::timespec {
                tv_sec: d.as_secs() as libc::time_t,
                tv_nsec: d.subsec_nanos() as libc::c_long,
            });

            let mut kevents: Vec<libc::kevent> =
                vec![unsafe { std::mem::zeroed() }; self.max_events];

            let n = unsafe {
                libc::kevent(
                    self.kq,
                    std::ptr::null(),
                    0,
                    kevents.as_mut_ptr(),
                    self.max_events as i32,
                    timespec.as_ref().map(|t| t as *const _).unwrap_or(std::ptr::null()),
                )
            };

            if n < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    return Ok(Vec::new());
                }
                return Err(err);
            }

            // Group events by fd
            let mut fd_events: HashMap<RawFd, Event> = HashMap::new();

            for i in 0..n as usize {
                let kevent = &kevents[i];
                let fd = kevent.ident as RawFd;

                if let Some(&token) = self.fd_to_token.get(&fd) {
                    let event = fd_events.entry(fd).or_insert_with(|| Event::new(token));

                    if kevent.filter == libc::EVFILT_READ {
                        event.readable = true;
                    }
                    if kevent.filter == libc::EVFILT_WRITE {
                        event.writable = true;
                    }
                    if (kevent.flags & libc::EV_ERROR) != 0 {
                        event.error = true;
                    }
                    if (kevent.flags & libc::EV_EOF) != 0 {
                        event.closed = true;
                    }
                }
            }

            Ok(fd_events.into_values().collect())
        }

        #[cfg(not(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            // Stub for non-BSD: just sleep and return empty
            if let Some(duration) = timeout {
                std::thread::sleep(duration.min(Duration::from_millis(100)));
            }
            Ok(Vec::new())
        }
    }

    fn register(&mut self, fd: RawFd, interest: Interest) -> io::Result<Token> {
        let token = Token(self.next_token);
        self.next_token += 1;

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            self.kevent_register(fd, interest, true)?;
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

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            // Remove old interest, add new
            self.kevent_register(reg.fd, reg.interest, false)?;
            self.kevent_register(reg.fd, interest, true)?;
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

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            // Ignore errors on deregister - fd might already be closed
            let _ = self.kevent_register(reg.fd, reg.interest, false);
        }

        Ok(())
    }

    fn supports_async_io(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "kqueue"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kqueue_creation() {
        let result = KqueueReactor::new(256);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kqueue_name() {
        let reactor = KqueueReactor::new(256).unwrap();
        assert_eq!(reactor.name(), "kqueue");
    }

    #[test]
    fn test_kqueue_no_async_io() {
        let reactor = KqueueReactor::new(256).unwrap();
        assert!(!reactor.supports_async_io());
    }
}
