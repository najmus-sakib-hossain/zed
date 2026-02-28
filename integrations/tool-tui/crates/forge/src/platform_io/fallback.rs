//! Fallback I/O backend using tokio async I/O.
//!
//! This backend provides cross-platform compatibility when native I/O
//! mechanisms (io_uring, kqueue, IOCP) are unavailable.

use anyhow::{Context, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::{EventStream, FileEvent, FileEventKind, PlatformIO, WriteOp};

/// Fallback backend using tokio's async I/O.
///
/// This backend works on all platforms and provides a baseline implementation
/// when platform-native I/O is unavailable.
#[derive(Debug)]
pub struct FallbackBackend {
    /// Runtime handle for spawning tasks.
    _runtime: tokio::runtime::Handle,
}

impl FallbackBackend {
    /// Create a new fallback backend.
    pub fn new() -> Self {
        Self {
            _runtime: tokio::runtime::Handle::current(),
        }
    }
}

impl Default for FallbackBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformIO for FallbackBackend {
    async fn read(&self, path: &Path, buf: &mut [u8]) -> Result<usize> {
        let mut file = tokio::fs::File::open(path)
            .await
            .with_context(|| format!("Failed to open file for reading: {}", path.display()))?;

        let bytes_read = file
            .read(buf)
            .await
            .with_context(|| format!("Failed to read from file: {}", path.display()))?;

        Ok(bytes_read)
    }

    async fn write(&self, path: &Path, buf: &[u8]) -> Result<usize> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }
        }

        let mut file = tokio::fs::File::create(path)
            .await
            .with_context(|| format!("Failed to create file for writing: {}", path.display()))?;

        let bytes_written = file
            .write(buf)
            .await
            .with_context(|| format!("Failed to write to file: {}", path.display()))?;

        Ok(bytes_written)
    }

    async fn read_all(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))
    }

    async fn write_all(&self, path: &Path, buf: &[u8]) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }
        }

        tokio::fs::write(path, buf)
            .await
            .with_context(|| format!("Failed to write file: {}", path.display()))
    }

    async fn watch(&self, path: &Path) -> Result<Box<dyn EventStream>> {
        let (tx, rx) = mpsc::channel(100);
        let path = path.to_path_buf();

        // Use notify crate for file watching
        let watcher = NotifyWatcher::new(path, tx)?;
        Ok(Box::new(FallbackEventStream::new(rx, watcher)))
    }

    async fn batch_read(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>> {
        // Sequential reads for fallback - could be parallelized with join_all
        let mut results = Vec::with_capacity(paths.len());

        for path in paths {
            let content = self.read_all(path).await?;
            results.push(content);
        }

        Ok(results)
    }

    async fn batch_write(&self, ops: &[WriteOp]) -> Result<()> {
        for op in ops {
            self.write_all(&op.path, &op.data).await?;

            if op.sync {
                // Open file with write access for sync_all to work properly
                let file = tokio::fs::OpenOptions::new().write(true).open(&op.path).await?;
                file.sync_all().await?;
            }
        }

        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "fallback"
    }

    fn is_available() -> bool {
        // Fallback is always available
        true
    }
}

// ============================================================================
// Event Stream Implementation
// ============================================================================

/// Wrapper around notify watcher for event streaming.
struct NotifyWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl NotifyWatcher {
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

/// Fallback event stream using notify.
pub struct FallbackEventStream {
    rx: mpsc::Receiver<FileEvent>,
    buffer: Mutex<Vec<FileEvent>>,
    closed: AtomicBool,
    _watcher: Arc<NotifyWatcher>,
}

impl FallbackEventStream {
    fn new(rx: mpsc::Receiver<FileEvent>, watcher: NotifyWatcher) -> Self {
        Self {
            rx,
            buffer: Mutex::new(Vec::new()),
            closed: AtomicBool::new(false),
            _watcher: Arc::new(watcher),
        }
    }

    /// Try to receive events from the channel into the buffer.
    fn drain_channel(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.buffer.lock().push(event);
        }
    }
}

impl EventStream for FallbackEventStream {
    fn poll_next(&mut self) -> Option<FileEvent> {
        if self.closed.load(Ordering::SeqCst) {
            return None;
        }

        // First check buffer
        {
            let mut buffer = self.buffer.lock();
            if !buffer.is_empty() {
                return Some(buffer.remove(0));
            }
        }

        // Try to receive from channel
        self.drain_channel();

        // Check buffer again
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
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fallback_read_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let backend = FallbackBackend::new();

        // Write data
        let data = b"Hello, World!";
        backend.write_all(&file_path, data).await.unwrap();

        // Read data back
        let read_data = backend.read_all(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_fallback_batch_operations() {
        let dir = tempdir().unwrap();
        let backend = FallbackBackend::new();

        // Batch write
        let ops = vec![
            WriteOp::new(dir.path().join("file1.txt"), b"content1".to_vec()),
            WriteOp::new(dir.path().join("file2.txt"), b"content2".to_vec()),
            WriteOp::new(dir.path().join("file3.txt"), b"content3".to_vec()),
        ];
        backend.batch_write(&ops).await.unwrap();

        // Batch read
        let paths: Vec<PathBuf> = ops.iter().map(|op| op.path.clone()).collect();
        let contents = backend.batch_read(&paths).await.unwrap();

        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0], b"content1");
        assert_eq!(contents[1], b"content2");
        assert_eq!(contents[2], b"content3");
    }

    #[tokio::test]
    async fn test_fallback_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nested").join("deep").join("test.txt");
        let backend = FallbackBackend::new();

        backend.write_all(&file_path, b"test").await.unwrap();
        assert!(file_path.exists());
    }

    #[test]
    fn test_fallback_is_available() {
        assert!(FallbackBackend::is_available());
    }

    #[tokio::test]
    async fn test_fallback_backend_name() {
        let backend = FallbackBackend::new();
        assert_eq!(backend.backend_name(), "fallback");
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
        fn prop_batch_roundtrip(
            // Generate 1-10 files with random content (0-1000 bytes each)
            file_contents in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..1000),
                1..10
            )
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let backend = FallbackBackend::new();

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
        fn prop_single_file_roundtrip(
            // Generate random file content (0-10000 bytes)
            content in prop::collection::vec(any::<u8>(), 0..10000)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let file_path = dir.path().join("test_file.bin");
                let backend = FallbackBackend::new();

                // Write
                backend.write_all(&file_path, &content).await.unwrap();

                // Read
                let read_content = backend.read_all(&file_path).await.unwrap();

                // Verify round-trip
                prop_assert_eq!(content, read_content);

                Ok(())
            })?;
        }

        #[test]
        fn prop_nested_directory_creation(
            // Generate random depth (1-5) and content
            depth in 1usize..5,
            content in prop::collection::vec(any::<u8>(), 0..100)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let backend = FallbackBackend::new();

                // Create nested path
                let mut path = dir.path().to_path_buf();
                for i in 0..depth {
                    path = path.join(format!("level_{}", i));
                }
                path = path.join("file.bin");

                // Write should create all parent directories
                backend.write_all(&path, &content).await.unwrap();

                // Verify file exists and content matches
                prop_assert!(path.exists());
                let read_content = backend.read_all(&path).await.unwrap();
                prop_assert_eq!(content, read_content);

                Ok(())
            })?;
        }
    }
}
