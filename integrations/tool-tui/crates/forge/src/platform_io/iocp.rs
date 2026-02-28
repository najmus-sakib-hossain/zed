//! IOCP I/O backend for Windows.
//!
//! This backend uses Windows I/O Completion Ports for high-performance
//! asynchronous I/O operations. IOCP is the native Windows mechanism for
//! scalable async I/O.
//!
//! IOCP provides:
//! - Efficient completion notification for overlapped I/O
//! - Thread pool integration for completion processing
//! - Directory change notifications via ReadDirectoryChangesW

use anyhow::{Context, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::{EventStream, FileEvent, FileEventKind, PlatformIO, WriteOp};

/// IOCP backend for Windows.
///
/// This backend uses Windows I/O Completion Ports for efficient
/// asynchronous I/O operations. For simplicity, this implementation
/// uses tokio's async I/O (which internally uses IOCP on Windows)
/// while providing native file watching through ReadDirectoryChangesW.
pub struct IocpBackend {
    /// Number of worker threads for completion processing.
    thread_count: usize,
    /// Runtime handle for async operations.
    _runtime: tokio::runtime::Handle,
}

impl IocpBackend {
    /// Create a new IOCP backend with the specified number of worker threads.
    ///
    /// # Arguments
    /// * `thread_count` - Number of worker threads for completion processing
    pub fn new(thread_count: usize) -> Result<Self> {
        Ok(Self {
            thread_count,
            _runtime: tokio::runtime::Handle::current(),
        })
    }

    /// Get the number of worker threads.
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }
}

#[async_trait]
impl PlatformIO for IocpBackend {
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
        let watcher = IocpWatcher::new(path.to_path_buf(), tx)?;
        Ok(Box::new(IocpEventStream::new(rx, watcher)))
    }

    async fn batch_read(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>> {
        // Use concurrent reads for better performance with IOCP
        let mut handles = Vec::with_capacity(paths.len());

        for path in paths {
            let path = path.clone();
            handles.push(tokio::spawn(async move { tokio::fs::read(&path).await }));
        }

        let mut results = Vec::with_capacity(paths.len());
        for handle in handles {
            let content = handle.await??;
            results.push(content);
        }

        Ok(results)
    }

    async fn batch_write(&self, ops: &[WriteOp]) -> Result<()> {
        // Use concurrent writes for better performance with IOCP
        let mut handles = Vec::with_capacity(ops.len());

        for op in ops {
            let path = op.path.clone();
            let data = op.data.clone();
            let sync = op.sync;

            handles.push(tokio::spawn(async move {
                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                }

                tokio::fs::write(&path, &data).await?;

                if sync {
                    // Open file with write access for sync_all to work properly on Windows
                    let file = tokio::fs::OpenOptions::new().write(true).open(&path).await?;
                    file.sync_all().await?;
                }

                Ok::<(), anyhow::Error>(())
            }));
        }

        for handle in handles {
            handle.await??;
        }

        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "iocp"
    }

    fn is_available() -> bool {
        // IOCP is always available on Windows
        true
    }
}

// ============================================================================
// Event Stream Implementation (using notify/ReadDirectoryChangesW)
// ============================================================================

/// Wrapper around notify watcher using ReadDirectoryChangesW on Windows.
struct IocpWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl IocpWatcher {
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

/// Event stream for IOCP backend.
pub struct IocpEventStream {
    rx: mpsc::Receiver<FileEvent>,
    buffer: Mutex<Vec<FileEvent>>,
    closed: AtomicBool,
    _watcher: Arc<IocpWatcher>,
}

impl IocpEventStream {
    fn new(rx: mpsc::Receiver<FileEvent>, watcher: IocpWatcher) -> Self {
        Self {
            rx,
            buffer: Mutex::new(Vec::new()),
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

impl EventStream for IocpEventStream {
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
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_iocp_read_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let backend = IocpBackend::new(4).unwrap();

        // Write data
        let data = b"Hello, IOCP!";
        backend.write_all(&file_path, data).await.unwrap();

        // Read data back
        let read_data = backend.read_all(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_iocp_batch_operations() {
        let dir = tempdir().unwrap();
        let backend = IocpBackend::new(4).unwrap();

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
    async fn test_iocp_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nested").join("deep").join("test.txt");
        let backend = IocpBackend::new(4).unwrap();

        backend.write_all(&file_path, b"test").await.unwrap();
        assert!(file_path.exists());
    }

    #[test]
    fn test_iocp_is_available() {
        assert!(IocpBackend::is_available());
    }

    #[tokio::test]
    async fn test_iocp_backend_name() {
        let backend = IocpBackend::new(4).unwrap();
        assert_eq!(backend.backend_name(), "iocp");
    }

    #[tokio::test]
    async fn test_iocp_thread_count() {
        let backend = IocpBackend::new(8).unwrap();
        assert_eq!(backend.thread_count(), 8);
    }
}
