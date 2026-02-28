
# Design Document: DX-Py-Runtime Compilation Fix

## Overview

This design document describes the technical approach to fix all 24 compilation errors in the dx-py-runtime project. The errors are concentrated in the `dx-py-interpreter` crate, specifically in two modules: -jit_integration.rs - API mismatches with dx-py-jit crate -async_integration.rs - Missing dependency on dx-py-reactor crate The fix strategy involves: -Adding the missing dx-py-reactor dependency -Updating API calls to match the actual dx-py-jit interface -Implementing local tier tracking since FunctionProfile doesn't store tier state -Adapting async integration to use the correct reactor API

## Architecture

The dx-py-interpreter sits at the center of the runtime, coordinating between: @tree[]

## Components and Interfaces

### Component 1: Cargo.toml Dependency Fix

File: `dx-py-interpreter/Cargo.toml` Add the missing dx-py-reactor dependency:
```toml
[dependencies]


# ... existing dependencies ...


dx-py-reactor = { path = "../dx-py-reactor" }
```

### Component 2: JIT Integration Rewrite

File: `dx-py-interpreter/src/jit_integration.rs`

#### 2.1 Type Mappings

The interpreter uses string function names, but dx-py-jit uses `FunctionId(u64)`. We need a consistent mapping:
```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
/// Maps function names to FunctionIds struct FunctionIdMapper { name_to_id: RwLock<HashMap<String, FunctionId>>, next_id: AtomicU64, }
impl FunctionIdMapper { fn get_or_create(&self, name: &str) -> FunctionId { // Check existing if let Some(id) = self.name_to_id.read().get(name) { return *id;
}
// Create new let id = FunctionId(self.next_id.fetch_add(1, Ordering::Relaxed));
self.name_to_id.write().insert(name.to_string(), id);
id }
}
```

#### 2.2 Local Tier Tracking

Since `FunctionProfile` doesn't have `current_tier()` or `set_tier()` methods, we track tiers locally:
```rust
pub struct JitIntegration { jit: Arc<TieredJit>, /// Local tier tracking (FunctionProfile doesn't store this)
tiers: RwLock<HashMap<FunctionId, CompilationTier>>, /// Function name to ID mapping func_ids: FunctionIdMapper, osr: Arc<OsrManager>, enabled: bool, tier_thresholds: TierThresholds, }
struct TierThresholds { baseline: u64, // 100 calls optimizing: u64, // 1000 calls aot: u64, // 10000 calls }
```

#### 2.3 Corrected API Calls

+--------------------------------+---------------------------------+
| Old                            | (Broken                         |
+================================+=================================+
| `FunctionProfile::new\(name\)` | `FunctionProfile::new\(bytecode |
+--------------------------------+---------------------------------+



#### 2.4 Updated record_call Implementation

```rust
pub fn record_call(&self, func_name: &str, bytecode: &[u8]) -> Option<CompilationTier> { if !self.enabled { return None;
}
let func_id = self.func_ids.get_or_create(func_name);
let profile = self.jit.get_profile(func_id, bytecode.len(), 0);
profile.record_call();
let call_count = profile.get_call_count();
// Get current tier from local tracking let current_tier = self.tiers.read()
.get(&func_id)
.copied()
.unwrap_or(CompilationTier::Interpreter);
// Determine if promotion is needed let new_tier = if call_count >= self.tier_thresholds.aot { CompilationTier::AotOptimized } else if call_count >= self.tier_thresholds.optimizing { CompilationTier::OptimizingJit } else if call_count >= self.tier_thresholds.baseline { CompilationTier::BaselineJit } else { CompilationTier::Interpreter };
if new_tier > current_tier { self.tiers.write().insert(func_id, new_tier);
Some(new_tier)
} else { None }
}
```

#### 2.5 Updated compile Implementation

```rust
pub fn compile(&self, func_name: &str, tier: CompilationTier, bytecode: &[u8]) -> Result<(), JitError> { if !self.enabled { return Err(JitError::Disabled);
}
let func_id = self.func_ids.get_or_create(func_name);
// compile returns Option<*const u8>, not Result match self.jit.compile(func_id, tier, bytecode) { Some(_ptr) => Ok(()), None => Err(JitError::CompilationFailed("Compilation returned None".to_string())), }
}
```

#### 2.6 Updated has_compiled Implementation

```rust
pub fn has_compiled(&self, func_name: &str) -> bool { let func_id = self.func_ids.get_or_create(func_name);
self.jit.get_compiled(func_id).is_some()
}
```

#### 2.7 Updated deoptimize Implementation

```rust
pub fn deoptimize(&self, func_name: &str) -> Result<(), JitError> { let func_id = self.func_ids.get_or_create(func_name);
self.jit.invalidate(func_id);
// Reset tier to interpreter self.tiers.write().insert(func_id, CompilationTier::Interpreter);
Ok(())
}
```

#### 2.8 Updated OSR Methods

```rust
pub fn can_osr(&self, func_name: &str, bytecode_offset: usize) -> bool { let func_id = self.func_ids.get_or_create(func_name);
self.osr.get_entry(func_id, bytecode_offset).is_some()
}
pub fn do_osr(&self, func_name: &str, bytecode_offset: usize) -> Result<(), JitError> { let func_id = self.func_ids.get_or_create(func_name);
match self.osr.get_entry(func_id, bytecode_offset) { Some(entry) => { // OSR entry exists, transition is implicit Ok(())
}
None => Err(JitError::OsrFailed("No OSR entry available".to_string())), }
}
```

#### 2.9 Updated stats Implementation

```rust
pub fn stats(&self) -> JitStats { let tiers = self.tiers.read();
let mut stats = JitStats::default();
for (func_id, tier) in tiers.iter() { if let Some(profile) = self.jit.get_profile(*func_id, 0, 0).as_ref() { stats.total_calls += profile.get_call_count();
}
match tier { CompilationTier::Interpreter => stats.tier0_functions += 1, CompilationTier::BaselineJit => stats.tier1_functions += 1, CompilationTier::OptimizingJit => stats.tier2_functions += 1, CompilationTier::AotOptimized => stats.tier3_functions += 1, }
}
stats.total_functions = tiers.len();
stats }
```

### Component 3: Async Integration Fix

File: `dx-py-interpreter/src/async_integration.rs` The async integration needs to be updated to use the actual dx-py-reactor API.

#### 3.1 Check ReactorPool API

First, we need to verify what methods ReactorPool provides. Based on the reactor lib.rs, we have: -`ReactorPool::new(num_reactors)` - Creates a pool -Pool likely provides methods to submit operations

#### 3.2 Simplified Async Integration

If ReactorPool doesn't have `submit_read`/`submit_write`, we can use the lower-level reactor API:
```rust
use dx_py_reactor::{ create_reactor, Reactor, ReactorPool, PyFuture, IoOperation, IoBuffer, ReactorStats };
pub struct AsyncRuntime { /// Individual reactor (simpler than pool for now)
reactor: Option<Box<dyn Reactor>>, /// Pending futures pending: Mutex<Vec<PendingFuture>>, enabled: bool, running: Mutex<bool>, }
impl AsyncRuntime { pub fn init(&mut self) -> Result<(), AsyncError> { if self.reactor.is_some() { return Err(AsyncError::AlreadyInitialized);
}
let reactor = create_reactor(0)
.map_err(|e| AsyncError::InitFailed(e.to_string()))?;
self.reactor = Some(reactor);
self.enabled = true;
Ok(())
}
pub fn read_file(&self, path: &str) -> Result<u64, AsyncError> { let reactor = self.reactor.as_ref()
.ok_or(AsyncError::NotInitialized)?;
// Use IoOperation for file read let buffer = IoBuffer::new(4096);
// ... submit operation to reactor Ok(0) // Return future ID }
}
```

#### 3.3 PyFuture API Adaptation

Check what methods PyFuture provides and adapt:
```rust
// If PyFuture has different methods, adapt accordingly struct PendingFuture { id: u64, future: PyFuture, callback: Option<Box<dyn FnOnce(FutureResult) + Send>>, }
// Adapt to actual PyFuture API fn check_completion(future: &PyFuture) -> bool { // Use whatever method PyFuture provides // e.g., future.is_resolved() or future.poll()
true // placeholder }
```

### Component 4: Test Updates

Update tests in `jit_integration.rs` to use the corrected API:
```rust


#[cfg(test)]


mod tests { use super::*;


#[test]


fn test_jit_integration_creation() { let jit = JitIntegration::new();
assert!(jit.is_enabled());
}


#[test]


fn test_tier_promotion() { let jit = JitIntegration::with_thresholds(10, 100, 1000);
let bytecode = vec![0u8; 100]; // Dummy bytecode // Initial tier is Interpreter assert_eq!(jit.get_tier("test_func"), CompilationTier::Interpreter);
// Record calls until BaselineJit for _ in 0..9 { assert!(jit.record_call("test_func", &bytecode).is_none());
}
// 10th call should trigger BaselineJit assert_eq!( jit.record_call("test_func", &bytecode), Some(CompilationTier::BaselineJit)
);
assert_eq!(jit.get_tier("test_func"), CompilationTier::BaselineJit);
}


#[test]


fn test_stats() { let jit = JitIntegration::with_thresholds(5, 50, 500);
let bytecode = vec![0u8; 100];
for _ in 0..10 { jit.record_call("func1", &bytecode);
}
for _ in 0..3 { jit.record_call("func2", &bytecode);
}
let stats = jit.stats();
assert_eq!(stats.total_functions, 2);
// func1 should be at BaselineJit (10 >= 5)
// func2 should be at Interpreter (3 < 5)
}
}
```

## Data Models

### FunctionIdMapper

```rust
struct FunctionIdMapper { /// Maps function names to their assigned FunctionId name_to_id: RwLock<HashMap<String, FunctionId>>, /// Counter for generating unique IDs next_id: AtomicU64, }
```

### TierThresholds

```rust
struct TierThresholds { /// Calls needed for BaselineJit (default: 100)
baseline: u64, /// Calls needed for OptimizingJit (default: 1000)
optimizing: u64, /// Calls needed for AotOptimized (default: 10000)
aot: u64, }
```

### JitStats (unchanged structure, different tier names)

```rust


#[derive(Debug, Default, Clone)]


pub struct JitStats { pub total_functions: usize, pub total_calls: u64, pub tier0_functions: usize, // Interpreter pub tier1_functions: usize, // BaselineJit pub tier2_functions: usize, // OptimizingJit pub tier3_functions: usize, // AotOptimized }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees. Based on the prework analysis, most acceptance criteria are verified by successful compilation (examples). However, there are a few properties we can test:

### Property 1: Function ID Mapping Consistency

For any function name, calling `get_or_create` multiple times SHALL always return the same FunctionId. Validates: Requirements 5.2

### Property 2: Tier Tracking Consistency

For any function that has been promoted to a tier, calling `get_tier` SHALL return that tier until the function is deoptimized. Validates: Requirements 4.1, 4.2, 4.3

### Property 3: Tier Promotion Monotonicity

For any function, the tier SHALL only increase (never decrease) unless explicitly deoptimized via `deoptimize()`. Validates: Requirements 4.3

### Property 4: Compilation Success Implies Tier Update

For any successful compilation at a tier, the local tier tracking SHALL reflect that tier. Validates: Requirements 4.3, 5.3

## Error Handling

### JitError Variants

```rust


#[derive(Debug, thiserror::Error)]


pub enum JitError {


#[error("JIT is disabled")]


Disabled,


#[error("Compilation failed: {0}")]


CompilationFailed(String),


#[error("Deoptimization failed: {0}")]


DeoptFailed(String),


#[error("OSR failed: {0}")]


OsrFailed(String), }
```

### AsyncError Variants

```rust


#[derive(Debug, thiserror::Error)]


pub enum AsyncError {


#[error("Async runtime not initialized")]


NotInitialized,


#[error("Async runtime already initialized")]


AlreadyInitialized,


#[error("Event loop already running")]


AlreadyRunning,


#[error("Initialization failed: {0}")]


InitFailed(String),


#[error("Submit failed: {0}")]


SubmitFailed(String),


#[error("I/O error: {0}")]


IoError(String),


#[error("Future not found")]


FutureNotFound,


#[error("Timeout")]


Timeout, }
```

## Testing Strategy

### Unit Tests

Unit tests verify specific examples and edge cases: -JIT Integration Tests -Test JitIntegration creation -Test tier promotion at thresholds -Test disabled JIT behavior -Test stats calculation -Test reset functionality -Async Integration Tests -Test AsyncRuntime creation -Test not-initialized error handling -Test pending count tracking

### Property-Based Tests

Property tests verify universal properties across all inputs: -Function ID Mapping Property Test -Generate random function names -Verify consistent ID assignment -Minimum 100 iterations -Tier Tracking Property Test -Generate random call sequences -Verify tier monotonicity -Minimum 100 iterations

### Integration Tests

- Compilation Test
- Run `cargo build
- -release` on dx-py-runtime
- Verify zero compilation errors
- Test Suite Execution
- Run `cargo test
- -lib` on dx-py-runtime
- Verify all tests pass

### Test Framework

- Use Rust's built-in test framework for unit tests
- Use `proptest` crate for property-based tests (already in dev-dependencies)
- Tag property tests with: `// Feature: dx-py-runtime-compilation-fix, Property N: description`
