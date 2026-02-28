//! epoll backend for Linux (fallback when io_uring is unavailable).

#![cfg(target_os = "linux")]

use super::{Completion, Interest, IoHandle, Reactor, ReactorConfig};
use std::io;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Handle for epoll registered resources.
#[derive(Debug, Clone)]
pub struct EpollHandle {
    /// User data for this handle.
    user_data: u64,
    /// File descriptor.
    fd: RawFd,
}

impl IoHandle for EpollHandle {
    fn user_data(&self) -> u64 {
        self.user_data
    }
}

/// epoll reactor implementation.
pub struct EpollReactor {
    /// epoll file descriptor.
    epoll_fd: RawFd,
    /// Configuration.
    config: ReactorConfig,
    /// Next user_data value.
    next_user_data: AtomicU64,
}

impl Reactor for EpollReactor {
    type Handle = EpollHandle;

    fn new(config: ReactorConfig) -> io::Result<Self> {
        // SAFETY: epoll_create1 is safe to call with valid flags.
        // EPOLL_CLOEXEC ensures the fd is closed on exec, preventing fd leaks.
        // Returns -1 on error, which we check below.
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if epoll_fd < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            epoll_fd,
            config,
            next_user_data: AtomicU64::new(1),
        })
    }

    fn register(&self, fd: RawFd, interest: Interest) -> io::Result<Self::Handle> {
        let user_data = self.next_user_data.fetch_add(1, Ordering::Relaxed);

        let mut events = 0u32;
        if interest.is_readable() {
            events |= libc::EPOLLIN as u32;
        }
        if interest.is_writable() {
            events |= libc::EPOLLOUT as u32;
        }
        if interest.is_edge() {
            events |= libc::EPOLLET as u32;
        }
        if interest.is_oneshot() {
            events |= libc::EPOLLONESHOT as u32;
        }

        let mut event = libc::epoll_event {
            events,
            u64: user_data,
        };

        // SAFETY: epoll_ctl is safe to call with:
        // - A valid epoll_fd (created in new() and not yet closed)
        // - A valid operation (EPOLL_CTL_ADD)
        // - A valid fd to register
        // - A pointer to a properly initialized epoll_event struct
        let result = unsafe { libc::epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(EpollHandle { user_data, fd })
    }

    fn submit(&self) -> io::Result<usize> {
        // epoll doesn't have a submission queue - operations are immediate
        Ok(0)
    }

    fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Completion>> {
        let timeout_ms = timeout.map(|d| d.as_millis() as i32).unwrap_or(-1);

        let mut events =
            vec![libc::epoll_event { events: 0, u64: 0 }; self.config.entries as usize];

        // SAFETY: epoll_wait is safe to call with:
        // - A valid epoll_fd (created in new() and not yet closed)
        // - A valid pointer to a buffer of epoll_event structs
        // - The correct length of that buffer
        // - A timeout value (-1 for infinite, or milliseconds)
        let count = unsafe {
            libc::epoll_wait(self.epoll_fd, events.as_mut_ptr(), events.len() as i32, timeout_ms)
        };

        if count < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok(Vec::new());
            }
            return Err(err);
        }

        let completions = events[..count as usize]
            .iter()
            .map(|event| {
                let mut flags = 0u32;
                if event.events & libc::EPOLLIN as u32 != 0 {
                    flags |= 0x01; // READABLE
                }
                if event.events & libc::EPOLLOUT as u32 != 0 {
                    flags |= 0x02; // WRITABLE
                }
                if event.events & libc::EPOLLERR as u32 != 0 {
                    flags |= 0x04; // ERROR
                }
                if event.events & libc::EPOLLHUP as u32 != 0 {
                    flags |= 0x08; // HUP
                }

                Completion {
                    user_data: event.u64,
                    result: 0, // epoll doesn't provide byte counts
                    flags,
                }
            })
            .collect();

        Ok(completions)
    }

    fn submit_and_wait(&self, _min_complete: usize) -> io::Result<Vec<Completion>> {
        self.wait(None)
    }
}

impl Drop for EpollReactor {
    fn drop(&mut self) {
        // SAFETY: close is safe to call on any file descriptor.
        // epoll_fd was created in new() and is valid until this drop.
        // After this call, epoll_fd is invalid and must not be used.
        unsafe {
            libc::close(self.epoll_fd);
        }
    }
}
