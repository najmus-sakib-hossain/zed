
# Platform-Native I/O API Reference

DX Forge provides a unified platform-native I/O abstraction layer that automatically selects the most performant backend for your operating system.

## Overview

The platform I/O system provides: -Automatic backend selection - io_uring on Linux, kqueue on macOS, IOCP on Windows -Graceful fallback - Falls back to tokio when native APIs are unavailable -Batch operations - Optimized for high-throughput scenarios -File watching - Platform-native file system monitoring

## Supported Platforms

+----------+---------+---------+---------+----------+
| Platform | Backend | Minimum | Version | Features |
+==========+=========+=========+=========+==========+
| Linux    | io      | uring   | Kernel  | 5.1+     |
+----------+---------+---------+---------+----------+



## Core Types

### `PlatformIO` Trait

The main trait for platform-native I/O operations:
```rust


#[async_trait]


pub trait PlatformIO: Send + Sync { /// Read file contents into buffer async fn read(&self, path: &Path, buf: &mut [u8]) -> Result<usize>;
/// Write buffer contents to file async fn write(&self, path: &Path, buf: &[u8]) -> Result<usize>;
/// Read entire file into Vec async fn read_all(&self, path: &Path) -> Result<Vec<u8>>;
/// Write entire buffer to file async fn write_all(&self, path: &Path, buf: &[u8]) -> Result<()>;
/// Watch path for changes async fn watch(&self, path: &Path) -> Result<Box<dyn EventStream>>;
/// Batch read multiple files async fn batch_read(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>>;
/// Batch write multiple files async fn batch_write(&self, ops: &[WriteOp]) -> Result<()>;
/// Get backend name for diagnostics fn backend_name(&self) -> &'static str;
/// Check if this backend is available fn is_available() -> bool where Self: Sized;
}
```

### `WriteOp` Struct

Represents a write operation for batch writes:
```rust
pub struct WriteOp { pub path: PathBuf, pub data: Vec<u8>, pub sync: bool, // fsync after write }
impl WriteOp { pub fn new(path: impl Into<PathBuf>, data: Vec<u8>) -> Self;
pub fn with_sync(path: impl Into<PathBuf>, data: Vec<u8>) -> Self;
}
```

### `FileEvent` Struct

Represents a file system event:
```rust
pub struct FileEvent { pub path: PathBuf, pub kind: FileEventKind, pub timestamp: Instant, }
pub enum FileEventKind { Created, Modified, Deleted, Renamed { from: PathBuf }, Metadata, }
```

### `Platform` Enum

Detected operating system:
```rust
pub enum Platform { Linux, MacOS, Windows, Unknown, }
impl Platform { pub fn current() -> Self;
pub fn name(&self) -> &'static str;
}
```

### `IoBackend` Enum

Active I/O backend:
```rust
pub enum IoBackend { IoUring, Kqueue, Iocp, Fallback, }
impl IoBackend { pub fn name(&self) -> &'static str;
}
```

## Functions

### `create_platform_io()`

Creates a platform I/O instance with automatic backend selection:
```rust
pub fn create_platform_io() -> Arc<dyn PlatformIO> ```
Example:
```rust
let io = create_platform_io();
println!("Using backend: {}", io.backend_name());
```


## Resource Manager



### `ResourceManager`


Manages file handles and system resources:
```rust
pub struct ResourceManager { // ...
}
impl ResourceManager { pub fn new(max_handles: usize) -> Self;
pub async fn acquire_handle(&self) -> Result<HandleGuard>;
pub fn active_count(&self) -> usize;
pub async fn shutdown(&self, timeout: Duration) -> Result<()>;
}
```


### `HandleGuard`


RAII guard for automatic handle release:
```rust
pub struct HandleGuard<'a> { // Automatically releases handle when dropped }
```


## Configuration Validator



### `ConfigValidator`


Validates configuration at startup:
```rust
pub struct ConfigValidator;
impl ConfigValidator { pub fn validate(config: &ForgeConfig) -> ValidationResult;
pub fn validate_path(path: &Path) -> Result<(), ValidationError>;
pub fn validate_range<T: Ord>(value: T, min: T, max: T, field: &str) -> Result<(), ValidationError>;
pub fn validate_network_address(addr: &str) -> Result<(), ValidationError>;
}
```


### `ValidationError`


Describes a validation failure:
```rust
pub struct ValidationError { pub field: String, pub message: String, pub suggestion: String, }
```


## Metrics Collector



### `MetricsCollector`


Collects I/O and performance metrics:
```rust
pub struct MetricsCollector { // ...
}
impl MetricsCollector { pub fn new() -> Self;
pub fn record_io_operation(&self, duration: Duration, success: bool);
pub fn record_cache_hit(&self);
pub fn record_cache_miss(&self);
pub fn export_json(&self) -> serde_json::Value;
pub fn reset(&self);
}
```
Exported Metrics: -`files_watched` - Number of files being watched -`operations_total` - Total I/O operations performed -`cache_hit_rate` - Cache hit percentage (0-100) -`errors_total` - Total errors encountered -`io_latency_p50_us` - 50th percentile latency in microseconds -`io_latency_p99_us` - 99th percentile latency in microseconds


## Shutdown Handler



### `ShutdownHandler`


Manages graceful shutdown:
```rust
pub struct ShutdownHandler { // ...
}
impl ShutdownHandler { pub fn new(config: ShutdownConfig) -> Self;
pub fn initiate_shutdown(&self);
pub fn is_shutting_down(&self) -> bool;
pub fn subscribe(&self) -> broadcast::Receiver<()>;
pub async fn wait_for_completion(&self, timeout: Duration) -> Result<()>;
}
```


### `ShutdownConfig`


Configuration for shutdown behavior:
```rust
pub struct ShutdownConfig { pub timeout: Duration, // Default: 30 seconds pub force_after_timeout: bool, // Default: true pub flush_logs: bool, // Default: true pub save_state: bool, // Default: true }
```


### `ExitCode`


Standard exit codes:
```rust
pub enum ExitCode { Success = 0, GeneralError = 1, ConfigurationError = 2, IoError = 3, TimeoutError = 4, ShutdownTimeout = 5, }
```


## Examples



### Basic File Operations


```rust
use dx_forge::{create_platform_io, PlatformIO};
use std::path::Path;

#[tokio::main]

async fn main() -> anyhow::Result<()> { let io = create_platform_io();
// Write a file io.write_all(Path::new("hello.txt"), b"Hello, World!").await?;
// Read it back let content = io.read_all(Path::new("hello.txt")).await?;
println!("Content: {}", String::from_utf8_lossy(&content));
Ok(())
}
```


### Batch Operations


```rust
use dx_forge::{create_platform_io, WriteOp};

#[tokio::main]

async fn main() -> anyhow::Result<()> { let io = create_platform_io();
// Batch write multiple files let ops = vec![ WriteOp::new("file1.txt", b"Content 1".to_vec()), WriteOp::new("file2.txt", b"Content 2".to_vec()), WriteOp::with_sync("important.txt", b"Critical data".to_vec()), ];
io.batch_write(&ops).await?;
// Batch read let paths = vec![ "file1.txt".into(), "file2.txt".into(), ];
let contents = io.batch_read(&paths).await?;
Ok(())
}
```


### Resource Management


```rust
use dx_forge::ResourceManager;
use std::time::Duration;

#[tokio::main]

async fn main() -> anyhow::Result<()> { let manager = ResourceManager::new(1024);
// Acquire a handle let guard = manager.acquire_handle().await?;
// Do work with the handle...
// Handle is automatically released when guard is dropped drop(guard);
// Graceful shutdown manager.shutdown(Duration::from_secs(5)).await?;
Ok(())
}
```


### Metrics Collection


```rust
use dx_forge::MetricsCollector;
use std::time::Instant;
fn main() { let metrics = MetricsCollector::new();
// Record operations let start = Instant::now();
// ... do I/O ...
metrics.record_io_operation(start.elapsed(), true);
// Export metrics let stats = metrics.export_json();
println!("{}", serde_json::to_string_pretty(&stats).unwrap());
}
```


## Error Handling


All I/O operations return `anyhow::Result` with detailed error context:
```rust
use dx_forge::{create_platform_io, categorize_error, ErrorCategory};

#[tokio::main]

async fn main() { let io = create_platform_io();
match io.read_all(Path::new("nonexistent.txt")).await { Ok(content) => println!("Read {} bytes", content.len()), Err(e) => { let category = categorize_error(&e);
match category { ErrorCategory::FileSystem => println!("File system error: {}", e), ErrorCategory::Timeout => println!("Operation timed out"), _ => println!("Error: {}", e), }
}
}
}
```


## Thread Safety


All platform I/O backends are `Send + Sync` and can be safely shared across threads:
```rust
use dx_forge::create_platform_io;
use std::sync::Arc;

#[tokio::main]

async fn main() { let io = Arc::new(create_platform_io());
let handles: Vec<_> = (0..10)
.map(|i| { let io = Arc::clone(&io);
tokio::spawn(async move { let path = format!("file_{}.txt", i);
io.write_all(Path::new(&path), b"content").await })
})
.collect();
for handle in handles { handle.await.unwrap().unwrap();
}
}
```


## Correctness Guarantees


The platform I/O layer is validated by property-based tests ensuring: -Platform Detection - Detected platform always matches actual OS -Fallback Guarantee - System always falls back to tokio when native APIs unavailable -API Consistency - All backends produce equivalent results for same operations -Batch Correctness - Batch write followed by batch read returns original data -Handle Limiting - Concurrent handles never exceed configured maximum -Handle Queuing - Operations queue when at limit and complete when handles available -Metrics Availability - Metrics always return valid values -Graceful Shutdown - All in-flight operations complete before shutdown


## Performance Characteristics


+---------+-------+------------+---------+---------+
| Backend | Batch | Throughput | Latency | (P99    |
+=========+=======+============+=========+=========+
| io      | uring | ~500K      | ops/sec | <100μs  |
+---------+-------+------------+---------+---------+
