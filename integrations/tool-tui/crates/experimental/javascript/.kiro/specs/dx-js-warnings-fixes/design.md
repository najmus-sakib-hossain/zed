
# Design Document

## Overview

This document describes the technical design for fixing build warnings and the JSON parsing issue in the dx-js runtime. The fixes are organized into three categories: critical errors (bit mask issues), clippy warnings, and JSON.parse error handling.

## Architecture

The fixes span multiple modules in the runtime: @tree:runtime/src[]

## Components and Interfaces

### Component 1: Tagged Value Bit Mask Fix

The current implementation has conflicting tag constants:
```rust
// Current (broken):
const TAG_MASK: u64 = 0xFFFF_0000_0000_0000;
const TAG_OBJECT: u64 = 0xFFF8_0001_0000_0000; // Has bits outside TAG_MASK!
const TAG_ARRAY: u64 = 0xFFF8_0002_0000_0000; // Has bits outside TAG_MASK!
const TAG_FUNCTION: u64 = 0xFFF8_0003_0000_0000; // Has bits outside TAG_MASK!
```
The fix requires using a wider mask for heap object subtypes:
```rust
// Fixed:
const TAG_MASK: u64 = 0xFFFF_0000_0000_0000;
const SUBTYPE_MASK: u64 = 0xFFFF_FFFF_0000_0000; // Includes subtype bits // Type checking for heap objects uses SUBTYPE_MASK fn is_object(&self) -> bool { (self.0 & SUBTYPE_MASK) == TAG_OBJECT }
```

### Component 2: Default Trait Implementations

Add `impl Default` for structs with `new()` methods:
```rust
impl Default for BuiltinRegistry { fn default() -> Self { Self::new()
}
}
```
Affected structs: -`BuiltinRegistry` -`FunctionCompiler` -`ModuleResolver` -`Promise` -`EventLoop` -`ReadableStream` -`WritableStream` -`BatchConsole`

### Component 3: JSON.parse Error Handling

Current implementation returns `f64::NAN` (undefined) on parse errors. Fix to throw proper errors:
```rust
extern "C" fn builtin_json_parse(json_str_id: f64) -> f64 { // ... get string from heap ...
match serde_json::from_str::<serde_json::Value>(&json_str) { Ok(json_value) => json_to_runtime_value(json_value, &mut heap), Err(e) => { // Set error state and return error indicator set_runtime_error(format!( "SyntaxError: {} at line {}, column {}", e, e.line(), e.column()
));
f64::NAN // Still return NAN but error is set }
}
}
```

## Data Models

No new data models required. Existing models are unchanged.

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system-, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Heap Object Type Identification

For any heap object pointer and object type (string, object, array, function), creating a TaggedValue with that type and then checking `is_<type>()` SHALL return true, and checking other type methods SHALL return false. Validates: Requirements 1.1, 1.2, 1.3

### Property 2: JSON Parse Error Handling

For any invalid JSON string, calling JSON.parse SHALL result in an error (not undefined), and the error message SHALL contain position information. Validates: Requirements 9.1, 9.2

### Property 3: JSON Round-Trip

For any valid JSON value (null, boolean, number, string, array, object), `JSON.parse(JSON.stringify(value))` SHALL produce an equivalent value. Validates: Requirements 9.3

## Error Handling

### JSON.parse Errors

When JSON.parse encounters invalid JSON: -Parse the JSON using serde_json -On error, extract line and column from serde_json error -Set runtime error state with SyntaxError message -Return NaN (undefined) but with error flag set

### Build Errors

The bit mask errors in tagged.rs are critical and must be fixed first, as they prevent compilation with clippy.

## Testing Strategy

### Unit Tests

- Test each Default trait implementation compiles and works
- Test TaggedValue type checking for all heap object types
- Test JSON.parse with various invalid inputs

### Property-Based Tests

Using proptest for Rust: -Heap Object Type Property: Generate random pointers and object types, verify type checking works correctly -JSON Error Property: Generate invalid JSON strings, verify errors are thrown with position info -JSON Round-Trip Property: Generate valid JSON values, verify parse(stringify(x)) == x

### Test Configuration

- Minimum 100 iterations per property test
- Use proptest crate for property-based testing
- Tag format: Feature: dx-js-warnings-fixes, Property N: description
