//! Platform I/O Abstraction Layer
//!
//! Provides a unified async I/O interface across platforms:
//! - Linux: io_uring (when available)
//! - macOS: kqueue via mio
//! - Windows: IOCP via mio
//! - Fallback: Tokio async runtime

mod reactor;
mod tokio_reactor;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

pub use reactor::{ProcessOutput, Reactor, Response, WatchEvent};

/// Create a platform-optimal reactor instance
pub fn create_reactor() -> Box<dyn Reactor> {
    #[cfg(target_os = "linux")]
    {
        // Try io_uring first, fall back to Tokio if unavailable
        match linux::IoUringReactor::new() {
            Ok(reactor) => Box::new(reactor),
            Err(_) => Box::new(tokio_reactor::TokioReactor::new()),
        }
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(macos::KqueueReactor::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::IocpReactor::new())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(tokio_reactor::TokioReactor::new())
    }
}
