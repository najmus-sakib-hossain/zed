//! Windows IOCP reactor implementation
//!
//! Uses I/O Completion Ports for efficient async I/O on Windows

use std::io;
use std::path::Path;

use tokio::sync::mpsc;

use super::reactor::{BoxFuture, ProcessOutput, Reactor, Response, WatchEvent};
use super::tokio_reactor::TokioReactor;

/// IOCP-based reactor for Windows
pub struct IocpReactor {
    /// Fallback to Tokio for most operations
    fallback: TokioReactor,
}

impl IocpReactor {
    /// Create a new IOCP reactor
    pub fn new() -> Self {
        Self {
            fallback: TokioReactor::new(),
        }
    }
}

impl Default for IocpReactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Reactor for IocpReactor {
    fn read_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<Vec<u8>>> {
        // Tokio on Windows already uses IOCP under the hood
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
