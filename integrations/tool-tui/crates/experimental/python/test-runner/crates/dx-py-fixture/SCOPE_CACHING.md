
# Fixture Scope Caching Implementation

## Overview

This document describes the implementation of fixture scope caching for the dx-py test runner, which enables pytest-compatible fixture behavior where fixtures are created and cached according to their scope (function, class, module, session).

## Requirements

Requirement 11.2: WHEN a fixture has `scope="module"`, THE Test_Runner SHALL create it once per module Requirement 11.3: WHEN a fixture has `scope="session"`, THE Test_Runner SHALL create it once per test session

## Design

### Scope Instance Tracking

The key insight is that fixtures need to be cached per scope instance, not just per scope type. A scope instance uniquely identifies the context in which a fixture should be cached: -Function scope: No caching - always create new (each test is a new instance) -Class scope: Cache per (module_path, class_name) combination -Module scope: Cache per module_path -Session scope: Cache globally (single instance for entire session)

### Data Structures

#### ScopeInstance Enum

```rust
pub enum ScopeInstance { Function, Class { module_path: PathBuf, class_name: String }, Module { module_path: PathBuf }, Session, }
```
This enum represents a unique scope instance and implements `PartialEq`, `Eq`, and `Hash` so it can be used as a HashMap key.

#### FixtureManager Enhancement

The `FixtureManager` was enhanced with:
```rust
pub struct FixtureManager { // ... existing fields ...
/// Cached fixture values per scope instance /// Maps (fixture_name, scope_instance) -> cached_bytes scope_cache: HashMap<(String, ScopeInstance), Vec<u8>>, }
```

### Key Methods

#### `resolve_fixtures_for_test_with_context`

This is the enhanced fixture resolution method that takes test context (module path and class name) and properly caches fixtures per scope instance:
```rust
pub fn resolve_fixtures_for_test_with_context( &self, test_parameters: &[String], module_path: &PathBuf, class_name: Option<&String>, ) -> Result<Vec<ResolvedFixture>, FixtureError> ```
For each resolved fixture, it: -Creates a `ScopeInstance` based on the fixture's scope and test context -Checks if a cached value exists for that (fixture_name, scope_instance) pair -Sets `needs_setup = false` if cached (except for function scope which always needs setup)


#### `cache_for_scope`


Stores a fixture value for a specific scope instance:
```rust
pub fn cache_for_scope( &mut self, fixture_name: &str, scope_instance: ScopeInstance, value: Vec<u8>, )
```
Function-scoped fixtures are explicitly not cached (the method returns early).


#### `clear_scope_cache`


Clears all cached fixtures for a specific scope instance:
```rust
pub fn clear_scope_cache( &mut self, scope: FixtureScope, module_path: &PathBuf, class_name: Option<&String>, )
```
This should be called when a scope ends (e.g., end of module, end of class).


## Testing



### Unit Tests


File: `tests/scope_caching_test.rs` Comprehensive unit tests covering: -Function scope never caches -Module scope caches per module -Class scope caches per class -Session scope caches globally -Clearing scope cache -Mixed scopes in single test -Scope instance equality


### Property-Based Tests


File: `tests/scope_semantics_property.rs` Property-based tests validating Property 21: Fixture Scope Semantics: -Function scope never caches: For any function-scoped fixture, each test receives fresh setup -Module scope caches per module: All tests in same module share cached value -Class scope caches per class: All tests in same class share cached value -Session scope caches globally: All tests share cached value regardless of context -Scope instance identity: Same context produces equal scope instances -Clear scope cache isolation: Clearing one scope doesn't affect others -Multiple scopes cache independently: Each fixture caches according to its own scope All property tests run 100 cases each using proptest.


## Test Results


```
Running unittests src\lib.rs: 53 tests passed Running tests\fixture_injection_integration.rs: 4 tests passed Running tests\property_tests.rs: 17 tests passed Running tests\scope_caching_test.rs: 7 tests passed Running tests\scope_semantics_property.rs: 7 tests passed Total: 88 tests passed ```

## Usage Example

```rust
let mut manager = FixtureManager::new(cache_dir)?;
// Register a module-scoped fixture let fixture = FixtureDefinition::new("db", "tests/conftest.py", 10)
.with_scope(FixtureScope::Module);
manager.register(fixture);
let module_path = PathBuf::from("tests/test_api.py");
// First test - needs setup let resolved = manager.resolve_fixtures_for_test_with_context( &["db".to_string()], &module_path, None, )?;
assert!(resolved[0].needs_setup);
// Simulate fixture setup let scope_instance = ScopeInstance::from_test_context( FixtureScope::Module, &module_path, None, );
manager.cache_for_scope("db", scope_instance, vec![1, 2, 3]);
// Second test in same module - uses cached value let resolved = manager.resolve_fixtures_for_test_with_context( &["db".to_string()], &module_path, None, )?;
assert!(!resolved[0].needs_setup);
assert_eq!(resolved[0].cached_value, Some(vec![1, 2, 3]));
```

## Implementation Notes

- Function scope special case: Function-scoped fixtures explicitly bypass caching to ensure each test gets a fresh instance.
- Session scope simplification: Session scope instances are always equal regardless of module/class context, ensuring global caching.
- Backward compatibility: The original `resolve_fixtures_for_test` method still exists for tests that don't need scope context.
- Memory efficiency: Cached values are stored as `Vec<u8>` (serialized bytes) to support any fixture type.
- Scope lifecycle: The `clear_scope_cache` method should be called by the test executor when scopes end (e.g., after all tests in a module complete).

## Future Enhancements

- Automatic scope cleanup: Integrate with test executor to automatically clear scope caches at appropriate times
- Fixture value serialization: Add helper methods to serialize/deserialize fixture values
- Cache statistics: Track cache hits/misses for performance monitoring
- Scope hierarchy: Implement proper scope hierarchy where session fixtures are available to all lower scopes
