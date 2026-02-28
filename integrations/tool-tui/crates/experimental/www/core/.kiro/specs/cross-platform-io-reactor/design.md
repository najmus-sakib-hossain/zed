
# Design Document: Cross-Platform I/O Reactor

## Overview

The dx-reactor is the foundational I/O layer for dx-www, providing platform-optimized async I/O that enables the framework to achieve its performance targets. The design follows a unified abstraction pattern where platform-specific backends (io_uring, kqueue, IOCP) implement a common `Reactor` trait, allowing the rest of the system to be platform-agnostic. The reactor integrates with dx-www's binary-first architecture through: -HBTP Protocol: Binary communication protocol replacing HTTP for dx-www traffic -Memory Teleportation: Zero-copy serialization where wire format equals memory layout -Compiler-Inlined Middleware: Zero-overhead middleware via compile-time inlining -Thread-Per-Core Architecture: One event loop per CPU core with work-stealing

## Architecture

@tree[]

## Components and Interfaces

### Reactor Trait

```rust
/// Unified I/O reactor trait for all platform-specific backends pub trait Reactor: Send + Sync + 'static { type Handle: IoHandle;
/// Create new reactor instance fn new(config: ReactorConfig) -> io::Result<Self> where Self: Sized;
/// Register a file descriptor for events fn register(&self, fd: RawFd, interest: Interest) -> io::Result<Self::Handle>;
/// Submit pending operations (batch)
fn submit(&self) -> io::Result<usize>;
/// Wait for completions fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Completion>>;
/// Submit and wait (optimized path)
fn submit_and_wait(&self, min_complete: usize) -> io::Result<Vec<Completion>>;
}
/// Platform-specific reactor selection at compile time


#[cfg(all(target_os = "linux", feature = "io_uring"))]


pub type PlatformReactor = uring::UringReactor;


#[cfg(all(target_os = "linux", not(feature = "io_uring")))]


pub type PlatformReactor = epoll::EpollReactor;


#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]


pub type PlatformReactor = kqueue::KqueueReactor;


#[cfg(target_os = "windows")]


pub type PlatformReactor = iocp::IocpReactor;
```

### ReactorConfig

```rust
/// Configuration for reactor initialization


#[derive(Clone, Debug)]


pub struct ReactorConfig { /// Number of submission queue entries pub entries: u32, /// Enable kernel-side polling (Linux io_uring only)
pub sqpoll: bool, /// SQPOLL idle timeout in milliseconds pub sqpoll_idle_ms: u32, /// CPU to pin SQPOLL thread to pub sqpoll_cpu: Option<u32>, /// Enable zero-copy I/O pub zero_copy: bool, /// Buffer size for registered buffers pub buffer_size: usize, /// Number of registered buffers pub buffer_count: usize, /// Concurrency hint for IOCP pub concurrency_hint: usize, }
```

### Completion Structure

```rust
/// I/O completion result


#[derive(Clone, Debug)]


pub struct Completion { /// User data associated with the operation pub user_data: u64, /// Result code (bytes transferred or error)
pub result: i32, /// Platform-specific flags pub flags: u32, }
```

### HBTP Protocol

```rust
/// HBTP OpCodes - 1 byte for common operations


#[repr(u8)]


pub enum HbtpOpcode { Ping = 0x00, Pong = 0x01, Close = 0x02, StateSync = 0x10, StateDelta = 0x11, RpcCall = 0x30, RpcResponse = 0x31, ClientEvent = 0x40, ServerEvent = 0x41, // ... more opcodes }
/// HBTP Message Header (8 bytes)


#[repr(C, packed)]


pub struct HbtpHeader { pub opcode: HbtpOpcode, pub flags: HbtpFlags, pub sequence: u16, pub length: u32, }
bitflags! { pub struct HbtpFlags: u8 { const COMPRESSED = 0b0000_0001;
const ENCRYPTED = 0b0000_0010;
const EXPECTS_RESPONSE = 0b0000_0100;
const FINAL = 0b0000_1000;
}
}
```

### Memory Teleportation

```rust
/// Marker trait for types that can be teleported (zero-copy serialization)
pub unsafe trait Teleportable: Copy + 'static { const LAYOUT: TeleportLayout;
}
/// Zero-copy teleport buffer pub struct TeleportBuffer { buffer: Vec<u8>, position: usize, string_table_start: usize, strings: Vec<u8>, }
impl TeleportBuffer { /// Write a teleportable value with correct alignment pub fn write<T: Teleportable>(&mut self, value: &T);
/// Write a string to the string table pub fn write_string(&mut self, s: &str) -> (u32, u32);
/// Finalize and get bytes pub fn finalize(&mut self) -> &[u8];
}
/// Zero-copy reader pub struct TeleportReader<'a> { buffer: &'a [u8], position: usize, string_table_offset: usize, }
impl<'a> TeleportReader<'a> { /// Read a teleportable value (zero-copy, returns reference)
pub fn read<T: Teleportable>(&mut self) -> &'a T;
/// Read a string by offset/length pub fn read_string(&self, offset: u32, len: u32) -> &'a str;
}
```

### Compiler-Inlined Middleware

```rust
/// Middleware trait for compile-time inlining pub trait Middleware: Sized + 'static { fn before(req: &mut Request) -> MiddlewareResult<()>;
fn after(req: &Request, res: &mut Response);
}
/// Macro to generate inlined middleware chain


#[macro_export]


macro_rules! dx_middleware { ($($middleware:ty),* $(,)?) => { move |req: &mut Request, handler: fn(&Request) -> Response| -> Response {
// Before hooks (all inlined)
$( match <$middleware>::before(req) { Ok(()) => {}, Err(e) => return e.into_response(), }
)* let mut res = handler(req);
// After hooks (reverse order)
dx_middleware!(@reverse_after req, res, $($middleware),*);
res }
};
}
```

### DxReactor (Main Entry Point)

```rust
/// The Binary Dawn Reactor pub struct DxReactor { config: ReactorConfig, cores: Vec<CoreState>, protocol: Arc<HbtpProtocol>, router: CompiledRouter, }
impl DxReactor { /// Build a new reactor with the builder pattern pub fn build() -> ReactorBuilder;
/// Start the reactor (blocking)
pub fn ignite(self) -> !;
}
pub struct ReactorBuilder { workers: WorkerStrategy, io_backend: Option<IoBackend>, teleport: bool, hbtp: bool, buffer_size: usize, buffer_count: usize, }
pub enum WorkerStrategy { ThreadPerCore, Fixed(usize), }
```

## Data Models

### CoreState

```rust
/// Per-core state for thread-per-core architecture pub struct CoreState { /// Core ID pub id: usize, /// Platform-specific reactor pub reactor: PlatformReactor, /// Local work queue pub queue: LocalQueue<Task>, /// Response buffer (reused)
pub response_buffer: ResponseBuffer, }
```

### CacheEntry (DbTeleport)

```rust
/// Binary cache entry for database teleport pub struct CacheEntry { /// Pre-serialized HTIP binary pub binary: Arc<[u8]>, /// Version for conditional requests pub version: u64, /// Timestamp pub updated_at: Instant, }
pub struct CacheKey { query_id: String, params_hash: u64, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: HBTP Header Size Invariant

For any HbtpHeader instance, `size_of::<HbtpHeader>()` SHALL equal exactly 8 bytes. Validates: Requirements 6.2

### Property 2: HBTP Message Round-Trip

For any valid HBTP message (with or without compression/encryption), serializing then deserializing SHALL produce an equivalent message with identical opcode, flags, sequence, length, and payload. Validates: Requirements 6.3, 6.4, 6.6

### Property 3: HBTP Route Lookup O(1)

For any number of registered routes N, route lookup time SHALL remain constant (O(1)) because routes are indexed by array position, not searched. Validates: Requirements 6.5

### Property 4: Teleportation Round-Trip

For any Teleportable type T and value v of type T, writing v to a TeleportBuffer and reading it back with TeleportReader SHALL produce a value equivalent to v, with correct alignment preserved. Validates: Requirements 7.1, 7.2, 7.4

### Property 5: Middleware After Hook Order

For any middleware chain [M1, M2,..., Mn], after hooks SHALL execute in reverse order [Mn,..., M2, M1] relative to before hooks. Validates: Requirements 8.3

### Property 6: Cache Lookup O(1)

For any cache size N, DbTeleport cache lookup time SHALL remain constant (O(1)) using hash-based indexing. Validates: Requirements 9.4

### Property 7: Binary Format Cross-Platform Compatibility

For any Teleportable type T, the binary representation produced on one platform SHALL be identical to the representation produced on any other supported platform (little-endian, same sizes). Validates: Requirements 10.4

### Property 8: Worker Count Configuration

For any configured worker count N (where N > 0), DxReactor SHALL spawn exactly N worker threads. Validates: Requirements 5.2

## Error Handling

### Reactor Errors

```rust


#[derive(Debug, thiserror::Error)]


pub enum ReactorError {


#[error("io_uring not available on this system")]


UringNotAvailable,


#[error("Failed to create reactor: {0}")]


CreationFailed(#[from] io::Error),


#[error("Submission queue full")]


SubmissionQueueFull,


#[error("Invalid configuration: {0}")]


InvalidConfig(String), }
```

### HBTP Errors

```rust


#[derive(Debug, thiserror::Error)]


pub enum HbtpError {


#[error("Invalid header")]


InvalidHeader,


#[error("Invalid payload")]


InvalidPayload,


#[error("Route not found")]


RouteNotFound,


#[error("Unknown opcode: {0}")]


UnknownOpcode(u8),


#[error("Decompression failed")]


DecompressionFailed,


#[error("Decryption failed")]


DecryptionFailed, }
```

### Middleware Errors

```rust


#[derive(Debug, thiserror::Error)]


pub enum MiddlewareError {


#[error("Unauthorized")]


Unauthorized,


#[error("Rate limited")]


RateLimited,


#[error("Forbidden")]


Forbidden, }
```

## Testing Strategy

### Dual Testing Approach

The testing strategy combines unit tests for specific examples and edge cases with property-based tests for universal correctness properties.

### Unit Tests

- Test each reactor backend on its native platform
- Test HBTP opcode definitions and header parsing
- Test middleware hook execution order
- Test cache invalidation on table changes
- Test platform detection functions

### Property-Based Tests

Using `proptest` crate with minimum 100 iterations per property: -HBTP Header Size: Verify header is always 8 bytes -HBTP Round-Trip: Generate random messages, serialize/deserialize -Teleportation Round-Trip: Generate random Teleportable values, verify round-trip -Middleware Order: Generate random middleware chains, verify after hook order -Worker Count: Generate random worker counts, verify thread spawning -Binary Compatibility: Generate values, verify identical binary on all platforms

### Integration Tests

- Cross-platform binary format compatibility tests
- End-to-end reactor tests with real I/O operations
- HBTP protocol tests with compression and encryption
- Database teleport cache invalidation tests

### Test Configuration

```rust
proptest! {


#![proptest_config(ProptestConfig::with_cases(100))]


// Property tests here }
```
Each property test must be tagged with:
```rust
// **Feature: cross-platform-io-reactor, Property N: Property Title** // **Validates: Requirements X.Y** ```
