
# dx-reactor: Binary Dawn I/O Architecture

(LICENSE) "The fastest I/O reactor architecture for web frameworks" dx-reactor is the core I/O engine powering dx-www, implementing the Binary Dawn architecture for unprecedented web framework performance.

## Performance Targets

+--------+--------+------------+
| Metric | Target | Comparison |
+========+========+============+
| HTTP   | Mode   | 2          |
+--------+--------+------------+



## Key Innovations

### 1. Cross-Platform I/O Abstraction

+----------+---------+----------+
| Platform | Backend | Features |
+==========+=========+==========+
| Linux    | 5.1+    | io       |
+----------+---------+----------+
```rust
use dx_reactor::{DxReactor, WorkerStrategy, IoBackend};
let reactor = DxReactor::build()
.workers(WorkerStrategy::ThreadPerCore)
.io_backend(IoBackend::Auto) // Auto-detect best backend .build();
println!("Running on {} cores", reactor.num_cores());
```



### 2. Thread-per-Core Architecture

Zero lock contention through CPU-pinned worker threads: -One worker thread per CPU core -Local work queues (no shared locks) -Work-stealing only when idle -CPU pinning for cache locality ```rust // Thread-per-core (default)
let reactor = DxReactor::build()
.workers(WorkerStrategy::ThreadPerCore)
.build();
// Or fixed worker count let reactor = DxReactor::build()
.workers(WorkerStrategy::Fixed(8))
.build();
```


### 3. HBTP Protocol (Hyper-Binary Transfer Protocol)


Binary protocol replacing HTTP with 8-byte headers:
```rust
use dx_reactor::protocol::{HbtpOpcode, HbtpHeader, HbtpProtocol};
// 8-byte header: opcode(1) + flags(1) + sequence(2) + length(4)
let mut protocol = HbtpProtocol::new();
// O(1) route lookup via array index protocol.route(HbtpOpcode::RpcCall, |header, payload| { // Handle RPC call Ok(response_bytes)
});
```
HBTP Opcodes: -`Ping/Pong` - Connection keepalive -`StateSync/StateDelta` - State synchronization -`HtipClone/HtipPatchText` - UI operations -`RpcCall/RpcResponse` - Remote procedure calls -`ClientEvent` - Client-side events


### 4. Memory Teleportation


Zero-copy serialization between Rust server and WASM client:
```rust
use dx_reactor::memory::{TeleportBuffer, TeleportReader};
// Write data let mut buffer = TeleportBuffer::new(256);
buffer.write(&user_id);
buffer.write(&timestamp);
let (offset, len) = buffer.write_string("Hello, World!");
// Read back (zero-copy)
let bytes = buffer.finalize();
let reader = TeleportReader::with_string_table(bytes, string_table_offset);
let id = reader.read::<u64>().unwrap();
```


### 5. Compiler-Inlined Middleware (CIM)


Zero runtime overhead through compile-time inlining:
```rust
use dx_reactor::middleware::{AuthMiddleware, TimingMiddleware, RateLimitMiddleware};
use dx_reactor::dx_middleware;
// Generates a single inlined function dx_middleware!(TimingMiddleware, AuthMiddleware, RateLimitMiddleware);
// Use the generated function let result = process_middleware(&mut req, &mut res, |req| { // Your handler Ok(())
});
```
Built-in Middleware: -`AuthMiddleware` - JWT verification, claims injection -`TimingMiddleware` - X-Response-Time header -`RateLimitMiddleware` - Thread-local rate limiting


## Architecture


@tree[]


## Modules


+--------+----------------+
| Module | Description    |
+========+================+
| `io`   | Cross-platform |
+--------+----------------+


## Testing


The crate includes comprehensive property-based tests:
```bash

# Run all tests

cargo test --package dx-reactor

# Run property tests (with proptest)

cargo test --package dx-reactor --test property_tests

# Run integration tests

cargo test --package dx-reactor --test integration_tests ```
Test Coverage: -35 property-based tests (proptest) -11 integration tests -20 correctness properties validated

## Correctness Properties

All implementations are validated against formal correctness properties: -Batch Submission Count - submit() returns exact queued count -Kernel Version Detection - io_uring availability check -Kqueue Batch Submission - pending changes cleared after wait() -Completion Structure Integrity - user_data, result, flags preserved -Thread-per-Core Default - workers == num_cpus -Fixed Worker Count - workers == specified count -Opcode Uniqueness - all opcodes have unique u8 values -Header Size Invariant - HbtpHeader == 8 bytes -Header Parsing - from_bytes() behavior -O(1) Route Lookup - constant-time handler lookup -Flag Composition - independent, composable flags -ResponseBuffer Reuse - reset() enables reuse -Teleportation Round-Trip - write/read preserves values -Middleware Execution Order - before: forward, after: reverse -Timing Header Presence - X-Response-Time added -Rate Limit Thread Isolation - independent per-thread counters

## License

MIT OR Apache-2.0
