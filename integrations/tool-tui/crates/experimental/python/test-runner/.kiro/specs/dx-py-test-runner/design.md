
# Design Document: dx-py-test-runner

## Overview

dx-py-test-runner is a high-performance Python test runner built with Rust, designed to achieve 10x+ speedup over traditional test runners. The system uses a hybrid architecture where Rust handles performance-critical operations (discovery, scheduling, IPC) while Python executes the actual test code. The core philosophy follows "Binary Dawn" principles: -Zero-import discovery via Rust AST parsing -Pre-warmed interpreter pools to eliminate cold-start -Binary protocols for zero-copy IPC -Smart change detection to minimize test execution -Work-stealing for optimal parallelism

## Architecture

@tree[]

## Components and Interfaces

### 1. Discovery Engine

The Discovery Engine scans Python files for test functions using tree-sitter, avoiding Python imports entirely.
```rust
pub trait DiscoveryEngine { /// Scan a single file for test cases fn scan_file(&mut self, path: &Path) -> Result<Vec<TestCase>, DiscoveryError>;
/// Scan entire directory recursively fn scan_directory(&mut self, root: &Path) -> Result<TestIndex, DiscoveryError>;
/// Load from cached index if valid fn load_or_scan(&mut self, root: &Path) -> Result<TestIndex, DiscoveryError>;
}
pub struct TreeSitterDiscovery { parser: tree_sitter::Parser, language: tree_sitter::Language, }
```
Test Detection Rules: -Function names matching `test_*` or `*_test` -Class names matching `Test*` with methods matching `test_*` -Functions with `@pytest.mark.*` decorators -Functions with `@pytest.fixture` decorators (for fixture discovery)

### 2. Daemon Pool

The Daemon Pool manages pre-warmed Python interpreters for fast test execution.
```rust
pub trait DaemonPool { /// Start the daemon with N workers async fn start(config: DaemonConfig) -> Result<Self, DaemonError>;
/// Execute a test on an available worker async fn execute(&self, test: &TestCase) -> Result<TestResult, ExecutionError>;
/// Shutdown all workers gracefully async fn shutdown(&self) -> Result<(), DaemonError>;
}
pub struct DaemonConfig { pub worker_count: usize, pub preload_modules: Vec<String>, pub shared_memory_size: usize, }
```

### 3. Binary Protocol

The Binary Protocol defines message formats for Rust-Python communication.
```rust
/// Test execution request (32-byte header)


#[repr(C, packed)]


pub struct TestMessage { pub magic: u32, // 0xDEADBEEF pub msg_type: u8, // RUN=1, RESULT=2, SKIP=3, ERROR=4 pub flags: u8, // ASYNC=1, PARAMETERIZED=2, FIXTURE=4 pub test_id: u16, pub file_hash: u64, pub payload_len: u32, pub reserved: [u8; 8], }
/// Test result (40-byte header)


#[repr(C, packed)]


pub struct TestResultMessage { pub test_id: u16, pub status: u8, // PASS=0, FAIL=1, SKIP=2, ERROR=3 pub _padding: u8, pub duration_ns: u64, pub assertions_passed: u32, pub assertions_failed: u32, pub stdout_len: u32, pub stderr_len: u32, pub traceback_len: u32, }
pub trait BinaryProtocol { fn serialize_test(&self, test: &TestCase) -> Vec<u8>;
fn deserialize_result(&self, data: &[u8]) -> Result<TestResult, ProtocolError>;
}
```

### 4. Dependency Graph

The Dependency Graph tracks file imports for smart change detection.
```rust
pub trait DependencyGraph { /// Build import graph from project root fn build(root: &Path) -> Result<Self, GraphError>;
/// Get tests affected by changed files fn affected_tests(&self, changed: &[PathBuf]) -> Vec<TestId>;
/// Save graph to cache fn save(&self, path: &Path) -> Result<(), GraphError>;
/// Load graph from cache fn load(path: &Path) -> Result<Self, GraphError>;
}
```

### 5. Work-Stealing Executor

The Work-Stealing Executor distributes tests across workers with dynamic load balancing.
```rust
pub trait Executor { /// Execute all tests with work-stealing fn execute_all(&self, tests: Vec<TestCase>) -> Vec<TestResult>;
/// Get current worker utilization fn utilization(&self) -> f64;
}
pub struct WorkStealingExecutor { global_queue: Injector<TestCase>, workers: Vec<WorkerHandle>, stealers: Vec<Stealer<TestCase>>, }
```

### 6. Fixture Cache

The Fixture Cache stores serialized fixture state for instant restoration.
```rust
pub trait FixtureCache { /// Get cached fixture or create new fn get_or_create<T: Serialize + DeserializeOwned>( &mut self, id: FixtureId, create: impl FnOnce() -> T ) -> T;
/// Invalidate fixture cache fn invalidate(&mut self, id: FixtureId);
}
```

### 7. Snapshot Index

The Snapshot Index provides O(1) snapshot verification via hash comparison.
```rust
pub trait SnapshotIndex { /// Verify snapshot matches expected fn verify(&self, test_id: TestId, actual: &[u8]) -> SnapshotResult;
/// Update snapshot content fn update(&mut self, test_id: TestId, content: &[u8]);
}
pub enum SnapshotResult { Match, Mismatch { diff: String }, New { content: Vec<u8> }, }
```

## Data Models

### TestCase

```rust
pub struct TestCase { pub id: TestId, pub name: String, pub file_path: PathBuf, pub line_number: u32, pub class_name: Option<String>, pub markers: Vec<Marker>, pub fixtures: Vec<FixtureId>, }
pub struct TestId(pub u64);
pub struct Marker { pub name: String, pub args: Vec<String>, }
```

### TestResult

```rust
pub struct TestResult { pub test_id: TestId, pub status: TestStatus, pub duration: Duration, pub stdout: String, pub stderr: String, pub traceback: Option<String>, pub assertions: AssertionStats, }
pub enum TestStatus { Pass, Fail, Skip { reason: String }, Error { message: String }, }
pub struct AssertionStats { pub passed: u32, pub failed: u32, }
```

### TestIndex (Binary Format)

@tree[]

### Dependency Graph (Serialized)

```rust
pub struct SerializedGraph { pub version: u16, pub file_count: u32, pub edge_count: u32, pub files: Vec<FileNode>, pub edges: Vec<(u32, u32)>, // (importer, imported)
pub file_hashes: Vec<Blake3Hash>, }
pub struct FileNode { pub path_hash: u64, pub path: String, pub tests: Vec<TestId>, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Test Function Detection

For any Python source file containing function definitions, the Discovery_Engine SHALL identify exactly those functions whose names start with "test_" or end with "_test", belong to classes starting with "Test", or have @pytest.mark decorators as test functions. Validates: Requirements 1.2, 1.3, 1.4

### Property 2: Test Index Round-Trip

For any set of discovered test cases, writing them to a Test_Index file and reading them back SHALL produce an equivalent set of test cases with identical names, file paths, line numbers, and markers. Validates: Requirements 1.5, 1.6

### Property 3: Worker Pool Invariant

For any sequence of test execution requests to the Daemon_Pool, each test SHALL be assigned to exactly one worker, and after completion, the worker SHALL be returned to the available pool (total workers = busy + available at all times). Validates: Requirements 2.3, 2.4, 2.5

### Property 4: Binary Message Header Size

For any TestMessage serialized by the Binary_Protocol, the header portion SHALL be exactly 32 bytes. Validates: Requirements 3.1

### Property 5: Protocol Message Round-Trip

For any valid TestCase, serializing it to a TestMessage and deserializing it back SHALL produce an equivalent TestCase with identical test_id, file_hash, and payload. Validates: Requirements 3.4, 3.5

### Property 6: Protocol Error Handling

For any byte sequence that does not conform to the TestMessage or TestResultMessage format, the Binary_Protocol SHALL return a ProtocolError rather than panicking or returning invalid data. Validates: Requirements 3.6

### Property 7: Import Graph Construction

For any Python project directory, the Dependency_Graph SHALL contain a node for each.py file and an edge (A → B) for each import statement in file A that references file B. Validates: Requirements 4.1

### Property 8: Transitive Dependency Detection

For any file F in the Dependency_Graph, the affected_tests(F) result SHALL include all tests in files that transitively import F (directly or through intermediate files). Validates: Requirements 4.2

### Property 9: Watch Mode Filtering

For any set of changed files, the Test_Runner in watch mode SHALL execute exactly the tests returned by affected_tests() and no others. Validates: Requirements 4.3

### Property 10: Dependency Graph Round-Trip

For any Dependency_Graph, saving it to disk and loading it back SHALL produce an equivalent graph with identical nodes, edges, and file hashes. Validates: Requirements 4.4, 4.5

### Property 11: Test Distribution Completeness

For any set of tests submitted to the Work_Stealer, every test SHALL be executed exactly once (no duplicates, no omissions). Validates: Requirements 5.1

### Property 12: Result Aggregation Completeness

For any set of N tests submitted to the Work_Stealer, the aggregated results SHALL contain exactly N TestResult entries, one for each submitted test. Validates: Requirements 5.3

### Property 13: Executor Fault Tolerance

For any set of tests where one worker encounters a fatal error, all tests assigned to other workers SHALL still complete and return results. Validates: Requirements 5.5

### Property 14: Fixture Cache Round-Trip

For any serializable fixture value, storing it in the Fixture_Cache and retrieving it SHALL produce an equivalent value. When the fixture function changes (different hash), the cache SHALL be invalidated and the fixture recreated. Validates: Requirements 6.1, 6.3

### Property 15: Snapshot Hash Correctness

For any snapshot content, the stored Blake3 hash SHALL equal blake3::hash(content). Validates: Requirements 7.1

### Property 16: Snapshot Diff Generation

For any two different byte sequences (actual vs expected), when hashes differ, the Snapshot_Index SHALL produce a diff that accurately represents the differences between them. Validates: Requirements 7.4

### Property 17: Snapshot Update Consistency

For any snapshot update operation, after updating with new content, verifying with that same content SHALL return SnapshotResult::Match. Validates: Requirements 7.5

### Property 18: Test Pattern Filtering

For any test pattern and set of test cases, the filtered results SHALL contain exactly those tests whose names match the pattern (glob or regex as specified). Validates: Requirements 8.3

### Property 19: JUnit XML Validity

For any set of test results in CI mode, the generated JUnit XML output SHALL be valid XML conforming to the JUnit XML schema. Validates: Requirements 10.4

## Error Handling

### Discovery Errors

+------------------------------+---------+----------+
| Error                        | Cause   | Handling |
+==============================+=========+==========+
| `DiscoveryError::ParseError` | Invalid | Python   |
+------------------------------+---------+----------+



### Daemon Errors

+----------------------------+--------+-------------+
| Error                      | Cause  | Handling    |
+============================+========+=============+
| `DaemonError::WorkerCrash` | Python | interpreter |
+----------------------------+--------+-------------+



### Protocol Errors

+-------------------------------+-------+----------+
| Error                         | Cause | Handling |
+===============================+=======+==========+
| `ProtocolError::InvalidMagic` | Wrong | magic    |
+-------------------------------+-------+----------+



### Executor Errors

+-------------------------------+--------+----------+
| Error                         | Cause  | Handling |
+===============================+========+==========+
| `ExecutionError::WorkerPanic` | Worker | thread   |
+-------------------------------+--------+----------+



### Fixture Errors

+-------------------------------------+--------+-----------+
| Error                               | Cause  | Handling  |
+=====================================+========+===========+
| `FixtureError::SerializationFailed` | Cannot | serialize |
+-------------------------------------+--------+-----------+



## Testing Strategy

### Dual Testing Approach

This project uses both unit tests and property-based tests: -Unit tests: Verify specific examples, edge cases, and error conditions -Property tests: Verify universal properties across randomly generated inputs

### Property-Based Testing Framework

- Rust: Use `proptest` crate for property-based testing
- Python: Use `hypothesis` for Python-side property tests
- Configuration: Minimum 100 iterations per property test

### Test Organization

@tree:tests[]

### Property Test Annotations

Each property test must be annotated with:
```rust
/// Feature: dx-py-test-runner, Property 1: Test Function Detection /// Validates: Requirements 1.2, 1.3, 1.4


#[test]


fn prop_test_function_detection() { // ...
}
```

### Test Generators

Key generators needed for property tests: -Python Source Generator: Generate valid Python source with various function/class patterns -TestCase Generator: Generate random TestCase instances -Import Graph Generator: Generate random but valid import graphs -Byte Sequence Generator: Generate both valid and invalid protocol messages

### Coverage Goals

- Unit test coverage: >80% line coverage
- Property tests: All 19 properties implemented
- Integration tests: Core workflows covered
