//! Safe wrappers for kqueue operations.
//!
//! This module provides safe Rust wrappers around the raw libc kqueue functions,
//! ensuring proper error handling and preventing common mistakes.

#![cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]

use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

/// Result type for kqueue operations.
pub type KqueueResult<T> = io::Result<T>;

/// Safe wrapper for kqueue creation.
///
/// Creates a new kqueue instance.
///
/// # Returns
///
/// The kqueue file descriptor on success, or an IO error on failure.
///
/// # Example
///
/// ```rust,ignore
/// let kq = safe_kqueue_create()?;
/// // Use kq...
/// safe_kqueue_close(kq)?;
/// ```
#[inline]
pub fn safe_kqueue_create() -> KqueueResult<RawFd> {
    // SAFETY: kqueue() is safe to call and returns a new file descriptor.
    let kq = unsafe { libc::kqueue() };

    if kq < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(kq)
    }
}

/// Kevent filter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KqueueFilter {
    /// Read filter (EVFILT_READ)
    Read,
    /// Write filter (EVFILT_WRITE)
    Write,
    /// Timer filter (EVFILT_TIMER)
    Timer,
    /// Signal filter (EVFILT_SIGNAL)
    Signal,
}

impl KqueueFilter {
    fn to_libc(self) -> i16 {
        match self {
            Self::Read => libc::EVFILT_READ,
            Self::Write => libc::EVFILT_WRITE,
            Self::Timer => libc::EVFILT_TIMER,
            Self::Signal => libc::EVFILT_SIGNAL,
        }
    }

    fn from_libc(filter: i16) -> Option<Self> {
        match filter {
            libc::EVFILT_READ => Some(Self::Read),
            libc::EVFILT_WRITE => Some(Self::Write),
            libc::EVFILT_TIMER => Some(Self::Timer),
            libc::EVFILT_SIGNAL => Some(Self::Signal),
            _ => None,
        }
    }
}

/// Kevent action flags.
#[derive(Debug, Clone, Copy, Default)]
pub struct KqueueFlags(u16);

impl KqueueFlags {
    /// Create empty flags.
    pub fn new() -> Self {
        Self(0)
    }

    /// Add the event to the kqueue (EV_ADD).
    pub fn add(mut self) -> Self {
        self.0 |= libc::EV_ADD as u16;
        self
    }

    /// Enable the event (EV_ENABLE).
    pub fn enable(mut self) -> Self {
        self.0 |= libc::EV_ENABLE as u16;
        self
    }

    /// Disable the event (EV_DISABLE).
    pub fn disable(mut self) -> Self {
        self.0 |= libc::EV_DISABLE as u16;
        self
    }

    /// Delete the event from the kqueue (EV_DELETE).
    pub fn delete(mut self) -> Self {
        self.0 |= libc::EV_DELETE as u16;
        self
    }

    /// One-shot event (EV_ONESHOT).
    pub fn oneshot(mut self) -> Self {
        self.0 |= libc::EV_ONESHOT as u16;
        self
    }

    /// Clear the event state after retrieval (EV_CLEAR).
    pub fn clear(mut self) -> Self {
        self.0 |= libc::EV_CLEAR as u16;
        self
    }

    /// Get the raw flags value.
    pub fn bits(self) -> u16 {
        self.0
    }
}

/// A kevent change to submit.
#[derive(Debug, Clone)]
pub struct KqueueChange {
    /// The identifier (usually a file descriptor).
    pub ident: usize,
    /// The filter type.
    pub filter: KqueueFilter,
    /// The action flags.
    pub flags: KqueueFlags,
    /// Filter-specific flags.
    pub fflags: u32,
    /// Filter-specific data.
    pub data: isize,
    /// User data.
    pub user_data: u64,
}

impl KqueueChange {
    /// Create a new read event registration.
    pub fn read(fd: RawFd, user_data: u64) -> Self {
        Self {
            ident: fd as usize,
            filter: KqueueFilter::Read,
            flags: KqueueFlags::new().add().enable(),
            fflags: 0,
            data: 0,
            user_data,
        }
    }

    /// Create a new write event registration.
    pub fn write(fd: RawFd, user_data: u64) -> Self {
        Self {
            ident: fd as usize,
            filter: KqueueFilter::Write,
            flags: KqueueFlags::new().add().enable(),
            fflags: 0,
            data: 0,
            user_data,
        }
    }

    /// Create a delete event.
    pub fn delete_read(fd: RawFd) -> Self {
        Self {
            ident: fd as usize,
            filter: KqueueFilter::Read,
            flags: KqueueFlags::new().delete(),
            fflags: 0,
            data: 0,
            user_data: 0,
        }
    }

    /// Create a delete event for write.
    pub fn delete_write(fd: RawFd) -> Self {
        Self {
            ident: fd as usize,
            filter: KqueueFilter::Write,
            flags: KqueueFlags::new().delete(),
            fflags: 0,
            data: 0,
            user_data: 0,
        }
    }

    fn to_kevent(&self) -> libc::kevent {
        libc::kevent {
            ident: self.ident,
            filter: self.filter.to_libc(),
            flags: self.flags.bits(),
            fflags: self.fflags,
            data: self.data,
            udata: self.user_data as *mut libc::c_void,
        }
    }
}

/// A kevent that was returned from kevent().
#[derive(Debug, Clone, Copy)]
pub struct KqueueEvent {
    /// The identifier (usually a file descriptor).
    pub ident: usize,
    /// The filter type.
    pub filter: Option<KqueueFilter>,
    /// The flags.
    pub flags: u16,
    /// Filter-specific flags.
    pub fflags: u32,
    /// Filter-specific data (e.g., bytes available).
    pub data: isize,
    /// User data.
    pub user_data: u64,
}

impl KqueueEvent {
    /// Check if this is a read event.
    pub fn is_readable(&self) -> bool {
        self.filter == Some(KqueueFilter::Read)
    }

    /// Check if this is a write event.
    pub fn is_writable(&self) -> bool {
        self.filter == Some(KqueueFilter::Write)
    }

    /// Check if an error occurred.
    pub fn is_error(&self) -> bool {
        self.flags & libc::EV_ERROR as u16 != 0
    }

    /// Check if EOF was reached.
    pub fn is_eof(&self) -> bool {
        self.flags & libc::EV_EOF as u16 != 0
    }

    /// Get the number of bytes available (for read events).
    pub fn bytes_available(&self) -> usize {
        if self.data > 0 { self.data as usize } else { 0 }
    }
}

/// Safe wrapper for kevent() to submit changes.
///
/// Submits a list of changes to the kqueue without waiting for events.
///
/// # Arguments
///
/// * `kq` - The kqueue file descriptor
/// * `changes` - The changes to submit
///
/// # Returns
///
/// The number of changes successfully submitted, or an IO error.
pub fn safe_kevent_submit(kq: RawFd, changes: &[KqueueChange]) -> KqueueResult<usize> {
    if changes.is_empty() {
        return Ok(0);
    }

    let kevents: Vec<libc::kevent> = changes.iter().map(|c| c.to_kevent()).collect();

    // SAFETY: We're passing valid pointers and the kq should be valid.
    // The kevents are properly initialized.
    let result = unsafe {
        libc::kevent(
            kq,
            kevents.as_ptr(),
            kevents.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
        )
    };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(changes.len())
    }
}

/// Safe wrapper for kevent() to wait for events.
///
/// Waits for events on the kqueue.
///
/// # Arguments
///
/// * `kq` - The kqueue file descriptor
/// * `max_events` - Maximum number of events to return
/// * `timeout` - Optional timeout duration (None = wait indefinitely)
///
/// # Returns
///
/// A vector of events that occurred, or an IO error.
pub fn safe_kevent_wait(
    kq: RawFd,
    max_events: usize,
    timeout: Option<Duration>,
) -> KqueueResult<Vec<KqueueEvent>> {
    let timespec = timeout.map(|d| libc::timespec {
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    });

    let mut events = vec![
        libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        max_events
    ];

    // SAFETY: We're passing valid pointers and the kq should be valid.
    // The events buffer is properly sized.
    let count = unsafe {
        libc::kevent(
            kq,
            std::ptr::null(),
            0,
            events.as_mut_ptr(),
            events.len() as i32,
            timespec.as_ref().map(|t| t as *const _).unwrap_or(std::ptr::null()),
        )
    };

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
        .map(|e| KqueueEvent {
            ident: e.ident,
            filter: KqueueFilter::from_libc(e.filter),
            flags: e.flags,
            fflags: e.fflags,
            data: e.data,
            user_data: e.udata as u64,
        })
        .collect();

    Ok(result)
}

/// Safe wrapper for kevent() to submit changes and wait for events.
///
/// Combines submission and waiting in a single syscall.
///
/// # Arguments
///
/// * `kq` - The kqueue file descriptor
/// * `changes` - The changes to submit
/// * `max_events` - Maximum number of events to return
/// * `timeout` - Optional timeout duration (None = wait indefinitely)
///
/// # Returns
///
/// A vector of events that occurred, or an IO error.
pub fn safe_kevent_submit_and_wait(
    kq: RawFd,
    changes: &[KqueueChange],
    max_events: usize,
    timeout: Option<Duration>,
) -> KqueueResult<Vec<KqueueEvent>> {
    let kevents: Vec<libc::kevent> = changes.iter().map(|c| c.to_kevent()).collect();

    let timespec = timeout.map(|d| libc::timespec {
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    });

    let mut events = vec![
        libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        max_events
    ];

    // SAFETY: We're passing valid pointers and the kq should be valid.
    let count = unsafe {
        libc::kevent(
            kq,
            if kevents.is_empty() {
                std::ptr::null()
            } else {
                kevents.as_ptr()
            },
            kevents.len() as i32,
            events.as_mut_ptr(),
            events.len() as i32,
            timespec.as_ref().map(|t| t as *const _).unwrap_or(std::ptr::null()),
        )
    };

    if count < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(Vec::new());
        }
        return Err(err);
    }

    let result = events[..count as usize]
        .iter()
        .map(|e| KqueueEvent {
            ident: e.ident,
            filter: KqueueFilter::from_libc(e.filter),
            flags: e.flags,
            fflags: e.fflags,
            data: e.data,
            user_data: e.udata as u64,
        })
        .collect();

    Ok(result)
}

/// Safe wrapper for closing a kqueue file descriptor.
///
/// # Arguments
///
/// * `kq` - The kqueue file descriptor to close
///
/// # Returns
///
/// Ok(()) on success, or an IO error on failure.
#[inline]
pub fn safe_kqueue_close(kq: RawFd) -> KqueueResult<()> {
    // SAFETY: close is safe to call on any file descriptor.
    let result = unsafe { libc::close(kq) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kqueue_flags_builder() {
        let flags = KqueueFlags::new().add().enable().oneshot();

        assert!(flags.bits() & libc::EV_ADD as u16 != 0);
        assert!(flags.bits() & libc::EV_ENABLE as u16 != 0);
        assert!(flags.bits() & libc::EV_ONESHOT as u16 != 0);
    }

    #[test]
    fn test_kqueue_change_read() {
        let change = KqueueChange::read(5, 42);

        assert_eq!(change.ident, 5);
        assert_eq!(change.filter, KqueueFilter::Read);
        assert_eq!(change.user_data, 42);
    }

    #[test]
    fn test_kqueue_event_checks() {
        let event = KqueueEvent {
            ident: 5,
            filter: Some(KqueueFilter::Read),
            flags: libc::EV_EOF as u16,
            fflags: 0,
            data: 100,
            user_data: 42,
        };

        assert!(event.is_readable());
        assert!(!event.is_writable());
        assert!(!event.is_error());
        assert!(event.is_eof());
        assert_eq!(event.bytes_available(), 100);
        assert_eq!(event.user_data, 42);
    }
}
