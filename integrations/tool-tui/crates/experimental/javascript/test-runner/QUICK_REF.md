
# DX Test Runner - Quick Reference

## Installation

```bash


# Build from source


cd crates/dx-js-test-runner cargo build --release


# Binary location


./target/release/dx-test ```


## Basic Usage


```bash

# Run all tests

dx-test

# Run with pattern filter

dx-test "array"

# Verbose output

dx-test --verbose

# Without parallel execution

dx-test --no-parallel ```

## Cache Management

```bash


# Show cache statistics


dx-test cache


# Clear cache


dx-test clear


# Cache location: %TEMP%\dx-test-cache\


```

## Architecture

### Components

```
dx-test-core Binary formats & types dx-test-cache O(1) layout cache dx-test-vm Custom bytecode VM dx-test-executor Parallel work-stealing executor dx-test-cli Command-line interface ```


### Data Flow



## Performance Metrics


+-----------+-------+------+
| Phase     | Cold  | Warm |
+===========+=======+======+
| Discovery | 102ms | 5ms  |
+-----------+-------+------+


## API (Future)


```javascript
// Import from dx-test import { test, expect } from 'dx-test';
// Simple test test('addition', () => { expect(1 + 1).toBe(2);
});
// Async test test('fetch data', async () => { const data = await fetchData();
expect(data).toBeDefined();
});
// Assertions expect(value).toBe(expected);
expect(value).toEqual(expected);
expect(value).toBeTruthy();
expect(value).toBeFalsy();
expect(value).toBeNull();
expect(value).toBeDefined();
```


## Bytecode Format


```rust
// Opcodes PushInt(i32) 0x20 PushTrue 0x22 PushFalse 0x23 AssertEq 0x50 AssertTruthy 0x52 TestPass 0xF0 End 0xFF // Example bytecode
[0x20, 0x01, 0x00, 0x00, 0x00] // PushInt(1)
[0x22] // PushTrue
[0x52] // AssertTruthy
[0xF0] // TestPass
[0xFF] // End ```

## Comparison

```
Bun: 297ms (for 50 tests)
DX: 11ms (26x faster)
```

## Development

```bash


# Run tests on crate itself


cargo test


# Format code


cargo fmt


# Check for issues


cargo clippy


# Build docs


cargo doc --open ```


## Future Features


- Watch mode
- Coverage reporting
- Snapshot testing
- Mock/Spy utilities
- Test prediction (skip unchanged)
- SIMD batch assertions
- Browser test runner


## License


MIT
