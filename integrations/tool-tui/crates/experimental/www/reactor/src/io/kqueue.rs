//! kqueue backend for macOS and BSD systems.

#![cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]

use super::{Completion, Interest, IoHandle, Reactor, ReactorConfig};
use std::io;
use std::os::unix::io::RawFd;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Handle for kqueue registered resources.
#[derive(Debug, Clone)]
pub struct KqueueHandle {
    /// User data for this handle.
    user_data: u64,
    /// File descriptor.
    fd: RawFd,
}

impl IoHandle for KqueueHandle {
    fn user_data(&self) -> u64 {
        self.user_data
    }
}

/// Pending kevent change.
#[derive(Debug, Clone)]
struct PendingChange {
    ident: usize,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: isize,
    udata: u64,
}

/// kqueue reactor implementation.
pub struct KqueueReactor {
    /// kqueue file descriptor.
    kq: RawFd,
    /// Configuration.
    config: ReactorConfig,
    /// Next user_data value.
    next_user_data: AtomicU64,
    /// Pending changes to submit.
    pending_changes: Mutex<Vec<PendingChange>>,
}

impl KqueueReactor {
    /// Register for read events.
    pub fn register_read(&self, fd: RawFd, user_data: u64) -> io::Result<()> {
        let change = PendingChange {
            ident: fd as usize,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD as u16 | libc::EV_ENABLE as u16,
            fflags: 0,
            data: 0,
            udata: user_data,
        };

        // Note: Mutex::lock() can only fail if the lock is poisoned (another thread panicked while holding it)
        // In that case, we return an error rather than panicking
        self.pending_changes
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "pending_changes lock poisoned"))?
            .push(change);
        Ok(())
    }

    /// Register for write events.
    pub fn register_write(&self, fd: RawFd, user_data: u64) -> io::Result<()> {
        let change = PendingChange {
            ident: fd as usize,
            filter: libc::EVFILT_WRITE,
            flags: libc::EV_ADD as u16 | libc::EV_ENABLE as u16,
            fflags: 0,
            data: 0,
            udata: user_data,
        };

        self.pending_changes
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "pending_changes lock poisoned"))?
            .push(change);
        Ok(())
    }

    /// Get the number of pending changes.
    pub fn pending_count(&self) -> usize {
        self.pending_changes.lock().map(|guard| guard.len()).unwrap_or(0)
    }
}

impl Reactor for KqueueReactor {
    type Handle = KqueueHandle;

    fn new(config: ReactorConfig) -> io::Result<Self> {
        // SAFETY: kqueue() is safe to call and returns a new file descriptor.
        // Returns -1 on error, which we check below.
        let kq = unsafe { libc::kqueue() };
        if kq < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            kq,
            config,
            next_user_data: AtomicU64::new(1),
            pending_changes: Mutex::new(Vec::new()),
        })
    }

    fn register(&self, fd: RawFd, interest: Interest) -> io::Result<Self::Handle> {
        let user_data = self.next_user_data.fetch_add(1, Ordering::Relaxed);

        if interest.is_readable() {
            self.register_read(fd, user_data)?;
        }
        if interest.is_writable() {
            self.register_write(fd, user_data)?;
        }

        Ok(KqueueHandle { user_data, fd })
    }

    fn submit(&self) -> io::Result<usize> {
        let changes = {
            let mut pending = self.pending_changes.lock().map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "pending_changes lock poisoned")
            })?;
            std::mem::take(&mut *pending)
        };

        if changes.is_empty() {
            return Ok(0);
        }

        let kevents: Vec<libc::kevent> = changes
            .iter()
            .map(|c| libc::kevent {
                ident: c.ident,
                filter: c.filter,
                flags: c.flags,
                fflags: c.fflags,
                data: c.data,
                udata: c.udata as *mut libc::c_void,
            })
            .collect();

        // SAFETY: kevent is safe to call with:
        // - A valid kqueue fd (created in new() and not yet closed)
        // - A valid pointer to an array of kevent structs for changes
        // - The correct count of changes
        // - null for events (we're only submitting, not waiting)
        // - 0 for nevents
        // - null for timeout
        let result = unsafe {
            libc::kevent(
                self.kq,
                kevents.as_ptr(),
                kevents.len() as i32,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(changes.len())
    }

    fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Completion>> {
        // First submit any pending changes
        let _ = self.submit()?;

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
            self.config.entries as usize
        ];

        // SAFETY: kevent is safe to call with:
        // - A valid kqueue fd (created in new() and not yet closed)
        // - null for changes (we're only waiting, not submitting)
        // - 0 for nchanges
        // - A valid pointer to a buffer of kevent structs
        // - The correct length of that buffer
        // - A timeout value (null for infinite, or pointer to timespec)
        let count = unsafe {
            libc::kevent(
                self.kq,
                std::ptr::null(),
                0,
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

        let completions = events[..count as usize]
            .iter()
            .map(|event| {
                let mut flags = 0u32;
                if event.filter == libc::EVFILT_READ {
                    flags |= 0x01; // READABLE
                }
                if event.filter == libc::EVFILT_WRITE {
                    flags |= 0x02; // WRITABLE
                }
                if event.flags & libc::EV_ERROR as u16 != 0 {
                    flags |= 0x04; // ERROR
                }
                if event.flags & libc::EV_EOF as u16 != 0 {
                    flags |= 0x08; // EOF/HUP
                }

                Completion {
                    user_data: event.udata as u64,
                    result: event.data as i32,
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

impl Drop for KqueueReactor {
    fn drop(&mut self) {
        // SAFETY: close is safe to call on any file descriptor.
        // self.kq was created in new() and is valid until this drop.
        // After this call, self.kq is invalid and must not be used.
        unsafe {
            libc::close(self.kq);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_changes_cleared_after_wait() {
        let config = ReactorConfig::default();
        let reactor = KqueueReactor::new(config).unwrap();

        // Add some pending changes
        reactor.register_read(0, 1).unwrap();
        reactor.register_write(0, 2).unwrap();

        assert_eq!(reactor.pending_count(), 2);

        // Wait should submit and clear pending changes
        // Note: This will fail on actual kqueue since fd 0 is stdin
        // but it tests the clearing behavior
        let _ = reactor.wait(Some(Duration::from_millis(0)));

        assert_eq!(reactor.pending_count(), 0);
    }
}
