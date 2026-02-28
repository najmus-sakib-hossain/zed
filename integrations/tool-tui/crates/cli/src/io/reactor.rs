//! Reactor trait - Unified async I/O interface
//!
//! This trait provides a platform-agnostic abstraction over native async I/O
//! mechanisms (io_uring, kqueue, IOCP) with a Tokio fallback.

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::process::ExitStatus;

/// Output from a spawned process
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// Exit status of the process
    pub status: ExitStatus,
    /// Standard output bytes
    pub stdout: Vec<u8>,
    /// Standard error bytes
    pub stderr: Vec<u8>,
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body bytes
    pub body: Vec<u8>,
}

/// File system watch event
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was created
    Create(std::path::PathBuf),
    /// File was modified
    Modify(std::path::PathBuf),
    /// File was deleted
    Delete(std::path::PathBuf),
    /// File was renamed (old path, new path)
    Rename(std::path::PathBuf, std::path::PathBuf),
}

/// Type alias for async boxed futures
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Unified reactor trait for platform-specific I/O
///
/// This trait provides async operations for file I/O, process spawning,
/// directory watching, and HTTP requests. Implementations use platform-native
/// mechanisms for optimal performance.
pub trait Reactor: Send + Sync + 'static {
    /// Read file contents asynchronously
    ///
    /// # Arguments
    /// * `path` - Path to the file to read
    ///
    /// # Returns
    /// The file contents as a byte vector
    fn read_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<Vec<u8>>>;

    /// Write file contents asynchronously
    ///
    /// # Arguments
    /// * `path` - Path to the file to write
    /// * `data` - Data to write to the file
    fn write_file<'a>(&'a self, path: &'a Path, data: &'a [u8]) -> BoxFuture<'a, io::Result<()>>;

    /// Spawn a process and capture output
    ///
    /// # Arguments
    /// * `cmd` - Command to execute
    /// * `args` - Arguments to pass to the command
    ///
    /// # Returns
    /// The process output including exit status, stdout, and stderr
    fn spawn_process<'a>(
        &'a self,
        cmd: &'a str,
        args: &'a [&'a str],
    ) -> BoxFuture<'a, io::Result<ProcessOutput>>;

    /// Watch a directory for changes
    ///
    /// # Arguments
    /// * `path` - Path to the directory to watch
    ///
    /// # Returns
    /// A receiver for watch events
    fn watch_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, io::Result<tokio::sync::mpsc::Receiver<WatchEvent>>>;

    /// Perform an HTTP GET request
    ///
    /// # Arguments
    /// * `url` - URL to request
    ///
    /// # Returns
    /// The HTTP response
    fn http_get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, io::Result<Response>>;

    /// Perform an HTTP POST request
    ///
    /// # Arguments
    /// * `url` - URL to request
    /// * `body` - Request body bytes
    ///
    /// # Returns
    /// The HTTP response
    fn http_post<'a>(&'a self, url: &'a str, body: &'a [u8])
    -> BoxFuture<'a, io::Result<Response>>;
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use tempfile::tempdir;

    // Feature: dx-cli, Property 1: File I/O Round-Trip
    // Validates: Requirements 1.6
    //
    // For any byte sequence written to a file via the Reactor,
    // reading that file should return the exact same byte sequence.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_file_io_round_trip(data in proptest::collection::vec(any::<u8>(), 0..10000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let reactor = super::super::create_reactor();
                let temp_dir = tempdir().unwrap();
                let test_file = temp_dir.path().join("test_file.bin");

                // Write data
                reactor.write_file(&test_file, &data).await.unwrap();

                // Read data back
                let read_data = reactor.read_file(&test_file).await.unwrap();

                // Verify round-trip
                prop_assert_eq!(data, read_data);
                Ok(())
            })?;
        }
    }

    #[tokio::test]
    async fn test_file_io_basic() {
        let reactor = super::super::create_reactor();
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("basic_test.txt");

        let data = b"Hello, DX CLI!";
        reactor.write_file(&test_file, data).await.unwrap();

        let read_data = reactor.read_file(&test_file).await.unwrap();
        assert_eq!(data.to_vec(), read_data);
    }

    #[tokio::test]
    async fn test_file_io_empty() {
        let reactor = super::super::create_reactor();
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("empty_test.txt");

        let data: &[u8] = b"";
        reactor.write_file(&test_file, data).await.unwrap();

        let read_data = reactor.read_file(&test_file).await.unwrap();
        assert_eq!(data.to_vec(), read_data);
    }

    #[tokio::test]
    async fn test_file_io_binary() {
        let reactor = super::super::create_reactor();
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("binary_test.bin");

        // Binary data with all byte values
        let data: Vec<u8> = (0..=255).collect();
        reactor.write_file(&test_file, &data).await.unwrap();

        let read_data = reactor.read_file(&test_file).await.unwrap();
        assert_eq!(data, read_data);
    }
}
