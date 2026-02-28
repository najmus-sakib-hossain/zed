
# Requirements Document

## Introduction

This specification defines the requirements for implementing a cross-platform I/O reactor (`dx-reactor`) for the dx-www framework. The reactor will provide platform-optimized async I/O using io_uring on Linux, kqueue on macOS/BSD, and IOCP on Windows. Additionally, this spec covers battle-testing dx-www to ensure it runs correctly across all supported platforms. The dx-reactor is a foundational component that enables dx-www to achieve its performance targets of 2.5M+ RPS (HTTP) and 5M+ RPS (HBTP binary protocol), with sub-100μs p99 latency.

## Glossary

- Reactor: The core event loop component that manages async I/O operations
- io_uring: Linux kernel async I/O interface (Linux 5.1+) providing zero-syscall operations
- kqueue: BSD/macOS kernel event notification interface
- IOCP: Windows I/O Completion Ports for async I/O
- epoll: Linux event polling interface (fallback for older Linux)
- HBTP: Hyper-Binary Transfer Protocol
- dx-www's binary communication protocol
- HTIP: Holographic Template Instruction Protocol
- binary format for UI operations
- Teleportation: Zero-copy serialization where wire format equals memory layout
- Thread_Per_Core: Architecture where each CPU core has its own dedicated thread and event loop
- CIM: Compiler-Inlined Middleware
- middleware inlined at compile time for zero overhead

## Requirements

### Requirement 1: Cross-Platform I/O Abstraction

User Story: As a developer, I want dx-www to automatically use the best I/O backend for my platform, so that I get optimal performance without manual configuration.

#### Acceptance Criteria

- THE Reactor SHALL provide a unified trait interface for all platform-specific backends
- WHEN running on Linux 5.1+, THE Reactor SHALL use io_uring as the default backend
- WHEN running on Linux older than 5.1, THE Reactor SHALL fall back to epoll
- WHEN running on macOS or BSD, THE Reactor SHALL use kqueue
- WHEN running on Windows, THE Reactor SHALL use IOCP
- THE Reactor SHALL select the appropriate backend at compile time using conditional compilation
- THE Reactor SHALL provide a runtime detection function to verify backend availability

### Requirement 2: io_uring Backend Implementation (Linux)

User Story: As a Linux user, I want dx-www to leverage io_uring for maximum I/O performance, so that I can achieve the highest possible throughput.

#### Acceptance Criteria

- THE UringReactor SHALL support kernel-side polling (SQPOLL) for zero-syscall operations
- THE UringReactor SHALL support registered buffers for zero-copy I/O
- THE UringReactor SHALL support multishot receive operations
- THE UringReactor SHALL support zero-copy send operations
- WHEN io_uring is not available, THE UringReactor SHALL return an appropriate error
- THE UringReactor SHALL implement the Reactor trait with all required methods
- THE UringReactor SHALL support configurable submission queue size

### Requirement 3: kqueue Backend Implementation (macOS/BSD)

User Story: As a macOS or BSD user, I want dx-www to use kqueue for efficient event-driven I/O, so that I get native performance on my platform.

#### Acceptance Criteria

- THE KqueueReactor SHALL support read and write event registration
- THE KqueueReactor SHALL support batch event submission and retrieval
- THE KqueueReactor SHALL support configurable timeout for event waiting
- THE KqueueReactor SHALL implement the Reactor trait with all required methods
- THE KqueueReactor SHALL properly handle edge-triggered events

### Requirement 4: IOCP Backend Implementation (Windows)

User Story: As a Windows user, I want dx-www to use IOCP for efficient async I/O, so that I get native performance on Windows.

#### Acceptance Criteria

- THE IocpReactor SHALL support completion port creation and association
- THE IocpReactor SHALL support async file read operations with OVERLAPPED
- THE IocpReactor SHALL support async socket recv operations with WSARecv
- THE IocpReactor SHALL support batch completion retrieval
- THE IocpReactor SHALL implement the Reactor trait with all required methods
- THE IocpReactor SHALL properly handle pending I/O operations

### Requirement 5: Thread-Per-Core Architecture

User Story: As a system administrator, I want dx-www to utilize all CPU cores efficiently, so that I can maximize server throughput.

#### Acceptance Criteria

- THE DxReactor SHALL spawn one worker thread per CPU core by default
- THE DxReactor SHALL support configurable worker count
- THE DxReactor SHALL pin each worker thread to its corresponding CPU core
- THE DxReactor SHALL provide per-core local queues to minimize lock contention
- WHEN a core is underloaded, THE DxReactor SHALL support work-stealing from other cores

### Requirement 6: HBTP Protocol Implementation

User Story: As a developer, I want dx-www to use a binary protocol for client-server communication, so that I can achieve minimal latency and bandwidth usage.

#### Acceptance Criteria

- THE HbtpProtocol SHALL define opcodes for all common operations (ping, state, RPC, events)
- THE HbtpHeader SHALL be exactly 8 bytes for minimal overhead
- THE HbtpProtocol SHALL support message compression with zstd
- THE HbtpProtocol SHALL support message encryption with ChaCha20
- THE HbtpProtocol SHALL support O(1) route lookup using array indexing
- THE HbtpProtocol SHALL serialize and deserialize messages correctly (round-trip)

### Requirement 7: Memory Teleportation (Zero-Copy Serialization)

User Story: As a developer, I want to transfer data between server and client without serialization overhead, so that I can achieve maximum throughput.

#### Acceptance Criteria

- THE TeleportBuffer SHALL write values with correct alignment
- THE TeleportBuffer SHALL support string table for variable-length data
- THE TeleportReader SHALL read values without copying (zero-copy)
- FOR ALL Teleportable types, serializing then deserializing SHALL produce an equivalent value (round-trip)
- THE Teleportable trait SHALL verify memory layout at compile time

### Requirement 8: Compiler-Inlined Middleware (CIM)

User Story: As a developer, I want middleware to have zero runtime overhead, so that I can add cross-cutting concerns without performance penalty.

#### Acceptance Criteria

- THE Middleware trait SHALL support before and after hooks
- THE dx_middleware macro SHALL inline all middleware at compile time
- THE CIM system SHALL execute after hooks in reverse order
- THE CIM system SHALL support short-circuiting in before hooks
- THE CIM system SHALL have zero virtual dispatch overhead

### Requirement 9: Database Teleport Cache

User Story: As a developer, I want frequently-read database queries to be pre-cached in binary format, so that I can achieve sub-millisecond response times.

#### Acceptance Criteria

- THE DbTeleport SHALL support query registration with table dependencies
- THE DbTeleport SHALL cache query results in pre-serialized binary format
- WHEN a table is modified, THE DbTeleport SHALL invalidate related cache entries via NOTIFY
- THE DbTeleport SHALL support cache lookup in O(1) time
- THE DbTeleport SHALL support configurable cache size limits

### Requirement 10: Cross-Platform Testing and Validation

User Story: As a developer, I want dx-www to be thoroughly tested on all platforms, so that I can deploy with confidence.

#### Acceptance Criteria

- THE test suite SHALL include unit tests for all reactor backends
- THE test suite SHALL include property-based tests for protocol correctness
- THE test suite SHALL include integration tests for cross-platform behavior
- THE test suite SHALL verify binary format compatibility across platforms
- THE test suite SHALL achieve minimum 80% code coverage for reactor code
- WHEN tests run on any supported platform, THE test suite SHALL pass completely
