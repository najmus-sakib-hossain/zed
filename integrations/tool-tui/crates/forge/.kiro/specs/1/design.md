
# Design Document: Platform-Native I/O and Hardening

## Overview

This design document describes the architecture for implementing platform-native I/O optimization and comprehensive hardening for DX Forge. The implementation leverages io_uring on Linux, kqueue on macOS, and IOCP on Windows to achieve maximum I/O performance while maintaining a unified API surface. The design follows a layered architecture with clear separation between the platform abstraction layer, the I/O operations layer, and the application layer. This enables testing, maintainability, and graceful fallback when native APIs are unavailable.

## Architecture

@tree[]

## Components and Interfaces

### 1. Platform I/O Trait (`src/platform_io/mod.rs`)

The core abstraction that all platform backends implement:
```rust
/// Platform-native I/O operations trait


#[async_trait]


pub trait PlatformIO: Send + Sync { /// Read file contents into buffer async fn read(&self, path: &Path, buf: &mut [u8]) -> Result<usize>;
/// Write buffer contents to file async fn write(&self, path: &Path, buf: &[u8]) -> Result<usize>;
/// Read entire file into Vec async fn read_all(&self, path: &Path) -> Result<Vec<u8>>;
/// Write entire buffer to file async fn write_all(&self, path: &Path, buf: &[u8]) -> Result<()>;
/// Watch path for changes, returns event stream async fn watch(&self, path: &Path) -> Result<Box<dyn EventStream>>;
/// Batch read multiple files (optimized for io_uring)
async fn batch_read(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>>;
/// Batch write multiple files async fn batch_write(&self, ops: &[WriteOp]) -> Result<()>;
/// Get backend name for diagnostics fn backend_name(&self) -> &'static str;
/// Check if this backend supports the current platform fn is_available() -> bool where Self: Sized;
}
/// Write operation for batch writes pub struct WriteOp { pub path: PathBuf, pub data: Vec<u8>, pub sync: bool, // fsync after write }
/// Event stream for file watching


#[async_trait]


pub trait EventStream: Send { async fn next(&mut self) -> Option<FileEvent>;
fn close(&mut self);
}
/// File system event pub struct FileEvent { pub path: PathBuf, pub kind: FileEventKind, pub timestamp: Instant, }
pub enum FileEventKind { Created, Modified, Deleted, Renamed { from: PathBuf }, Metadata, }
```

### 2. io_uring Backend (`src/platform_io/io_uring.rs`)

Linux-specific implementation using io_uring for maximum performance:
```rust


#[cfg(target_os = "linux")]


pub struct IoUringBackend { ring: IoUring, pending_ops: DashMap<u64, oneshot::Sender<Result<i32>>>, next_user_data: AtomicU64, completion_thread: Option<JoinHandle<()>>, }
impl IoUringBackend { pub fn new(queue_depth: u32) -> Result<Self> { let ring = IoUring::builder()
.setup_sqpoll(1000) // Kernel-side polling .setup_iopoll() // Use polling for block devices .build(queue_depth)?;
// ...
}
/// Submit batch of read operations async fn submit_batch_reads(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>> { // Prepare SQEs for all reads // Submit in single syscall // Wait for completions }
}
```

### 3. kqueue Backend (`src/platform_io/kqueue.rs`)

macOS-specific implementation using kqueue:
```rust


#[cfg(target_os = "macos")]


pub struct KqueueBackend { kq: RawFd, event_buffer: Vec<kevent>, watched_fds: DashMap<PathBuf, RawFd>, }
impl KqueueBackend { pub fn new() -> Result<Self> { let kq = unsafe { kqueue() };
if kq < 0 { return Err(anyhow!("Failed to create kqueue"));
}
// ...
}
}
```

### 4. IOCP Backend (`src/platform_io/iocp.rs`)

Windows-specific implementation using I/O Completion Ports:
```rust


#[cfg(target_os = "windows")]


pub struct IocpBackend { completion_port: HANDLE, worker_threads: Vec<JoinHandle<()>>, pending_ops: DashMap<usize, oneshot::Sender<Result<u32>>>, }
impl IocpBackend { pub fn new(thread_count: usize) -> Result<Self> { let completion_port = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, null_mut(), 0, thread_count as u32)
};
// ...
}
}
```

### 5. Fallback Backend (`src/platform_io/fallback.rs`)

Tokio-based fallback for unsupported platforms or when native APIs fail:
```rust
pub struct FallbackBackend { runtime: Handle, }
impl FallbackBackend { pub fn new() -> Self { Self { runtime: Handle::current(), }
}
}


#[async_trait]


impl PlatformIO for FallbackBackend { async fn read(&self, path: &Path, buf: &mut [u8]) -> Result<usize> { let mut file = tokio::fs::File::open(path).await?;
file.read(buf).await.map_err(Into::into)
}
// ... other implementations using tokio::fs }
```

### 6. Platform Selector (`src/platform_io/selector.rs`)

Automatic backend selection based on platform and availability:
```rust
pub fn create_platform_io() -> Arc<dyn PlatformIO> {


#[cfg(target_os = "linux")]


{ if IoUringBackend::is_available() { if let Ok(backend) = IoUringBackend::new(256) { tracing::info!("Using io_uring backend");
return Arc::new(backend);
}
}
}


#[cfg(target_os = "macos")]


{ if KqueueBackend::is_available() { if let Ok(backend) = KqueueBackend::new() { tracing::info!("Using kqueue backend");
return Arc::new(backend);
}
}
}


#[cfg(target_os = "windows")]


{ if IocpBackend::is_available() { if let Ok(backend) = IocpBackend::new(num_cpus::get()) { tracing::info!("Using IOCP backend");
return Arc::new(backend);
}
}
}
tracing::info!("Using fallback (tokio) backend");
Arc::new(FallbackBackend::new())
}
```

### 7. Enhanced Error Handler (`src/error.rs` updates)

```rust
/// Enhanced error with full context pub struct ForgeError { pub kind: ErrorKind, pub message: String, pub source: Option<Box<dyn std::error::Error + Send + Sync>>, pub context: ErrorContext, pub backtrace: Option<Backtrace>, }
pub struct ErrorContext { pub file: Option<PathBuf>, pub operation: String, pub timestamp: DateTime<Utc>, pub retry_count: u32, pub platform: &'static str, pub backend: &'static str, }
impl ForgeError { pub fn is_retryable(&self) -> bool { matches!(self.kind, ErrorKind::Network |
ErrorKind::Timeout | ErrorKind::TemporaryFailure )
}
pub fn suggestions(&self) -> Vec<String> { // Return context-specific suggestions }
}
```

### 8. Resource Manager (`src/resource_manager.rs`)

Centralized resource management with limits and cleanup:
```rust
pub struct ResourceManager { file_handles: Semaphore, max_handles: usize, active_handles: AtomicUsize, shutdown_signal: broadcast::Sender<()>, }
impl ResourceManager { pub fn new(max_handles: usize) -> Self { Self { file_handles: Semaphore::new(max_handles), max_handles, active_handles: AtomicUsize::new(0), shutdown_signal: broadcast::channel(1).0, }
}
pub async fn acquire_handle(&self) -> Result<HandleGuard> { let permit = self.file_handles.acquire().await?;
self.active_handles.fetch_add(1, Ordering::SeqCst);
Ok(HandleGuard { permit, manager: self })
}
pub async fn shutdown(&self, timeout: Duration) -> Result<()> { let _ = self.shutdown_signal.send(());
// Wait for all handles to be released let deadline = Instant::now() + timeout;
while self.active_handles.load(Ordering::SeqCst) > 0 { if Instant::now() > deadline { tracing::warn!("Shutdown timeout, {} handles still active", self.active_handles.load(Ordering::SeqCst));
return Err(anyhow!("Shutdown timeout"));
}
tokio::time::sleep(Duration::from_millis(10)).await;
}
Ok(())
}
}
```

### 9. Configuration Validator (`src/config/validator.rs`)

```rust
pub struct ConfigValidator { errors: Vec<ValidationError>, }
impl ConfigValidator { pub fn validate(config: &ForgeConfig) -> Result<(), Vec<ValidationError>> { let mut validator = Self { errors: vec![] };
validator.validate_paths(config);
validator.validate_limits(config);
validator.validate_network(config);
validator.validate_timeouts(config);
if validator.errors.is_empty() { Ok(())
} else { Err(validator.errors)
}
}
fn validate_paths(&mut self, config: &ForgeConfig) { if !config.project_root.exists() { self.errors.push(ValidationError { field: "project_root".to_string(), message: format!("Path does not exist: {}", config.project_root.display()), suggestion: "Ensure the project directory exists".to_string(), });
}
// ... more path validations }
}
```

### 10. Metrics Collector (`src/metrics.rs`)

```rust
pub struct MetricsCollector { files_watched: AtomicU64, operations_total: AtomicU64, operations_per_second: AtomicU64, cache_hits: AtomicU64, cache_misses: AtomicU64, errors_total: AtomicU64, io_latency_histogram: Histogram, }
impl MetricsCollector { pub fn record_io_operation(&self, duration: Duration, success: bool) { self.operations_total.fetch_add(1, Ordering::Relaxed);
self.io_latency_histogram.record(duration.as_micros() as u64);
if !success { self.errors_total.fetch_add(1, Ordering::Relaxed);
}
}
pub fn export_json(&self) -> serde_json::Value { json!({ "files_watched": self.files_watched.load(Ordering::Relaxed), "operations_total": self.operations_total.load(Ordering::Relaxed), "cache_hit_rate": self.cache_hit_rate(), "errors_total": self.errors_total.load(Ordering::Relaxed), "io_latency_p50_us": self.io_latency_histogram.percentile(50.0), "io_latency_p99_us": self.io_latency_histogram.percentile(99.0), })
}
}
```

## Data Models

### Platform Detection

```rust


#[derive(Debug, Clone, Copy, PartialEq, Eq)]


pub enum Platform { Linux, MacOS, Windows, Unknown, }


#[derive(Debug, Clone, Copy, PartialEq, Eq)]


pub enum IoBackend { IoUring, Kqueue, Iocp, Fallback, }
pub struct PlatformInfo { pub platform: Platform, pub backend: IoBackend, pub kernel_version: Option<String>, pub features: HashSet<String>, }
```

### Configuration

```rust


#[derive(Debug, Clone, Serialize, Deserialize)]


pub struct IoConfig { /// Maximum concurrent file handles pub max_file_handles: usize, /// io_uring queue depth (Linux only)
pub io_uring_queue_depth: u32, /// IOCP thread count (Windows only)
pub iocp_threads: usize, /// Enable direct I/O for large files pub direct_io_threshold: usize, /// Memory map threshold pub mmap_threshold: usize, /// Batch operation size pub batch_size: usize, /// Operation timeout pub operation_timeout: Duration, /// Retry policy pub retry_policy: RetryPolicy, }
impl Default for IoConfig { fn default() -> Self { Self { max_file_handles: 1024, io_uring_queue_depth: 256, iocp_threads: num_cpus::get(), direct_io_threshold: 1024 * 1024, // 1MB mmap_threshold: 1024 * 1024, // 1MB batch_size: 64, operation_timeout: Duration::from_secs(30), retry_policy: RetryPolicy::default(), }
}
}
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Platform Detection Correctness

For any execution of the platform detection logic, the detected platform SHALL match the actual operating system the code is running on. Validates: Requirements 1.1

### Property 2: Fallback Behavior Guarantee

For any platform where the native I/O backend initialization fails or is unavailable, the system SHALL successfully fall back to the tokio-based backend and all I/O operations SHALL complete successfully. Validates: Requirements 1.5

### Property 3: API Consistency Across Backends

For any I/O operation (read, write, batch_read, batch_write) and any valid input, all backends (io_uring, kqueue, IOCP, fallback) SHALL produce equivalent results when operating on the same data. Validates: Requirements 1.6

### Property 4: Batch Operation Correctness

For any set of file paths and corresponding data, a batch write followed by a batch read SHALL return data equivalent to the original input for all files. Validates: Requirements 1.7

### Property 5: Watcher Scalability

For any set of up to 10,000 files being watched, the watcher SHALL successfully register all watches and detect changes to any file within the set. Validates: Requirements 2.4

### Property 6: Event Debouncing and Deduplication

For any sequence of N rapid changes to the same file within the debounce window, the watcher SHALL emit at most 1 event for that file. Validates: Requirements 2.5, 2.6

### Property 7: Concurrent Storage Operations

For any set of concurrent read and write operations to different blobs, all operations SHALL complete successfully without data corruption, and the final state SHALL be consistent with some sequential ordering of the operations. Validates: Requirements 3.3

### Property 8: Blob Integrity Round-Trip

For any valid blob content, storing the blob and then reading it back SHALL return content identical to the original, and the SHA-256 checksum SHALL match. Validates: Requirements 3.5

### Property 9: Error Categorization Completeness

For any error produced by the system, the error categorization function SHALL assign exactly one category from {Network, FileSystem, Configuration, Validation, Dependency, Timeout, Unknown}. Validates: Requirements 5.1

### Property 10: Exponential Backoff Retry

For any retryable error with retry policy configured for N attempts, the delay between attempt i and attempt i+1 SHALL be greater than or equal to the delay between attempt i-1 and attempt i (exponential growth), up to the maximum delay. Validates: Requirements 5.2

### Property 11: Error Context Completeness

For any error logged by the system, the error record SHALL contain: a non-empty message, a valid category, a timestamp, and context-specific suggestions (non-empty list). Validates: Requirements 5.3, 5.4, 5.5

### Property 12: File Handle Limiting

For any sequence of file operations, the number of concurrently held file handles SHALL never exceed the configured maximum limit. Validates: Requirements 6.1

### Property 13: Handle Queuing at Limit

For any operation requested when the file handle limit is reached, the operation SHALL be queued and SHALL complete successfully once a handle becomes available. Validates: Requirements 6.2

### Property 14: Watcher Handle Cleanup

For any watcher that is started and then stopped, the number of registered watch handles after stop SHALL be zero. Validates: Requirements 6.5

### Property 15: Configuration Validation Completeness

For any configuration with invalid values (missing required fields, out-of-range values, non-existent paths, malformed addresses), the validator SHALL return an error containing a description of each invalid field and the valid constraints. Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5, 7.6

### Property 16: Structured Logging Format

For any log entry emitted by the system, the entry SHALL be valid JSON containing at minimum: timestamp, level, and message fields. Validates: Requirements 8.1, 8.3

### Property 17: Log Level Filtering

For any configured log level L, only log entries with level >= L SHALL be emitted (where trace < debug < info < warn < error). Validates: Requirements 8.2

### Property 18: Metrics Availability

For any running Forge instance, the metrics endpoint SHALL return valid values for: files_watched, operations_total, cache_hit_rate, and errors_total. Validates: Requirements 8.4

### Property 19: Slow Operation Warning

For any I/O operation that takes longer than the configured threshold, a warning log entry SHALL be emitted containing the operation type and actual duration. Validates: Requirements 8.5

### Property 20: Log Rotation

For any log file that exceeds 100MB, the system SHALL rotate the log file before writing additional entries. Validates: Requirements 8.6

### Property 21: Graceful Shutdown Completeness

For any graceful shutdown initiated, all in-flight write operations SHALL complete, all pending log entries SHALL be flushed, and the current state SHALL be persisted to disk before the process exits. Validates: Requirements 9.2, 9.3, 9.4

### Property 22: Exit Code Correctness

For any process exit, the exit code SHALL be 0 if and only if shutdown was clean (no errors during shutdown), and non-zero otherwise. Validates: Requirements 9.6

### Property 23: Concurrent Read Support

For any set of concurrent read operations to the same blob from multiple threads, all reads SHALL complete successfully and return identical data. Validates: Requirements 10.1

### Property 24: Write Serialization

For any set of concurrent write operations to the same blob, the final blob content SHALL be equal to the content from exactly one of the write operations (last-writer-wins or first-writer-wins, consistently). Validates: Requirements 10.2

### Property 25: Thread-Safe Watcher Operations

For any sequence of start and stop operations on a watcher from arbitrary threads, the watcher SHALL maintain consistent state and not exhibit undefined behavior. Validates: Requirements 10.3

### Property 26: Connection Pool Sizing

For any configured pool size N, the database connection pool SHALL maintain at most N active connections at any time. Validates: Requirements 10.5

## Error Handling

### Error Categories and Recovery

t:0(Category,Retryable,Max,Retries,Recovery,Strategy)[]

### Platform-Specific Error Handling

```rust


#[cfg(target_os = "linux")]


fn handle_io_uring_error(err: io_uring::Error) -> ForgeError { match err { io_uring::Error::SqFull => ForgeError::temporary("io_uring submission queue full"), io_uring::Error::CqOverflow => ForgeError::temporary("io_uring completion queue overflow"), _ => ForgeError::from(err), }
}


#[cfg(target_os = "windows")]


fn handle_iocp_error(err: windows::Error) -> ForgeError { match err.code() { ERROR_IO_PENDING => ForgeError::temporary("I/O operation pending"), ERROR_OPERATION_ABORTED => ForgeError::cancelled("Operation aborted"), _ => ForgeError::from(err), }
}
```

### Panic Recovery

```rust
pub fn install_panic_handler() { let default_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |panic_info| { // Attempt to release critical resources if let Some(resource_manager) = GLOBAL_RESOURCE_MANAGER.get() { let _ = resource_manager.emergency_cleanup();
}
// Log panic with full context tracing::error!( panic = true, location = ?panic_info.location(), payload = ?panic_info.payload().downcast_ref::<&str>(), "Panic occurred, attempting graceful degradation"
);
// Call default hook default_hook(panic_info);
}));
}
```

## Testing Strategy

### Dual Testing Approach

This implementation requires both unit tests and property-based tests: -Unit tests: Verify specific examples, edge cases, and platform-specific behavior -Property tests: Verify universal properties across all valid inputs using randomized testing

### Property-Based Testing Framework

We will use the `proptest` crate for property-based testing in Rust:
```rust
use proptest::prelude::*;
proptest! {


#![proptest_config(ProptestConfig::with_cases(100))]


// Feature: platform-native-io-hardening, Property 8: Blob Integrity Round-Trip


#[test]


fn prop_blob_roundtrip(content in prop::collection::vec(any::<u8>(), 0..10000)) { let blob = Blob::from_content("test.bin", content.clone());
let binary = blob.to_binary().unwrap();
let restored = Blob::from_binary(&binary).unwrap();
prop_assert_eq!(blob.content, restored.content);
prop_assert_eq!(blob.metadata.hash, restored.metadata.hash);
}
}
```

### Test Categories

- Platform Detection Tests (unit)
- Verify correct platform detection on each OS
- Verify backend selection logic
- I/O Backend Tests (property + unit)
- Round-trip read/write for all backends
- Batch operation correctness
- Concurrent operation safety
- Watcher Tests (property + unit)
- Event debouncing behavior
- Scalability with many files
- Handle cleanup on stop
- Error Handling Tests (property + unit)
- Error categorization completeness
- Retry behavior with exponential backoff
- Context completeness in error messages
- Resource Management Tests (property + unit)
- File handle limiting
- Graceful shutdown behavior
- Connection pool sizing
- Configuration Tests (property + unit)
- Validation error messages
- Range checking
- Path validation
- Stress Tests (integration)
- 1000+ concurrent operations
- 10,000+ watched files
- Long-running stability

### Platform-Specific Test Matrix

+------+----------+---------+-------+---------+
| Test | Category | Linux   | macOS | Windows |
+======+==========+=========+=======+=========+
| io   | uring    | backend | ✓     | -       |
+------+----------+---------+-------+---------+



### Test Configuration

Each property test MUST: -Run minimum 100 iterations -Be tagged with the design property reference -Use the format: `Feature: platform-native-io-hardening, Property N: {property_text}`
