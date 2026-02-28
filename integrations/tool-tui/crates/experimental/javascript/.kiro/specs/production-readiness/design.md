
# Design Document: DX-JS Production Readiness

## Overview

This design document outlines the technical approach for making DX-JS production-ready. The implementation addresses critical gaps in JavaScript language support (BigInt, dynamic import), Node.js API compatibility (fs.watch, HTTP server, streams, crypto), ecosystem validation, binary distribution, and developer experience improvements. The design leverages the existing Cranelift JIT architecture while extending the value representation system to support BigInt. Node.js compatibility is achieved through the existing `dx-compat-node` crate with new modules for file watching, complete HTTP server, streams, and crypto APIs.

## Architecture

### Current Architecture Overview

@tree[]

### Extended Architecture for Production Readiness

@tree[]

## Components and Interfaces

### 1. BigInt Implementation

#### Value Representation Extension

The current JIT uses NaN-boxing with f64 for all values. BigInt requires extending this:
```rust
// Current value tagging in codegen.rs const STRING_TAG_OFFSET: f64 = 1_000_000.0;
// New BigInt tagging const BIGINT_TAG_OFFSET: f64 = 2_000_000.0;
fn is_bigint_id(value: f64) -> bool { value < -BIGINT_TAG_OFFSET + 1.0 && value >= -BIGINT_TAG_OFFSET - 1_000_000.0 && value.fract() == 0.0 }
fn encode_bigint_id(id: u64) -> f64 {
- (id as f64 + BIGINT_TAG_OFFSET)
}
fn decode_bigint_id(value: f64) -> u64 { (-(value + BIGINT_TAG_OFFSET)) as u64 }
```

#### RuntimeHeap Extension

```rust
struct RuntimeHeap { // Existing fields...
strings: HashMap<u64, String>, arrays: HashMap<u64, Vec<f64>>, objects: HashMap<u64, HashMap<String, f64>>, closures: HashMap<u64, ClosureData>, // New BigInt storage bigints: HashMap<u64, num_bigint::BigInt>, next_id: u64, }
impl RuntimeHeap { fn allocate_bigint(&mut self, value: num_bigint::BigInt) -> u64 { let id = self.next_id;
self.next_id += 1;
self.bigints.insert(id, value);
id }
fn get_bigint(&self, id: u64) -> Option<&num_bigint::BigInt> { self.bigints.get(&id)
}
}
```

#### BigInt Built-in Functions

```rust
// Arithmetic operations extern "C" fn builtin_bigint_add(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_sub(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_mul(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_div(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_mod(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_pow(a: f64, b: f64) -> f64;
// Comparison operations extern "C" fn builtin_bigint_lt(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_gt(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_eq(a: f64, b: f64) -> f64;
// Bitwise operations extern "C" fn builtin_bigint_and(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_or(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_xor(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_not(a: f64) -> f64;
extern "C" fn builtin_bigint_shl(a: f64, b: f64) -> f64;
extern "C" fn builtin_bigint_shr(a: f64, b: f64) -> f64;
// Conversion extern "C" fn builtin_bigint_to_string(a: f64) -> f64;
extern "C" fn builtin_bigint_from_string(s: f64) -> f64;
extern "C" fn builtin_bigint_from_number(n: f64) -> f64;
```

### 2. Dynamic Import System

#### Module Loader Interface

```rust
pub struct DynamicImportLoader { /// Cache of loaded modules module_cache: HashMap<String, ModuleNamespace>, /// Pending imports (for deduplication)
pending: HashMap<String, Vec<oneshot::Sender<ModuleNamespace>>>, /// Module resolver resolver: ModuleResolver, }
impl DynamicImportLoader { /// Load a module dynamically, returning a Promise pub async fn import(&mut self, specifier: &str, referrer: &str) -> Result<ModuleNamespace, ImportError> { // 1. Resolve the specifier let resolved = self.resolver.resolve(specifier, referrer)?;
// 2. Check cache if let Some(module) = self.module_cache.get(&resolved) { return Ok(module.clone());
}
// 3. Load and compile let source = tokio::fs::read_to_string(&resolved).await?;
let module = self.compile_and_link(&source, &resolved)?;
// 4. Cache and return self.module_cache.insert(resolved, module.clone());
Ok(module)
}
}
```

#### Bundler Code Splitting

```rust
pub struct ChunkGenerator { /// Entry points for dynamic imports dynamic_entries: Vec<String>, /// Generated chunks chunks: HashMap<String, Chunk>, }
impl ChunkGenerator { /// Generate chunks for dynamic imports pub fn generate_chunks(&mut self, module_graph: &ModuleGraph) -> Vec<Chunk> { // Identify dynamic import boundaries // Generate separate chunks for each dynamic import // Handle shared dependencies }
}
```

### 3. File System Watching

#### FSWatcher Interface

```rust
pub struct FSWatcher { /// Platform-specific watcher implementation inner: notify::RecommendedWatcher, /// Event receiver rx: mpsc::Receiver<notify::Event>, /// Watched paths watched: HashSet<PathBuf>, /// Callback for events callback: Box<dyn Fn(WatchEvent) + Send>, }
pub enum WatchEvent { Change { path: PathBuf }, Rename { path: PathBuf, new_path: Option<PathBuf> }, Error { error: std::io::Error }, }
impl FSWatcher { pub fn new(callback: impl Fn(WatchEvent) + Send + 'static) -> Result<Self, WatchError>;
pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<(), WatchError>;
pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<(), WatchError>;
pub fn close(self);
}
```

#### FSWatchFile (Polling)

```rust
pub struct FSWatchFile { /// Polling interval interval: Duration, /// Watched files with last stats watched: HashMap<PathBuf, Option<Stats>>, /// Polling task handle task: Option<JoinHandle<()>>, }
impl FSWatchFile { pub fn watch(&mut self, path: impl AsRef<Path>, callback: impl Fn(Stats, Stats));
pub fn unwatch(&mut self, path: impl AsRef<Path>);
}
```

### 4. Complete HTTP Server

#### HTTP Server Interface

```rust
pub struct HttpServer { /// Underlying TCP listener listener: TcpListener, /// TLS configuration (for HTTPS)
tls_config: Option<TlsConfig>, /// Keep-alive settings keep_alive: KeepAliveConfig, /// Request handler handler: Box<dyn Fn(Request, Response) + Send + Sync>, }
pub struct Request { pub method: Method, pub url: String, pub headers: HeaderMap, pub body: Box<dyn AsyncRead + Send>, }
pub struct Response { status: u16, headers: HeaderMap, body_writer: Box<dyn AsyncWrite + Send>, chunked: bool, }
impl Response { pub fn write_head(&mut self, status: u16, headers: HeaderMap);
pub async fn write(&mut self, chunk: &[u8]) -> Result<(), IoError>;
pub async fn end(&mut self) -> Result<(), IoError>;
}
```

### 5. Stream API Completion

#### Duplex Stream

```rust
pub struct DuplexStream { readable: ReadableHalf, writable: WritableHalf, }
impl AsyncRead for DuplexStream { ... }
impl AsyncWrite for DuplexStream { ... }
```

#### Transform Stream

```rust
pub struct TransformStream<F> { transform: F, readable: ReadableHalf, writable: WritableHalf, }
impl<F: Fn(&[u8]) -> Vec<u8>> TransformStream<F> { pub fn new(transform: F) -> Self;
}
```

#### Pipeline

```rust
pub async fn pipeline<S1, S2>( source: S1, destination: S2, ) -> Result<(), PipelineError> where S1: AsyncRead + Unpin, S2: AsyncWrite + Unpin;
pub async fn pipeline_many( streams: Vec<Box<dyn Stream>>, ) -> Result<(), PipelineError>;
```

### 6. Crypto API Completion

#### Key Derivation

```rust
pub async fn pbkdf2( password: &[u8], salt: &[u8], iterations: u32, key_len: usize, digest: DigestAlgorithm, ) -> Result<Vec<u8>, CryptoError>;
pub async fn scrypt( password: &[u8], salt: &[u8], options: ScryptOptions, ) -> Result<Vec<u8>, CryptoError>;
```

#### Key Generation and Signing

```rust
pub async fn generate_key_pair( algorithm: KeyAlgorithm, options: KeyGenOptions, ) -> Result<KeyPair, CryptoError>;
pub fn sign( algorithm: SignAlgorithm, key: &PrivateKey, data: &[u8], ) -> Result<Vec<u8>, CryptoError>;
pub fn verify( algorithm: SignAlgorithm, key: &PublicKey, signature: &[u8], data: &[u8], ) -> Result<bool, CryptoError>;
```

## Data Models

### BigInt Storage

```rust
/// BigInt value stored in runtime heap pub struct BigIntValue { /// The actual big integer value value: num_bigint::BigInt, /// Cached string representation (lazily computed)
string_cache: Option<String>, }
```

### Module Namespace

```rust
/// Represents a loaded ES module's namespace object pub struct ModuleNamespace { /// Module URL/path pub url: String, /// Exported bindings pub exports: HashMap<String, Value>, /// Default export (if any)
pub default: Option<Value>, }
```

### Watch Event

```rust
/// File system watch event pub struct WatchEventData { pub event_type: WatchEventType, pub filename: Option<String>, }
pub enum WatchEventType { Change, Rename, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### BigInt Properties

Property 1: BigInt literal round-trip For any valid BigInt literal string, parsing it and converting back to string SHALL produce an equivalent representation. Validates: Requirements 1.1, 1.4 Property 2: BigInt arithmetic correctness For any two BigInt values a and b, the arithmetic operations (+, -, *, %) SHALL produce mathematically correct results. Validates: Requirements 1.2 Property 3: BigInt comparison correctness For any two BigInt values a and b, comparison operations SHALL reflect the correct mathematical ordering. Validates: Requirements 1.3 Property 4: BigInt bitwise correctness For any BigInt value, bitwise operations SHALL produce results consistent with two's complement representation. Validates: Requirements 1.5 Property 5: BigInt division error handling For any BigInt division that would produce a non-integer, the Runtime SHALL throw a RangeError. Validates: Requirements 1.6 Property 6: BigInt/Number mixing error For any arithmetic operation mixing BigInt and Number without explicit conversion, the Runtime SHALL throw a TypeError. Validates: Requirements 1.7 Property 7: BigInt constructor correctness For any valid input to BigInt(), the constructor SHALL return the correct BigInt value. Validates: Requirements 1.8

### Dynamic Import Properties

Property 8: Dynamic import resolution For any valid module specifier, `import()` SHALL return a Promise that resolves to the correct module namespace. Validates: Requirements 2.1, 2.2, 2.3 Property 9: Dynamic import error handling For any non-existent module path, `import()` SHALL reject with an appropriate error. Validates: Requirements 2.4, 2.5

### File Watching Properties

Property 10: Watch event correctness For any file modification on a watched path, the watcher SHALL invoke the callback with the correct event type ('change' or 'rename'). Validates: Requirements 3.1, 3.2, 3.3, 3.6 Property 11: Watch resource cleanup For any watcher that is closed or unwatched, no further callbacks SHALL be invoked and resources SHALL be released. Validates: Requirements 3.5, 3.8 Property 12: Watch error handling For any non-existent watched path, the watcher SHALL emit an error event. Validates: Requirements 3.7

### HTTP Server Properties

Property 13: HTTP method parsing For any valid HTTP request, the server SHALL correctly parse the HTTP method. Validates: Requirements 4.2 Property 14: HTTP response lifecycle For any response, writeHead() SHALL set headers, write() SHALL send body chunks, and end() SHALL complete the response. Validates: Requirements 4.4, 4.5, 4.6 Property 15: HTTP request body streaming For any request with a body, the body SHALL be available as a readable stream. Validates: Requirements 4.3

### Stream Properties

Property 16: Duplex stream bidirectionality For any Duplex stream, both read and write operations SHALL function correctly. Validates: Requirements 5.1 Property 17: Transform stream transformation For any data written to a Transform stream, the transformation function SHALL be applied before the data is readable. Validates: Requirements 5.2 Property 18: Pipeline correctness For any pipeline of streams, data SHALL flow from source to destination, errors SHALL propagate, and cleanup SHALL occur on completion or error. Validates: Requirements 5.3, 5.6, 5.7 Property 19: Backpressure handling For any pipeline where the destination is slower than the source, the source SHALL be paused until the destination is ready. Validates: Requirements 5.5

### Crypto Properties

Property 20: Key derivation correctness For any password and salt, pbkdf2() and scrypt() SHALL produce deterministic, correct key material. Validates: Requirements 6.1, 6.2 Property 21: Sign/verify round-trip For any data and key pair, signing then verifying SHALL return true for the original data. Validates: Requirements 6.4, 6.5 Property 22: Encrypt/decrypt round-trip For any plaintext and key, encrypting then decrypting SHALL produce the original plaintext. Validates: Requirements 6.6, 6.7 Property 23: Key pair validity For any generated key pair, the keys SHALL be valid for their intended cryptographic operations. Validates: Requirements 6.3

### Error Handling Properties

Property 24: Error source location accuracy For any JavaScript error, the reported source location SHALL accurately reflect the actual source position, including when source maps are available. Validates: Requirements 10.1, 10.2, 10.3 Property 25: Unhandled rejection reporting For any unhandled Promise rejection, the Runtime SHALL report it with the rejection reason and stack trace. Validates: Requirements 10.6

## Error Handling

### BigInt Errors

+------------+----------+-----------+-------------+---------+
| Error      | Type     | Condition | Message     | Format  |
+============+==========+===========+=============+=========+
| RangeError | Division | produces  | non-integer | "Cannot |
+------------+----------+-----------+-------------+---------+



### Dynamic Import Errors

+-------+--------+-----------+---------+---------+
| Error | Type   | Condition | Message | Format  |
+=======+========+===========+=========+=========+
| Error | Module | not       | found   | "Cannot |
+-------+--------+-----------+---------+---------+



### File Watching Errors

+--------+------+-----------+---------+--------+
| Error  | Type | Condition | Message | Format |
+========+======+===========+=========+========+
| ENOENT | Path | does      | not     | exist  |
+--------+------+-----------+---------+--------+



### HTTP Server Errors

+-------+------------+-----------+--------------+------------+
| Error | Type       | Condition | Message      | Format     |
+=======+============+===========+==============+============+
| Error | Connection | reset     | "ECONNRESET: | connection |
+-------+------------+-----------+--------------+------------+



### Crypto Errors

+-------+---------+-----------+----------+--------+
| Error | Type    | Condition | Message  | Format |
+=======+=========+===========+==========+========+
| Error | Invalid | key       | "Invalid | key    |
+-------+---------+-----------+----------+--------+



## Testing Strategy

### Dual Testing Approach

This implementation uses both unit tests and property-based tests: -Unit tests: Verify specific examples, edge cases, and error conditions -Property tests: Verify universal properties across randomly generated inputs

### Property-Based Testing Configuration

- Library: `proptest` (Rust) for runtime components
- Minimum iterations: 100 per property test
- Tag format: `Feature: production-readiness, Property N: [property_text]`

### Test Categories

#### BigInt Tests

- Unit tests for specific values (0n, 1n,
- 1n, MAX_SAFE_INTEGER + 1)
- Property tests for arithmetic, comparison, and bitwise operations
- Error condition tests for type mixing and invalid operations

#### Dynamic Import Tests

- Unit tests for specific module resolution scenarios
- Property tests for path resolution correctness
- Integration tests with real module files

#### File Watching Tests

- Unit tests for specific file operations
- Property tests for event correctness
- Platform-specific tests (Windows, macOS, Linux)

#### HTTP Server Tests

- Unit tests for request parsing
- Property tests for response correctness
- Integration tests with real HTTP clients

#### Stream Tests

- Unit tests for specific stream operations
- Property tests for data integrity through pipelines
- Backpressure tests with slow consumers

#### Crypto Tests

- Unit tests with known test vectors (NIST, RFC)
- Property tests for round-trip correctness
- Key generation validity tests

### Ecosystem Compatibility Tests

- Test262 conformance suite (target: 95%+ pass rate)
- Popular package tests: lodash, express, typescript, jest
- Real-world application tests

## Dependencies

### New Rust Dependencies

```toml
[dependencies]


# BigInt support


num-bigint = "0.4"
num-traits = "0.2"


# File watching


notify = "6.0"


# HTTP/TLS


hyper = { version = "1.0", features = ["server", "http1", "http2"] }
rustls = "0.21"
tokio-rustls = "0.24"


# Crypto


ring = "0.17"
rsa = "0.9"
p256 = "0.13"
p384 = "0.13"


# Testing


proptest = "1.4"
```

## Performance Considerations

### BigInt Performance

- Use `num-bigint` which is optimized for arbitrary-precision arithmetic
- Cache string representations for frequently converted values
- Avoid heap allocation for small BigInts that fit in 64 bits

### File Watching Performance

- Use native OS APIs via `notify` crate (inotify, FSEvents, ReadDirectoryChangesW)
- Debounce rapid file changes to avoid callback flooding
- Use efficient path matching for directory watches

### HTTP Server Performance

- Use `hyper` for high-performance HTTP handling
- Support HTTP/2 for multiplexed connections
- Implement connection pooling for Keep-Alive

### Stream Performance

- Use zero-copy where possible
- Implement proper backpressure to prevent memory exhaustion
- Use vectored I/O for efficient multi-buffer writes
