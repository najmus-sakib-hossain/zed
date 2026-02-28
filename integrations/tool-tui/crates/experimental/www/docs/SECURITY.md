
# Security Documentation

This document describes the security model, threat assumptions, and unsafe code usage in dx-www.

## Threat Model

### Trust Boundaries

- Compiler (Trusted): The dx-www compiler runs on the developer's machine and is trusted.
- Server (Trusted): The server runtime is trusted to serve correct HTIP streams.
- Client (Untrusted): The browser environment is untrusted; all client-side code must be defensive.
- User Input (Untrusted): All user input is untrusted and must be validated/sanitized.

### Security Assumptions

- The Rust compiler and standard library are correct
- Dependencies from crates.io are audited via `cargo audit`
- The WASM sandbox provides memory isolation
- TLS is used for all network communication in production

### Out of Scope

- Physical access attacks
- Side-channel attacks (timing, cache)
- Denial of service (resource exhaustion)

## Unsafe Code Inventory

All unsafe code in dx-www is documented here with safety invariants.

### reactor/src/memory/teleport.rs

Purpose: Zero-copy serialization for efficient data transfer between server and WASM client.
```rust
// SAFETY: Teleportable trait requires:
// 1. #[repr(C)] for stable memory layout // 2. No pointers or references (Copy types only)
// 3. Identical layout on server and WASM client pub unsafe trait Teleportable: Copy + 'static { ... }
```
Unsafe Operations: -`std::slice::from_raw_parts` - Creating slices from raw pointers -`std::ptr::read` - Reading values from raw pointers Safety Invariants: -All types implementing `Teleportable` must be `#[repr(C)]` -Alignment is checked before pointer casts -Size bounds are validated before access

### reactor/src/protocol/hbtp.rs

Purpose: Zero-copy parsing of HBTP protocol headers.
```rust
// SAFETY: HbtpHeader is #[repr(C, packed)] with no padding.
// We verify the slice is large enough before casting.
Some(unsafe { &*(bytes.as_ptr() as *const Self) })
```
Safety Invariants: -Header struct is `#[repr(C, packed)]` -Slice length is checked before cast -No uninitialized memory is read

### morph/src/lib.rs

Purpose: Atomic dirty bit tracking for incremental DOM updates.
```rust
// SAFETY: AtomicDirtyMask uses UnsafeCell for interior mutability // with atomic operations for thread safety.
let atomic = unsafe { &*(self.inner.get() as *const AtomicU64) };
```
Safety Invariants: -All operations use atomic memory ordering -`UnsafeCell` provides interior mutability -Single-threaded WASM context (no data races)

### framework-core/src/lib.rs

Purpose: Memory management for WASM linear memory.
```rust
// SAFETY: MemoryManager operates on WASM linear memory.
// - base_ptr points to valid memory of MEMORY_SIZE bytes // - Memory regions are non-overlapping // - Exclusive access via &mut self unsafe impl Send for MemoryManager {}
unsafe impl Sync for MemoryManager {}
```
Safety Invariants: -WASM is single-threaded (Send/Sync are safe) -Memory regions are statically defined and non-overlapping -Bounds checking before all memory access -Lifetime tied to `&self` or `&mut self`

### client/src/allocator.rs

Purpose: Custom allocator for WASM client to minimize binary size. Safety Invariants: -Allocator operates within WASM linear memory bounds -Alignment requirements are respected -No double-free or use-after-free

### packet/src/lib.rs

Purpose: Binary packet encoding/decoding. Safety Invariants: -All pointer casts are to `#[repr(C)]` types -Size and alignment are validated -No uninitialized memory access

## Security Mitigations

### Input Validation

- Parser Security: The parser rejects dangerous patterns:
- `eval()` and `Function()` calls
- `innerHTML` and `outerHTML` assignments
- `document.write()` calls
- `javascript:` and `data:` URLs
- `dangerouslySetInnerHTML` props
- HTIP Validation: Binary streams are validated:
- Magic bytes verification
- Version compatibility check
- Bounds checking on all offsets
- Opcode validation

### Memory Safety

- No Raw Pointers in Public API: All public APIs use safe Rust types
- Bounds Checking: All array/slice access is bounds-checked
- No Panics in Library Code: All errors are returned as `Result`

### Cryptographic Security

- Ed25519 Tokens: Authentication uses Ed25519 signatures
- BLAKE3 Hashing: Content hashing uses BLAKE3
- Argon2 Passwords: Password hashing uses Argon2id

## Crates with `#![forbid(unsafe_code)]`

The following crates contain no unsafe code and have `#![forbid(unsafe_code)]` enabled: -`dx-www-a11y` - Accessibility analysis -`dx-www-auth` - Authentication (uses safe crypto wrappers) -`dx-www-cache` - Caching layer -`dx-www-db` - Database abstractions -`dx-www-db-teleport` - Database teleport utilities -`dx-www-debug` - Debugging utilities -`dx-www-dom` - DOM abstractions -`dx-www-error` - Error types -`dx-www-fallback` - Fallback rendering -`dx-www-form` - Form handling -`dx-www-guard` - Route guards -`dx-www-interaction` - User interaction handling -`dx-www-offline` - Offline support -`dx-www-print` - Print stylesheets -`dx-www-query` - Query handling -`dx-www-rtl` - RTL support -`dx-www-sched` - Scheduling -`dx-www-state` - State management -`dx-www-sync` - Synchronization

## Crates Requiring Unsafe

The following crates require unsafe for performance or FFI: -`dx-www-compiler` - OXC FFI bindings -`dx-www-binary` - Zero-copy binary parsing -`dx-www-client` - WASM memory management -`dx-www-reactor` - Platform I/O (epoll, kqueue, IOCP) -`dx-www-morph` - Atomic dirty bit tracking -`dx-www-packet` - Binary packet encoding -`dx-www-framework-core` - WASM memory regions

## Vulnerability Reporting

Please report security vulnerabilities to security@dx-www.dev. Do NOT open public issues for security vulnerabilities. We will acknowledge receipt within 48 hours and provide a detailed response within 7 days.

## Security Audit Status

- Initial security review (completed January 2026)
- External audit (scheduled for Q2 2026)
- `cargo audit` in CI
- `cargo audit` passes with no vulnerabilities (last verified: January 2026)
- Clippy with security lints
- Unsafe code documentation (SAFETY comments on all unsafe blocks)
- `#![forbid(unsafe_code)]` on all safe crates (19 crates)
- Property-based tests for production operations
- Observability infrastructure (tracing, metrics, logging)
- Graceful shutdown with configurable timeout
- Health probes (liveness and readiness)
- Circuit breaker pattern for resilience
