//! macOS kqueue reactor implementation
//!
//! Uses kqueue for efficient async I/O on macOS

#![cfg(target_os = "macos")]

use std::io;
use std::path::Path;

use tokio::sync::mpsc;

use super::reactor::{BoxFuture, ProcessOutput, Reactor, Response, WatchEvent};
use super::tokio_reactor::TokioReactor;

/// kqueue-based reactor for macOS
pub struct KqueueReactor {
    /// Fallback to Tokio for most operations
    fallback: TokioReactor,
}

impl KqueueReactor {
    /// Create a new kqueue reactor
    pub fn new() -> Self {
        Self {
            fallback: TokioReactor::new(),
        }
    }
}

impl Default for KqueueReactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Reactor for KqueueReactor {
    fn read_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<Vec<u8>>> {
        // kqueue is primarily for event notification, not file I/O
        // Delegate to Tokio for actual file operations
        self.fallback.read_file(path)
    }

    fn write_file<'a>(&'a self, path: &'a Path, data: &'a [u8]) -> BoxFuture<'a, io::Result<()>> {
        self.fallback.write_file(path, data)
    }

    fn spawn_process<'a>(
        &'a self,
        cmd: &'a str,
        args: &'a [&'a str],
    ) -> BoxFuture<'a, io::Result<ProcessOutput>> {
        self.fallback.spawn_process(cmd, args)
    }

    fn watch_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, io::Result<mpsc::Receiver<WatchEvent>>> {
        // kqueue excels at directory watching
        // For now, use the notify-based implementation which uses kqueue on macOS
        self.fallback.watch_dir(path)
    }

    fn http_get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, io::Result<Response>> {
        self.fallback.http_get(url)
    }

    fn http_post<'a>(
        &'a self,
        url: &'a str,
        body: &'a [u8],
    ) -> BoxFuture<'a, io::Result<Response>> {
        self.fallback.http_post(url, body)
    }
}
