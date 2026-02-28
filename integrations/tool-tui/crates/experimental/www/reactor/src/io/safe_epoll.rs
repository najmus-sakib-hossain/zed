//! Safe wrappers for epoll operations.
//!
//! This module provides safe Rust wrappers around the raw libc epoll functions,
//! ensuring proper error handling and preventing common mistakes.

#![cfg(target_os = "linux")]

use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

/// Result type for epoll operations.
pub type EpollResult<T> = io::Result<T>;

/// Safe wrapper for epoll_create1.
///
/// Creates a new epoll instance with the CLOEXEC flag set.
///
/// # Returns
///
/// The epoll file descriptor on success, or an IO error on failure.
///
/// # Example
///
/// ```rust,ignore
/// let epoll_fd = safe_epoll_create()?;
/// // Use epoll_fd...
/// safe_epoll_close(epoll_fd)?;
/// ```
#[inline]
pub fn safe_epoll_create() -> EpollResult<RawFd> {
    // SAFETY: epoll_create1 is safe to call with valid flags.
    // EPOLL_CLOEXEC ensures the fd is closed on exec.
    let fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };

    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(fd)
    }
}

/// Safe wrapper for epoll_ctl with EPOLL_CTL_ADD.
///
/// Registers a file descriptor with the epoll instance.
///
/// # Arguments
///
/// * `epoll_fd` - The epoll file descriptor
/// * `fd` - The file descriptor to register
/// * `events` - The events to monitor (EPOLLIN, EPOLLOUT, etc.)
/// * `user_data` - User data to associate with this registration
///
/// # Returns
///
/// Ok(()) on success, or an IO error on failure.
#[inline]
pub fn safe_epoll_ctl_add(
    epoll_fd: RawFd,
    fd: RawFd,
    events: u32,
    user_data: u64,
) -> EpollResult<()> {
    let mut event = libc::epoll_event {
        events,
        u64: user_data,
    };

    // SAFETY: We're passing valid pointers and the epoll_fd should be valid.
    // The event struct is properly initialized.
    let result = unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Safe wrapper for epoll_ctl with EPOLL_CTL_MOD.
///
/// Modifies the events for an already registered file descriptor.
///
/// # Arguments
///
/// * `epoll_fd` - The epoll file descriptor
/// * `fd` - The file descriptor to modify
/// * `events` - The new events to monitor
/// * `user_data` - User data to associate with this registration
///
/// # Returns
///
/// Ok(()) on success, or an IO error on failure.
#[inline]
pub fn safe_epoll_ctl_mod(
    epoll_fd: RawFd,
    fd: RawFd,
    events: u32,
    user_data: u64,
) -> EpollResult<()> {
    let mut event = libc::epoll_event {
        events,
        u64: user_data,
    };

    // SAFETY: Same as safe_epoll_ctl_add
    let result = unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_MOD, fd, &mut event) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Safe wrapper for epoll_ctl with EPOLL_CTL_DEL.
///
/// Removes a file descriptor from the epoll instance.
///
/// # Arguments
///
/// * `epoll_fd` - The epoll file descriptor
/// * `fd` - The file descriptor to remove
///
/// # Returns
///
/// Ok(()) on success, or an IO error on failure.
#[inline]
pub fn safe_epoll_ctl_del(epoll_fd: RawFd, fd: RawFd) -> EpollResult<()> {
    // SAFETY: For EPOLL_CTL_DEL, the event pointer can be null (since Linux 2.6.9)
    let result =
        unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Epoll event wrapper for safe iteration.
#[derive(Debug, Clone, Copy)]
pub struct EpollEvent {
    /// The events that occurred.
    pub events: u32,
    /// The user data associated with this event.
    pub user_data: u64,
}

impl EpollEvent {
    /// Check if the event indicates readability.
    #[inline]
    pub fn is_readable(&self) -> bool {
        self.events & libc::EPOLLIN as u32 != 0
    }

    /// Check if the event indicates writability.
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.events & libc::EPOLLOUT as u32 != 0
    }

    /// Check if the event indicates an error.
    #[inline]
    pub fn is_error(&self) -> bool {
        self.events & libc::EPOLLERR as u32 != 0
    }

    /// Check if the event indicates a hangup.
    #[inline]
    pub fn is_hangup(&self) -> bool {
        self.events & libc::EPOLLHUP as u32 != 0
    }
}

/// Safe wrapper for epoll_wait.
///
/// Waits for events on the epoll instance.
///
/// # Arguments
///
/// * `epoll_fd` - The epoll file descriptor
/// * `events` - Buffer to store the events
/// * `timeout` - Optional timeout duration (None = wait indefinitely)
///
/// # Returns
///
/// A slice of the events buffer containing the ready events, or an IO error.
///
/// # Example
///
/// ```rust,ignore
/// let mut events = vec![EpollEvent { events: 0, user_data: 0 }; 64];
/// let ready = safe_epoll_wait(epoll_fd, &mut events, Some(Duration::from_secs(1)))?;
/// for event in ready {
///     println!("Event: user_data={}, readable={}", event.user_data, event.is_readable());
/// }
/// ```
pub fn safe_epoll_wait(
    epoll_fd: RawFd,
    events: &mut [libc::epoll_event],
    timeout: Option<Duration>,
) -> EpollResult<Vec<EpollEvent>> {
    let timeout_ms = timeout.map(|d| d.as_millis() as i32).unwrap_or(-1);

    // SAFETY: We're passing a valid buffer and the epoll_fd should be valid.
    // The events buffer is properly sized.
    let count =
        unsafe { libc::epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, timeout_ms) };

    if count < 0 {
        let err = io::Error::last_os_error();
        // EINTR is not a real error - just means we were interrupted
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(Vec::new());
        }
        return Err(err);
    }

    let result = events[..count as usize]
        .iter()
        .map(|e| EpollEvent {
            events: e.events,
            user_data: e.u64,
        })
        .collect();

    Ok(result)
}

/// Safe wrapper for closing an epoll file descriptor.
///
/// # Arguments
///
/// * `epoll_fd` - The epoll file descriptor to close
///
/// # Returns
///
/// Ok(()) on success, or an IO error on failure.
#[inline]
pub fn safe_epoll_close(epoll_fd: RawFd) -> EpollResult<()> {
    // SAFETY: close is safe to call on any file descriptor.
    // If the fd is invalid, it will return an error.
    let result = unsafe { libc::close(epoll_fd) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Builder for epoll events flags.
#[derive(Debug, Clone, Copy, Default)]
pub struct EpollFlags(u32);

impl EpollFlags {
    /// Create a new empty flags builder.
    pub fn new() -> Self {
        Self(0)
    }

    /// Add EPOLLIN (readable) flag.
    pub fn readable(mut self) -> Self {
        self.0 |= libc::EPOLLIN as u32;
        self
    }

    /// Add EPOLLOUT (writable) flag.
    pub fn writable(mut self) -> Self {
        self.0 |= libc::EPOLLOUT as u32;
        self
    }

    /// Add EPOLLET (edge-triggered) flag.
    pub fn edge_triggered(mut self) -> Self {
        self.0 |= libc::EPOLLET as u32;
        self
    }

    /// Add EPOLLONESHOT flag.
    pub fn oneshot(mut self) -> Self {
        self.0 |= libc::EPOLLONESHOT as u32;
        self
    }

    /// Get the raw flags value.
    pub fn bits(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoll_flags_builder() {
        let flags = EpollFlags::new().readable().writable().edge_triggered();

        assert!(flags.bits() & libc::EPOLLIN as u32 != 0);
        assert!(flags.bits() & libc::EPOLLOUT as u32 != 0);
        assert!(flags.bits() & libc::EPOLLET as u32 != 0);
    }

    #[test]
    fn test_epoll_event_checks() {
        let event = EpollEvent {
            events: libc::EPOLLIN as u32 | libc::EPOLLHUP as u32,
            user_data: 42,
        };

        assert!(event.is_readable());
        assert!(!event.is_writable());
        assert!(!event.is_error());
        assert!(event.is_hangup());
        assert_eq!(event.user_data, 42);
    }
}
