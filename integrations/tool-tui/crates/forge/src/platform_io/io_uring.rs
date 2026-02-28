//! io_uring I/O backend for Linux.
//!
//! This backend uses Linux's io_uring interface for high-performance
//! asynchronous I/O operations. Requires kernel 5.1+.
//!
//! io_uring provides significant performance benefits:
//! - Batch submission of multiple I/O operations in a single syscall
//! - Kernel-side polling (SQPOLL) to avoid syscall overhead
//! - Zero-copy I/O for large transfers
//! - Efficient completion notification

#![cfg(target_os = "linux")]

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use io_uring::{IoUring, opcode, types};
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use super::{EventStream, FileEvent, FileEventKind, PlatformIO, WriteOp};

/// io_uring backend for Linux.
///
/// This backend provides high-performance I/O using Linux's io_uring
/// interface, which allows batching multiple I/O operations into a
/// single system call.
pub struct IoUringBackend {
    /// The io_uring instance.
    ring: Mutex<IoUring>,
    /// Queue depth for the ring.
    queue_depth: u32,
}

impl IoUringBackend {
    /// Create a new io_uring backend with the specified queue depth.
    ///
    /// # Arguments
    /// * `queue_depth` - Number of entries in the submission queue (typically 256)
    ///
    /// # Errors
    /// Returns an error if io_uring initialization fails (e.g., kernel too old).
    pub fn new(queue_depth: u32) -> Result<Self> {
        // Check kernel support first
        if !Self::check_kernel_support() {
            return Err(anyhow!("io_uring requires Linux kernel 5.1 or higher"));
        }

        let ring = IoUring::builder()
            .build(queue_depth)
            .context("Failed to create io_uring instance")?;

        Ok(Self {
            ring: Mutex::new(ring),
            queue_depth,
        })
    }

    /// Check if io_uring is available on this system.
    ///
    /// Returns true if:
    /// - Running on Linux
    /// - Kernel version is 5.1 or higher
    /// - io_uring syscalls are available
    pub fn check_kernel_support() -> bool {
        // Check kernel version
        if let Ok(output) = std::process::Command::new("uname").arg("-r").output() {
            if let Ok(version) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = version.trim().split('.').collect();
                if parts.len() >= 2 {
                    if let (Ok(major), Ok(minor)) = (
                        parts[0].parse::<u32>(),
                        parts[1].split('-').next().unwrap_or("0").parse::<u32>(),
                    ) {
                        // io_uring requires kernel 5.1+
                        if major > 5 || (major == 5 && minor >= 1) {
                            // Try to create a small ring to verify io_uring works
                            return IoUring::builder().build(8).is_ok();
                        }
                    }
                }
            }
        }
        false
    }

    /// Submit a read operation using io_uring.
    fn submit_read(&self, fd: i32, buf: &mut [u8], offset: u64) -> Result<usize> {
        let mut ring = self.ring.lock();

        let read_e = opcode::Read::new(types::Fd(fd), buf.as_mut_ptr(), buf.len() as u32)
            .offset(offset)
            .build()
            .user_data(0x01);

        // Safety: We're submitting a valid read operation
        unsafe {
            ring.submission()
                .push(&read_e)
                .map_err(|_| anyhow!("io_uring submission queue full"))?;
        }

        ring.submit_and_wait(1)?;

        let cqe = ring.completion().next().ok_or_else(|| anyhow!("No completion event"))?;
        let result = cqe.result();

        if result < 0 {
            Err(anyhow!("io_uring read failed with error: {}", -result))
        } else {
            Ok(result as usize)
        }
    }

    /// Submit a write operation using io_uring.
    fn submit_write(&self, fd: i32, buf: &[u8], offset: u64) -> Result<usize> {
        let mut ring = self.ring.lock();

        let write_e = opcode::Write::new(types::Fd(fd), buf.as_ptr(), buf.len() as u32)
            .offset(offset)
            .build()
            .user_data(0x02);

        // Safety: We're submitting a valid write operation
        unsafe {
            ring.submission()
                .push(&write_e)
                .map_err(|_| anyhow!("io_uring submission queue full"))?;
        }

        ring.submit_and_wait(1)?;

        let cqe = ring.completion().next().ok_or_else(|| anyhow!("No completion event"))?;
        let result = cqe.result();

        if result < 0 {
            Err(anyhow!("io_uring write failed with error: {}", -result))
        } else {
            Ok(result as usize)
        }
    }
}

#[async_trait]
impl PlatformIO for IoUringBackend {
    async fn read(&self, path: &Path, buf: &mut [u8]) -> Result<usize> {
        let file =
            File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
        let fd = file.as_raw_fd();

        // Use io_uring for the actual read
        self.submit_read(fd, buf, 0)
    }

    async fn write(&self, path: &Path, buf: &[u8]) -> Result<usize> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to create file: {}", path.display()))?;
        let fd = file.as_raw_fd();

        // Use io_uring for the actual write
        self.submit_write(fd, buf, 0)
    }

    async fn read_all(&self, path: &Path) -> Result<Vec<u8>> {
        // For read_all, we need to know the file size first
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get file metadata: {}", path.display()))?;
        let size = metadata.len() as usize;

        if size == 0 {
            return Ok(Vec::new());
        }

        let mut buf = vec![0u8; size];
        let file =
            File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
        let fd = file.as_raw_fd();

        let bytes_read = self.submit_read(fd, &mut buf, 0)?;
        buf.truncate(bytes_read);

        Ok(buf)
    }

    async fn write_all(&self, path: &Path, buf: &[u8]) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to create file: {}", path.display()))?;
        let fd = file.as_raw_fd();

        let bytes_written = self.submit_write(fd, buf, 0)?;

        if bytes_written != buf.len() {
            return Err(anyhow!(
                "Incomplete write: wrote {} of {} bytes",
                bytes_written,
                buf.len()
            ));
        }

        Ok(())
    }

    async fn watch(&self, path: &Path) -> Result<Box<dyn EventStream>> {
        // For file watching, we use inotify through notify crate
        // io_uring doesn't directly support file watching
        let (tx, rx) = mpsc::channel(100);
        let watcher = IoUringWatcher::new(path.to_path_buf(), tx)?;
        Ok(Box::new(IoUringEventStream::new(rx, watcher)))
    }

    async fn batch_read(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>> {
        // For batch reads, we can submit multiple operations at once
        // This is where io_uring really shines
        let mut results = Vec::with_capacity(paths.len());

        // Open all files first
        let files: Vec<_> = paths
            .iter()
            .map(|p| {
                let metadata = std::fs::metadata(p)?;
                let file = File::open(p)?;
                Ok((file, metadata.len() as usize))
            })
            .collect::<Result<Vec<_>>>()?;

        // Submit all reads in batches
        let batch_size = self.queue_depth as usize;
        for chunk in files.chunks(batch_size) {
            let mut buffers: Vec<Vec<u8>> =
                chunk.iter().map(|(_, size)| vec![0u8; *size]).collect();

            {
                let mut ring = self.ring.lock();

                // Submit all reads in this batch
                for (i, ((file, _), buf)) in chunk.iter().zip(buffers.iter_mut()).enumerate() {
                    let fd = file.as_raw_fd();
                    let read_e =
                        opcode::Read::new(types::Fd(fd), buf.as_mut_ptr(), buf.len() as u32)
                            .offset(0)
                            .build()
                            .user_data(i as u64);

                    unsafe {
                        ring.submission()
                            .push(&read_e)
                            .map_err(|_| anyhow!("io_uring submission queue full"))?;
                    }
                }

                // Wait for all completions
                ring.submit_and_wait(chunk.len())?;

                // Process completions
                for cqe in ring.completion() {
                    let result = cqe.result();
                    if result < 0 {
                        return Err(anyhow!("io_uring batch read failed with error: {}", -result));
                    }
                    let idx = cqe.user_data() as usize;
                    buffers[idx].truncate(result as usize);
                }
            }

            results.extend(buffers);
        }

        Ok(results)
    }

    async fn batch_write(&self, ops: &[WriteOp]) -> Result<()> {
        // Ensure all parent directories exist
        for op in ops {
            if let Some(parent) = op.path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }
        }

        // Open all files
        let files: Vec<_> = ops
            .iter()
            .map(|op| OpenOptions::new().write(true).create(true).truncate(true).open(&op.path))
            .collect::<std::io::Result<Vec<_>>>()?;

        // Submit all writes in batches
        let batch_size = self.queue_depth as usize;
        for (chunk_ops, chunk_files) in ops.chunks(batch_size).zip(files.chunks(batch_size)) {
            {
                let mut ring = self.ring.lock();

                // Submit all writes in this batch
                for (i, (op, file)) in chunk_ops.iter().zip(chunk_files.iter()).enumerate() {
                    let fd = file.as_raw_fd();
                    let write_e =
                        opcode::Write::new(types::Fd(fd), op.data.as_ptr(), op.data.len() as u32)
                            .offset(0)
                            .build()
                            .user_data(i as u64);

                    unsafe {
                        ring.submission()
                            .push(&write_e)
                            .map_err(|_| anyhow!("io_uring submission queue full"))?;
                    }
                }

                // Wait for all completions
                ring.submit_and_wait(chunk_ops.len())?;

                // Process completions
                for cqe in ring.completion() {
                    let result = cqe.result();
                    if result < 0 {
                        return Err(anyhow!("io_uring batch write failed with error: {}", -result));
                    }
                }
            }

            // Handle fsync if requested
            for (op, file) in chunk_ops.iter().zip(chunk_files.iter()) {
                if op.sync {
                    file.sync_all()?;
                }
            }
        }

        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "io_uring"
    }

    fn is_available() -> bool {
        Self::check_kernel_support()
    }
}

// ============================================================================
// Event Stream Implementation (using notify/inotify)
// ============================================================================

/// Wrapper around notify watcher for io_uring backend.
struct IoUringWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl IoUringWatcher {
    fn new(path: PathBuf, tx: mpsc::Sender<FileEvent>) -> Result<Self> {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

        let tx_clone = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let kind = match event.kind {
                            notify::EventKind::Create(_) => FileEventKind::Created,
                            notify::EventKind::Modify(_) => FileEventKind::Modified,
                            notify::EventKind::Remove(_) => FileEventKind::Deleted,
                            notify::EventKind::Access(_) => FileEventKind::Metadata,
                            _ => continue,
                        };

                        let file_event = FileEvent::new(path, kind);
                        let _ = tx_clone.blocking_send(file_event);
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(&path, RecursiveMode::Recursive)?;

        Ok(Self { _watcher: watcher })
    }
}

/// Event stream for io_uring backend.
pub struct IoUringEventStream {
    rx: mpsc::Receiver<FileEvent>,
    buffer: parking_lot::Mutex<Vec<FileEvent>>,
    closed: AtomicBool,
    _watcher: Arc<IoUringWatcher>,
}

impl IoUringEventStream {
    fn new(rx: mpsc::Receiver<FileEvent>, watcher: IoUringWatcher) -> Self {
        Self {
            rx,
            buffer: parking_lot::Mutex::new(Vec::new()),
            closed: AtomicBool::new(false),
            _watcher: Arc::new(watcher),
        }
    }

    fn drain_channel(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.buffer.lock().push(event);
        }
    }
}

impl EventStream for IoUringEventStream {
    fn poll_next(&mut self) -> Option<FileEvent> {
        if self.closed.load(Ordering::SeqCst) {
            return None;
        }

        {
            let mut buffer = self.buffer.lock();
            if !buffer.is_empty() {
                return Some(buffer.remove(0));
            }
        }

        self.drain_channel();

        let mut buffer = self.buffer.lock();
        if !buffer.is_empty() {
            Some(buffer.remove(0))
        } else {
            None
        }
    }

    fn close(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        self.rx.close();
    }

    fn has_pending(&self) -> bool {
        !self.buffer.lock().is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_support_check() {
        // This test just verifies the check doesn't panic
        let _ = IoUringBackend::check_kernel_support();
    }

    #[test]
    fn test_backend_name() {
        if IoUringBackend::is_available() {
            let backend = IoUringBackend::new(64).unwrap();
            assert_eq!(backend.backend_name(), "io_uring");
        }
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::tempdir;

    // Feature: platform-native-io-hardening, Property 4: Batch Operation Correctness
    // For any set of file paths and corresponding data, a batch write followed by
    // a batch read SHALL return data equivalent to the original input for all files.
    // **Validates: Requirements 1.7**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_io_uring_batch_roundtrip(
            // Generate 1-10 files with random content (0-1000 bytes each)
            file_contents in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..1000),
                1..10
            )
        ) {
            // Skip test if io_uring is not available
            if !IoUringBackend::is_available() {
                return Ok(());
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let backend = IoUringBackend::new(64).unwrap();

                // Create write operations
                let ops: Vec<WriteOp> = file_contents
                    .iter()
                    .enumerate()
                    .map(|(i, content)| {
                        WriteOp::new(
                            dir.path().join(format!("file_{}.bin", i)),
                            content.clone(),
                        )
                    })
                    .collect();

                // Batch write
                backend.batch_write(&ops).await.unwrap();

                // Batch read
                let paths: Vec<PathBuf> = ops.iter().map(|op| op.path.clone()).collect();
                let read_contents = backend.batch_read(&paths).await.unwrap();

                // Verify round-trip
                prop_assert_eq!(read_contents.len(), file_contents.len());
                for (original, read) in file_contents.iter().zip(read_contents.iter()) {
                    prop_assert_eq!(original, read);
                }

                Ok(())
            })?;
        }

        #[test]
        fn prop_io_uring_single_file_roundtrip(
            // Generate random file content (0-10000 bytes)
            content in prop::collection::vec(any::<u8>(), 0..10000)
        ) {
            // Skip test if io_uring is not available
            if !IoUringBackend::is_available() {
                return Ok(());
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let file_path = dir.path().join("test_file.bin");
                let backend = IoUringBackend::new(64).unwrap();

                // Write
                backend.write_all(&file_path, &content).await.unwrap();

                // Read
                let read_content = backend.read_all(&file_path).await.unwrap();

                // Verify round-trip
                prop_assert_eq!(content, read_content);

                Ok(())
            })?;
        }
    }
}
