
# Parametrize Parsing Implementation

## Overview

This document describes the implementation of Task 17.1: Implement parametrize parsing from the dx-py-production-ready spec.

## Requirement

Requirement 12.1: Parse `@pytest.mark.parametrize` decorator and extract parameter names and values.

## Implementation

The parametrize parsing functionality is implemented in `src/parametrize.rs` with the following key components:

### 1. ParameterSet Structure

```rust
pub struct ParameterSet { pub argnames: Vec<String>, // Parameter names (e.g., ["x", "y"])
pub values: Vec<Vec<String>>, // Parameter values - each inner Vec is one set pub ids: Vec<Option<String>>, // Optional custom IDs pub marks: Vec<Vec<String>>, // Marks for each parameter set (e.g., xfail)
}
```
This structure stores the parsed information from a `@pytest.mark.parametrize` decorator.

### 2. ParametrizeExpander

The `ParametrizeExpander` class provides the parsing functionality: -`parse_parametrize_marker`: Extracts parameter names and values from a marker -`parse_values_list`: Parses the values list (handles lists, tuples, pytest.param) -`parse_pytest_param`: Parses `pytest.param(...)` wrappers with custom IDs and marks -`split_list_items`: Splits list items while respecting nested brackets and strings

### 3. Parsing Capabilities

The implementation supports: ✅ Single parameters: `@pytest.mark.parametrize("x", [1, 2, 3])` ✅ Multiple parameters: `@pytest.mark.parametrize("x,y", 1, 2), (3, 4)])` ✅ Custom IDs: `pytest.param(1, id="one")` ✅ Marks: `pytest.param(1, marks=pytest.mark.xfail)` ✅ String values: `["alice", "bob"]` ✅ Mixed types: `1, "a"), (2, "b")]` ✅ Cartesian products: Multiple `@pytest.mark.parametrize` decorators

## Usage Example

```rust
use dx_py_discovery::{ParametrizeExpander, TestScanner};
// Scan a test file let mut scanner = TestScanner::new()?;
let tests = scanner.scan_file("test_example.py")?;
// Expand parametrized tests let expander = ParametrizeExpander::new();
for test in &tests { let expanded = expander.expand(test);
for exp_test in expanded { println!("Test: {}", exp_test.full_id());
println!("Parameters: {:?}", exp_test.param_values);
}
}
```

## Test Coverage

The implementation is thoroughly tested with:

### Unit Tests (`src/parametrize_tests.rs`)

- Single parameter parsing
- Multiple parameter parsing
- Cartesian product expansion
- Custom IDs
- xfail markers
- String values
- Mixed types

### Integration Tests (`tests/parametrize_parsing_test.rs`)

- End-to-end parsing verification
- Edge cases (empty lists, etc.)

### Property-Based Tests

- Expansion count correctness
- Parameter value correctness
- Unique ID generation
- Cartesian product correctness All tests pass successfully:
```
running 17 tests test result: ok. 17 passed; 0 failed ```


## Examples


See `examples/parametrize_demo.rs` for a complete demonstration of the parsing functionality. Run with:
```bash
cd test-runner cargo run --example parametrize_demo -p dx-py-discovery ```

## Integration with Test Discovery

The parametrize parsing integrates with the test discovery pipeline: -Scanner (`scanner.rs`) extracts decorators from Python AST using tree-sitter -Markers are stored in `TestCase` objects -ParametrizeExpander parses the markers and expands tests -Expanded tests are converted back to `TestCase` objects with updated IDs

## Compliance with Requirements

✅ Parse `@pytest.mark.parametrize` decorator: Implemented in `parse_parametrize_marker` ✅ Extract parameter names: Stored in `ParameterSet.argnames` ✅ Extract parameter values: Stored in `ParameterSet.values` The implementation fully satisfies Requirement 12.1.
