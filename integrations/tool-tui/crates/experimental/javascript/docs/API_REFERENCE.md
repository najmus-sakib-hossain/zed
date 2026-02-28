
# DX-JS API Reference

⚠️ Early Development Notice (v0.0.1) DX-JS is in early development. APIs documented here may change without notice. Some features may be incomplete or experimental. This document provides comprehensive API documentation for all DX-JS tools and their Rust APIs.

## Table of Contents

- Package Manager (dx)
- Runtime (dx-js)
- Bundler (dx-bundle)
- Test Runner (dx-test)
- Rust API
- Error Types
- Configuration

## Package Manager (dx)

### Commands

+---------+-------------+
| Command | Description |
+=========+=============+
| `dx     | init`       |
+---------+-------------+



### Version Specifiers

+--------+---------+---------+
| Syntax | Example | Matches |
+========+=========+=========+
| Exact  | `1.2.3` | Only    |
+--------+---------+---------+



### Global Installation

Global packages are installed to: -Linux/macOS: `~/.dx/global/node_modules` -Windows: `%USERPROFILE%\.dx\global\node_modules` Binaries are symlinked to: -Linux/macOS: `~/.dx/bin` -Windows: `%USERPROFILE%\.dx\bin`

## Runtime (dx-js)

### Command Line Usage

```bash
dx-js [options] [file] [args...]
```

### Options

t:0(Option,Description)[]

### Examples

```bash


# Run a JavaScript file


dx-js script.js


# Run a TypeScript file


dx-js app.ts


# Start interactive REPL


dx-js


# Run with arguments


dx-js script.js --config=prod arg1 arg2


# Evaluate code


dx-js -e "console.log(1 + 2)"


# Set memory limit


dx-js --max-heap-size=1024 large-script.js ```


### Environment Variables


+----------+-------------+---------+
| Variable | Description | Default |
+==========+=============+=========+
| `DX      | DEBUG`      | Enable  |
+----------+-------------+---------+


### Supported JavaScript Features


ES2022 Features: -Classes with public/private fields -Static class blocks -Top-level await -`Array.prototype.at()` -`Object.hasOwn()` -RegExp match indices ES2021 Features: -Logical assignment operators (`||=`, `&&=`, `??=`) -Numeric separators (`1_000_000`) -`String.prototype.replaceAll()` -`Promise.any()` -WeakRefs (partial) ES2020 Features: -Optional chaining (`?.`) - coming soon -Nullish coalescing (`??`) -`BigInt` - coming soon -`Promise.allSettled()` -`globalThis` TypeScript Support: -Type annotations (stripped at runtime) -Interfaces and type aliases -Enums (const enums only) -Generics -Decorators - coming soon


## Bundler (dx-bundle)



### Commands


```bash
dx-bundle bundle <entry> [options] # Bundle files dx-bundle cache # Show cache statistics dx-bundle clear # Clear the cache ```

### Bundle Options

+------------+-------+------------------+-------------+
| Option     | Short | Default          | Description |
+============+=======+==================+=============+
| `--output` | `-o`  | `dist/bundle.js` | Output      |
+------------+-------+------------------+-------------+



### Output Formats

+--------+-------------+---------+--------+
| Format | Description | Use     | Case   |
+========+=============+=========+========+
| `esm`  | ES          | Modules | Modern |
+--------+-------------+---------+--------+



### Examples

```bash


# Basic bundle


dx-bundle bundle src/index.ts -o dist/bundle.js


# Production bundle


dx-bundle bundle src/index.ts \
- output dist/bundle.min.js \
- format esm \
- minify \
- sourcemap


# Watch mode


dx-bundle bundle src/index.ts -o dist/bundle.js --watch


# External packages


dx-bundle bundle src/index.ts \
- external react \
- external react-dom


# Define constants


dx-bundle bundle src/index.ts \
- define "process.env.NODE_ENV='production'"
```

## Test Runner (dx-test)

### Commands

```bash
dx-test [pattern] [options] # Run tests dx-test cache # Show cache statistics dx-test clear # Clear the cache ```


### Options


+-----------+-------+-------------+
| Option    | Short | Description |
+===========+=======+=============+
| `--watch` | `-w`  | Watch       |
+-----------+-------+-------------+


### Test API



#### Test Definition


```javascript
// Basic test test('description', () => { expect(1 + 1).toBe(2);
});
// Async test test('async test', async () => { const result = await fetchData();
expect(result).toBeDefined();
});
// Skip a test test.skip('skipped test', () => { });
// Only run this test test.only('focused test', () => { });
// Test with timeout test('slow test', () => { }, 10000);
```


#### Test Grouping


```javascript
describe('Math operations', () => { beforeAll(() => { // Run once before all tests });
afterAll(() => { // Run once after all tests });
beforeEach(() => { // Run before each test });
afterEach(() => { // Run after each test });
test('adds numbers', () => { expect(1 + 1).toBe(2);
});
describe('nested group', () => { test('nested test', () => { });
});
});
```


#### Assertions


```javascript
// Equality expect(value).toBe(expected); // Strict equality (===)
expect(value).toEqual(expected); // Deep equality expect(value).toStrictEqual(expected); // Strict deep equality // Truthiness expect(value).toBeTruthy();
expect(value).toBeFalsy();
expect(value).toBeNull();
expect(value).toBeUndefined();
expect(value).toBeDefined();
expect(value).toBeNaN();
// Numbers expect(value).toBeGreaterThan(n);
expect(value).toBeGreaterThanOrEqual(n);
expect(value).toBeLessThan(n);
expect(value).toBeLessThanOrEqual(n);
expect(value).toBeCloseTo(n, precision);
// Strings expect(str).toMatch(/pattern/);
expect(str).toContain('substring');
expect(str).toHaveLength(n);
// Arrays/Iterables expect(arr).toContain(item);
expect(arr).toContainEqual(item);
expect(arr).toHaveLength(n);
// Objects expect(obj).toHaveProperty('key');
expect(obj).toHaveProperty('key', value);
expect(obj).toMatchObject(partial);
// Exceptions expect(() => fn()).toThrow();
expect(() => fn()).toThrow('message');
expect(() => fn()).toThrow(ErrorType);
// Promises await expect(promise).resolves.toBe(value);
await expect(promise).rejects.toThrow();
// Snapshots expect(value).toMatchSnapshot();
expect(value).toMatchInlineSnapshot(`expected`);
// Negation expect(value).not.toBe(other);
```


#### Mocking


```javascript
// Mock function const mockFn = jest.fn();
mockFn.mockReturnValue(42);
mockFn.mockImplementation((x) => x * 2);
// Spy on method const spy = jest.spyOn(object, 'method');
spy.mockReturnValue('mocked');
// Mock module jest.mock('./module', () => ({ default: jest.fn(), namedExport: jest.fn(), }));
// Clear mocks jest.clearAllMocks();
jest.resetAllMocks();
jest.restoreAllMocks();
```


#### Timers


```javascript
// Use fake timers jest.useFakeTimers();
// Advance time jest.advanceTimersByTime(1000);
jest.runAllTimers();
jest.runOnlyPendingTimers();
// Restore real timers jest.useRealTimers();
```


## Rust API



### Runtime


```rust
use dx_js_runtime::{DxRuntime, DxConfig};
use std::path::PathBuf;
// Create runtime with defaults let mut runtime = DxRuntime::new()?;
// Create runtime with configuration let config = DxConfig { cache_dir: PathBuf::from(".dx/cache"), max_heap_size: 512 * 1024 * 1024, // 512MB ..Default::default()
};
let mut runtime = DxRuntime::with_config(config)?;
// Run a file let result = runtime.run_file("script.js")?;
// Run source code let result = runtime.run_sync("console.log('hello')", "inline.js")?;
// Get memory usage let usage = runtime.memory_usage();
println!("Heap used: {} bytes", usage.heap_used);
```


### Error Handling


```rust
use dx_js_runtime::error::{DxError, DxResult, JsException};
fn run_code() -> DxResult<()> { let mut runtime = DxRuntime::new()?;
match runtime.run_sync("throw new Error('oops')", "test.js") { Ok(result) => println!("Result: {:?}", result), Err(DxError::TypeError { message }) => { eprintln!("Type error: {}", message);
}
Err(DxError::ReferenceError { name }) => { eprintln!("Reference error: {} is not defined", name);
}
Err(e) => eprintln!("Error: {}", e), }
Ok(())
}
```


### Garbage Collection


```rust
use dx_js_runtime::gc::{GcHeap, GcConfig, MemoryUsage};
// Create heap with custom config let config = GcConfig { max_heap_size: 256 * 1024 * 1024, // 256MB young_size: 16 * 1024 * 1024, // 16MB old_size: 240 * 1024 * 1024, // 240MB ..Default::default()
};
let mut heap = GcHeap::with_config(config)?;
// Get memory statistics let usage: MemoryUsage = heap.memory_usage();
println!("Heap total: {}", usage.heap_total);
println!("Heap used: {}", usage.heap_used);
println!("RSS: {}", usage.rss);
// Force garbage collection heap.force_gc();
```


### Feature Detection


```rust
use dx_js_runtime::features::{DxFeatures, DxGlobal};
// Get current feature support let features = DxFeatures::current();
println!("ES2022 supported: {}", features.es2022);
println!("TypeScript supported: {}", features.typescript);
// Check specific feature if let Some(supported) = features.is_supported("decorators") { println!("Decorators: {}", if supported { "yes" } else { "no" });
}
// Get all feature names let names = DxFeatures::feature_names();
for name in names { println!("Feature: {}", name);
}
```


## Error Types



### DxError Variants


```rust
pub enum DxError { // Parse errors ParseError(String), ParseErrorWithLocation { file, line, column, message }, // Runtime errors TypeError { message }, ReferenceError { name }, SyntaxError { message, line, column }, RangeError(String), // Module errors ModuleNotFound(String), ModuleNotFoundDetailed { specifier, importer, searched_paths }, // Package errors PackageInstallError { package, version, reason, suggestion }, NetworkError { message, url }, // Feature errors UnsupportedFeature { feature, description, suggestion }, UnsupportedOptions { api, options, supported }, // Internal errors CompileError(String), RuntimeError(String), IoError(String), CacheError(String), Internal(String), }
```


### Creating Errors


```rust
use dx_js_runtime::error::{DxError, unsupported_feature, unsupported_options};
// Unsupported feature error let err = unsupported_feature( "decorators", "Stage 3 decorators are not yet supported", "Use higher-order functions instead"
);
// Unsupported options error let err = unsupported_options( "fs.readFile", &["signal", "flag"], &["encoding", "mode"]
);
// Not implemented error let err = DxError::not_implemented("fs.watch");
```


## Configuration



### dx.config.json


```json
{ "runtime": { "maxHeapSize": 512, "experimental": [], "moduleResolution": "node"
}, "packageManager": { "registry": "https://registry.npmjs.org", "cacheDir": ".dx-cache"
}, "bundler": { "target": "es2022", "minify": true, "sourceMaps": true }, "testRunner": { "parallel": true, "coverage": false, "timeout": 5000 }
}
```


### Configuration Options



#### Runtime Options


+---------------+--------+---------+-------------+
| Option        | Type   | Default | Description |
+===============+========+=========+=============+
| `maxHeapSize` | number | 512     | Maximum     |
+---------------+--------+---------+-------------+


#### Package Manager Options


+------------+--------+---------+-------------+
| Option     | Type   | Default | Description |
+============+========+=========+=============+
| `registry` | string | npm     | registry    |
+------------+--------+---------+-------------+


#### Bundler Options


+----------+--------+----------+-------------+
| Option   | Type   | Default  | Description |
+==========+========+==========+=============+
| `target` | string | "es2022" | Target      |
+----------+--------+----------+-------------+


#### Test Runner Options


+------------+---------+---------+-------------+
| Option     | Type    | Default | Description |
+============+=========+=========+=============+
| `parallel` | boolean | true    | Run         |
+------------+---------+---------+-------------+


## See Also


- Getting Started (./GETTING_STARTED.md)
- Quick start guide
- Benchmarks (./BENCHMARKS.md)
- Performance information
- Compatibility Matrix (./COMPATIBILITY.md)
- Node.js API support
