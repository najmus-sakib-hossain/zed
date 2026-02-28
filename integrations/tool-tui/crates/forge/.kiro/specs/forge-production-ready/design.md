
# Design Document: DX Forge Production-Ready Transformation

## Overview

This design document outlines the architectural changes and implementation approach required to transform DX Forge from its current prototype state into a production-ready codebase. The transformation focuses on six key areas: -Eliminating Global State - Replacing `OnceLock`/`static` patterns with dependency injection -Completing Stub Implementations - Making all 132 API functions work -Hardening Error Handling - Removing panics, adding context to all errors -Cleaning Up Dead Code - Removing unused code, empty directories, duplicates -Improving Test Coverage - Adding meaningful property tests and integration tests -Honest Documentation - Accurate status reporting and proper versioning

## Architecture

### Current Architecture (Problems)

@tree[] Problems: -Cannot create multiple isolated instances -Tests interfere with each other -Hidden dependencies make code hard to reason about -Cannot mock dependencies for testing

### Target Architecture (Solution)

@tree[] Benefits: -Multiple isolated instances possible -Tests can run in parallel safely -All dependencies explicit and injectable -Easy to mock for testing

## Components and Interfaces

### Core Components

#### 1. Forge (Main Entry Point)

```rust
/// Main Forge instance - unified API for DX tools /// All state is owned by this struct, no global state pub struct Forge { /// Configuration config: ForgeConfig, /// Tool orchestration orchestrator: Orchestrator, /// Registered tools tool_registry: ToolRegistry, /// Branching decision engine branching_engine: BranchingEngine, /// Event bus for pub/sub event_bus: EventBus, /// Storage backend (pluggable)
storage: Box<dyn StorageBackend>, /// File watcher (optional)
watcher: Option<DualWatcher>, /// Current execution context context: ExecutionContext, /// Platform I/O backend platform_io: Box<dyn IoBackend>, }
impl Forge { /// Create a new Forge instance pub fn new(project_root: impl AsRef<Path>) -> Result<Self>;
/// Create with custom configuration pub fn with_config(config: ForgeConfig) -> Result<Self>;
/// Create with injected dependencies (for testing)
pub fn with_dependencies( config: ForgeConfig, storage: Box<dyn StorageBackend>, platform_io: Box<dyn IoBackend>, ) -> Result<Self>;
}
```

#### 2. BranchingEngine (Extracted from Global State)

```rust
/// Branching decision engine - manages file change safety pub struct BranchingEngine { voters: Vec<String>, pending_changes: Vec<FileChange>, votes: HashMap<PathBuf, Vec<BranchingVote>>, last_application: Option<ApplicationRecord>, }
/// Record of applied changes for revert support pub struct ApplicationRecord { files: Vec<PathBuf>, backups: HashMap<PathBuf, Vec<u8>>, timestamp: DateTime<Utc>, }
impl BranchingEngine { pub fn new() -> Self;
/// Apply changes with full branching resolution pub fn apply_changes(&mut self, changes: Vec<FileChange>) -> Result<Vec<PathBuf>>;
/// Revert the most recent application pub fn revert_most_recent(&mut self) -> Result<Vec<PathBuf>>;
/// Submit a vote for a file change pub fn submit_vote(&mut self, file: &Path, vote: BranchingVote) -> Result<()>;
/// Query predicted branch color pub fn predict_color(&self, file: &Path) -> BranchColor;
}
```

#### 3. EventBus (Extracted from Global State)

```rust
/// Event bus for publish/subscribe pattern pub struct EventBus { subscribers: HashMap<String, Vec<Sender<ForgeEvent>>>, }
impl EventBus { pub fn new() -> Self;
/// Publish an event to all subscribers pub fn publish(&self, event: ForgeEvent) -> Result<()>;
/// Subscribe to events of a specific type pub fn subscribe(&mut self, event_type: &str) -> Receiver<ForgeEvent>;
}
```

#### 4. StorageBackend (Trait for Pluggable Storage)

```rust
/// Pluggable storage backend pub trait StorageBackend: Send + Sync { /// Store a blob fn store_blob(&mut self, content: &[u8]) -> Result<BlobId>;
/// Retrieve a blob fn get_blob(&self, id: &BlobId) -> Result<Vec<u8>>;
/// Sync to remote (R2)
fn sync_to_remote(&mut self) -> Result<SyncResult>;
/// Pull from remote fn pull_from_remote(&mut self) -> Result<SyncResult>;
}
/// Default SQLite-based storage pub struct SqliteStorage { ... }
/// Mock storage for testing pub struct MockStorage { ... }
```

### API Migration Strategy

The 132 API functions will be migrated in phases: Phase 1: Core Lifecycle (4 functions) -`initialize_forge()` → `Forge::new()` (constructor) -`register_tool(tool)` → `forge.register_tool(tool)` -`get_tool_context()` → `forge.context()` -`shutdown_forge()` → `drop(forge)` (RAII) Phase 2: Branching (15 functions) -All branching functions become methods on `Forge` that delegate to `BranchingEngine` Phase 3: Events (9 functions) -All event functions become methods on `Forge` that delegate to `EventBus` Phase 4: Remaining Functions -Continue pattern for all 132 functions

### Backward Compatibility

For backward compatibility during migration, provide deprecated wrapper functions:
```rust


#[deprecated(since = "0.2.0", note = "Use Forge::new() instead")]


pub fn initialize_forge() -> Result<()> { // Create a global instance for legacy code // Log deprecation warning tracing::warn!("initialize_forge() is deprecated, use Forge::new() instead");
// ... legacy behavior }
```

## Data Models

### FileChange (Enhanced)

```rust
/// File change with backup support for revert


#[derive(Debug, Clone)]


pub struct FileChange { pub path: PathBuf, pub old_content: Option<Vec<u8>>, // Changed from String to Vec<u8> pub new_content: Vec<u8>, // Changed from String to Vec<u8> pub tool_id: String, pub timestamp: DateTime<Utc>, }
```

### ApplicationRecord (New)

```rust
/// Record of applied changes for revert support


#[derive(Debug, Clone)]


pub struct ApplicationRecord { pub id: Uuid, pub files: Vec<PathBuf>, pub backups: HashMap<PathBuf, Vec<u8>>, pub timestamp: DateTime<Utc>, pub tool_id: String, }
```

### ForgeConfig (Enhanced)

```rust
/// Configuration for Forge instance


#[derive(Clone, Debug)]


pub struct ForgeConfig { pub project_root: PathBuf, pub forge_dir: PathBuf, pub auto_watch: bool, pub enable_lsp: bool, pub enable_versioning: bool, pub worker_threads: usize, // New fields pub debounce_delay: Duration, pub idle_threshold: Duration, pub max_backup_size: usize, pub enable_r2_sync: bool, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Instance Isolation

For any two Forge instances created with different project roots, operations performed on one instance SHALL NOT affect the state of the other instance. Validates: Requirements 1.2, 1.3, 1.5 This property ensures that the elimination of global state is complete. We can test this by: -Creating two Forge instances -Registering a tool in instance A -Verifying the tool is NOT visible in instance B -Applying changes in instance A -Verifying instance B's branching state is unchanged

### Property 2: Debounce Timing Correctness

For any sequence of N events triggered within a debounce window of duration D, the debounced event handler SHALL be called exactly once, after the last event plus the debounce delay. Validates: Requirements 2.6 This property ensures debouncing works correctly: -Generate a random sequence of events with timestamps -Trigger all events -Verify exactly one debounced callback fires -Verify the callback fires after (last_event_time + debounce_delay)

### Property 3: Idle Detection Correctness

For any idle period exceeding the configured idle threshold, the idle event handler SHALL be triggered exactly once per idle period. Validates: Requirements 2.7, 2.8 This property ensures idle detection works: -Configure an idle threshold -Simulate activity followed by inactivity -Verify idle event fires after threshold -Verify no duplicate idle events during continued inactivity

### Property 4: Revert Round-Trip

For any set of file changes applied via `apply_changes()`, calling `revert_most_recent_application()` SHALL restore all affected files to their exact previous state. Validates: Requirements 2.9 This is a classic round-trip property: -Record original file contents -Apply changes -Revert changes -Verify files match original contents exactly

### Property 5: Graceful Error Handling

For any invalid input to a public API function, the function SHALL return a `Result::Err` with a descriptive message containing context about what operation failed, rather than panicking. Validates: Requirements 3.3, 3.6, 9.2 This property ensures no panics: -Generate random invalid inputs (empty paths, malformed data, etc.) -Call API functions with invalid inputs -Verify all return `Err` (not panic) -Verify error messages contain operation context

### Property 6: Backend Fallback

For any platform where the native I/O backend fails to initialize, the system SHALL fall back to the portable backend and continue operating correctly. Validates: Requirements 11.4 This property ensures graceful degradation: -Simulate native backend initialization failure -Verify system falls back to portable backend -Verify all I/O operations still work correctly

## Error Handling

### Error Strategy

All errors will use `anyhow::Result` with `.context()` for operation context:
```rust
pub fn apply_changes(&mut self, changes: Vec<FileChange>) -> Result<Vec<PathBuf>> { for change in &changes { self.validate_change(change)
.with_context(|| format!("validating change to {:?}", change.path))?;
}
self.branching_engine .apply_changes(changes)
.context("applying changes through branching engine")
}
```

### Error Categories

Use the existing `ErrorCategory` enum consistently: -`Network` - R2 sync failures, registry queries -`FileSystem` - File I/O, permissions -`Configuration` - Invalid config, missing required fields -`Validation` - Invalid input data -`Dependency` - Tool dependency resolution failures -`Timeout` - Operation timeouts -`Unknown` - Unexpected errors

### Panic Elimination

Replace all panic-prone patterns:
```rust
// BEFORE (panic-prone)
let value = map.get(&key).unwrap();
let item = vec[index];
// AFTER (graceful)
let value = map.get(&key)
.ok_or_else(|| anyhow!("key {:?} not found in map", key))?;
let item = vec.get(index)
.ok_or_else(|| anyhow!("index {} out of bounds (len={})", index, vec.len()))?;
```

## Testing Strategy

### Dual Testing Approach

This project uses both unit tests and property-based tests: -Unit tests: Verify specific examples, edge cases, and error conditions -Property tests: Verify universal properties across randomly generated inputs Both are complementary and necessary for comprehensive coverage.

### Property-Based Testing Configuration

- Library: `proptest` (already in dev-dependencies)
- Minimum iterations: 100 per property test
- Tag format: `Feature: forge-production-ready, Property N: <property_text>`

### Test Organization

@tree:tests[]

### Property Test Examples

```rust
// Property 1: Instance Isolation proptest! {


#![proptest_config(ProptestConfig::with_cases(100))]


/// Feature: forge-production-ready, Property 1: Instance Isolation /// For any two Forge instances, operations on one shall not affect the other


#[test]


fn prop_instance_isolation( tool_name in "[a-z]{3,10}", tool_version in "[0-9]\\.[0-9]\\.[0-9]", ) { let temp_a = tempfile::tempdir()?;
let temp_b = tempfile::tempdir()?;
let forge_a = Forge::new(temp_a.path())?;
let forge_b = Forge::new(temp_b.path())?;
// Register tool in A let tool = TestTool::new(&tool_name, &tool_version);
forge_a.register_tool(Box::new(tool))?;
// Verify tool NOT in B prop_assert!(!forge_b.is_tool_registered(&tool_name));
}
}
// Property 4: Revert Round-Trip proptest! {


#![proptest_config(ProptestConfig::with_cases(100))]


/// Feature: forge-production-ready, Property 4: Revert Round-Trip /// For any applied changes, reverting shall restore original state


#[test]


fn prop_revert_roundtrip( original_content in "[a-zA-Z0-9\\s]{10,1000}", new_content in "[a-zA-Z0-9\\s]{10,1000}", ) { let temp = tempfile::tempdir()?;
let file_path = temp.path().join("test.txt");
// Write original content std::fs::write(&file_path, &original_content)?;
let mut forge = Forge::new(temp.path())?;
// Apply change let change = FileChange { path: file_path.clone(), old_content: Some(original_content.as_bytes().to_vec()), new_content: new_content.as_bytes().to_vec(), tool_id: "test".to_string(), timestamp: Utc::now(), };
forge.apply_changes(vec![change])?;
// Verify new content prop_assert_eq!(std::fs::read_to_string(&file_path)?, new_content);
// Revert forge.revert_most_recent_application()?;
// Verify original content restored prop_assert_eq!(std::fs::read_to_string(&file_path)?, original_content);
}
}
```

### Coverage Targets

+-----------------+--------+------------+
| Module          | Target | Coverage   |
+=================+========+============+
| `core/forge.rs` | 80%    | `branching |
+-----------------+--------+------------+
