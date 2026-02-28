
# dx-www

The Transpiler-to-Binary Pipeline — A revolutionary web framework that compiles `.tsx` to `.dxb` binary artifacts, achieving 46x smaller bundles than Svelte and zero hydration overhead. "The developer writes code. The compiler decides how to execute it."

## Table of Contents

- Overview
- Key Features
- Architecture
- Ecosystem Crates
- Performance
- Quick Start
- Compilation Pipeline
- Runtime Variants
- Binary Protocol (HTIP)
- API Reference
- Development
- Roadmap
- License

## Overview

dx-www is a next-generation web framework built in Rust that fundamentally reimagines how web applications are built and delivered. Instead of shipping JavaScript bundles, dx-www compiles your TSX components into optimized binary artifacts that are interpreted by a tiny WASM runtime.

### The Problem with Traditional Frameworks

+-----------+---------+--------+-----------+-----------+------+-----+-------------+
| Framework | Initial | Bundle | Hydration | Cost      | Time | to  | Interactive |
+===========+=========+========+===========+===========+======+=====+=============+
| React     | ~45     | KB     | High      | 200-500ms | Vue  | ~34 | KB          |
+-----------+---------+--------+-----------+-----------+------+-----+-------------+



### The dx-www Solution

```
Traditional: TSX JavaScript Parse Execute Hydrate Interactive dx-www: TSX Binary Stream Render Interactive (Zero Hydration)
```

## Key Features

### 🚀 Extreme Performance

- 338-byte Micro Runtime — For simple, static-heavy applications
- 7.5 KB Macro Runtime — For complex, interactive applications
- Zero Hydration — Binary templates are directly rendered, no rehydration needed
- < 200ms Hot Reload — WebSocket-based development server with instant updates

### 🔒 Security First

- Banned Keywords Detection — `eval`, `innerHTML`, `dangerouslySetInnerHTML` blocked at compile time
- Ed25519 Signed Payloads — Cryptographic verification of binary artifacts
- No Runtime Code Execution — Pure data interpretation, no `eval` or `Function`

### 🧠 Intelligent Compilation

- Automatic Runtime Selection — Compiler analyzes complexity and chooses optimal runtime
- Tree Shaking — Dead code elimination at compile time
- Template Deduplication — Identical DOM structures share binary representations
- Auto-Import Resolution — Components are automatically discovered and linked

### 📦 Holographic Splitting

- Template Extraction — Static DOM structures separated from dynamic bindings
- Slot-Based Updates — Only changed values are patched, not entire DOM trees
- Binary Diffing — Delta updates for minimal network transfer

## Architecture

@tree[]

## Ecosystem Crates

dx-www is composed of 38 specialized crates, each handling a specific concern:

### Core Crates

+----------+-------------+----------+
| Crate    | Description | Size     |
+==========+=============+==========+
| `dx-www` | Main        | compiler |
+----------+-------------+----------+



### DOM & Rendering

+--------------+-------------+
| Crate        | Description |
+==============+=============+
| `dx-www-dom` | Virtual     |
+--------------+-------------+



### State Management

+----------------+-------------+
| Crate          | Description |
+================+=============+
| `dx-www-state` | Binary      |
+----------------+-------------+



### Data & Forms

+---------------+--------------+
| Crate         | Description  |
+===============+==============+
| `dx-www-form` | Compile-time |
+---------------+--------------+



### Security & Auth

+---------------+-------------+
| Crate         | Description |
+===============+=============+
| `dx-www-auth` | Ed25519     |
+---------------+-------------+



### Accessibility & i18n

+---------------+--------------+
| Crate         | Description  |
+===============+==============+
| `dx-www-a11y` | Compile-time |
+---------------+--------------+



### Infrastructure

+-------------------+-------------+
| Crate             | Description |
+===================+=============+
| `dx-www-fallback` | HTML        |
+-------------------+-------------+



## Performance

### Bundle Size Comparison

@tree[]

### Benchmark Results

+--------+--------+-------+-------------+
| Metric | dx-www | React | Improvement |
+========+========+=======+=============+
| Create | 10K    | rows  | 4ms         |
+--------+--------+-------+-------------+



## Quick Start

### Installation

```bash


# Add to your Cargo.toml


[dependencies]
dx-www = "0.1"
```

### Basic Usage

```rust
use dx_compiler::{compile_tsx, analyze_tsx, CompileResult};
use std::path::Path;
fn main() -> anyhow::Result<()> { // Compile a TSX file to binary artifacts let result = compile_tsx( Path::new("src/App.tsx"), Path::new("dist"), true, // verbose )?;
println!("Runtime: {:?}", result.runtime_variant);
println!("Compile time: {}ms", result.compile_time_ms);
println!("Output size: {} bytes", result.total_size);
Ok(())
}
```

### Example TSX Component

```tsx
// App.tsx import { useState } from 'dx';
export default function App() { const [count, setCount] = useState(0);
return ( <div class="counter"> <h1>Count: {count}</h1> <button onClick={() => setCount(count + 1)}> Increment </button> </div> );
}
```

### Compilation Output

```
🏭 Compiling src/App.tsx dist 📊 Complexity Analysis:
Components: 1 State Variables: 1 Event Handlers: 1 JSX Nodes: 4 State: Low 🎯 Decision: Micro (338 bytes) - Optimized for simplicity Generating HTIP binary stream...
HTIP stream size: 127 bytes String table: 3 entries Templates: 1 entries Opcodes: 2 entries ✓ Packed to: dist/app.dxb (156 bytes - TINY!)
✓ Compilation complete in 12ms Total size: 283 bytes ```


## Compilation Pipeline



### Stage 1: Parsing


The parser reads `.tsx` files and builds a dependency graph with security validation.
```rust
// Security: Banned keywords are rejected at parse time const BANNED_KEYWORDS: &[&str] = &[ "eval", "innerHTML", "outerHTML", "document.write", "Function", "dangerouslySetInnerHTML"
];
```


### Stage 2: Analysis


The analyzer computes complexity metrics and selects the optimal runtime.
```rust
pub struct ComplexityMetrics { pub component_count: usize, pub total_state_vars: usize, pub event_handler_count: usize, pub max_component_depth: usize, pub has_async_logic: bool, pub total_jsx_nodes: usize, pub state_complexity: StateComplexity, }
```


### Stage 3: Holographic Splitting


The splitter separates static templates from dynamic bindings.
```
Input: <div class="box">Count: {state.count}</div> Output:
Template: <div class="box">Count: <!--SLOT_0--></div> Binding: SLOT_0 self.count ```

### Stage 4: HTIP Code Generation

Binary opcodes are generated for the runtime interpreter.
```rust
// HTIP Opcodes Clone = 1 // Clone template to DOM PatchText = 2 // Update text slot PatchAttr = 3 // Update attribute Remove = 4 // Remove node ```


### Stage 5: Packing


Final `.dxb` artifact is created with compression. @tree:.dxb Format:[]


## Runtime Variants



### Micro Runtime (338 bytes)


Selected when: -Components < 10 -State complexity: Low/Medium -Event handlers < 10 -No complex async logic -JSX nodes < 50 ```rust // Decision matrix if state_complexity == Low && component_count < 10 && event_handlers < 10 { RuntimeVariant::Micro }
```

### Macro Runtime (7.5 KB)

Selected when: -Components ≥ 10 -High state complexity (6+ vars, arrays, objects) -Many event handlers (≥ 10) -Complex async logic with multiple hooks -Deep component trees (> 5 levels)

## Binary Protocol (HTIP)

HTIP (Holographic Template Instruction Protocol) is the binary format that replaces HTML and JavaScript.

### Header Structure

```rust
struct HtipHeader { magic: u16, // 0x4458 ("DX")
version: u8, // Protocol version flags: u8, // Feature flags template_count: u16, // Number of templates string_count: u16, // String table size opcode_count: u32, // Number of opcodes payload_size: u32, // Total payload bytes }
```

### Opcode Format

```rust
struct Opcode { op_type: u8, // Operation type reserved: u8, // Future use target_id: u16, // Target node ID value: u16, // String index or value extra: u16, // Additional data }
```

## API Reference

### Core Functions

```rust
/// Compile TSX to binary artifacts pub fn compile_tsx( entry: &Path, output: &Path, verbose: bool ) -> Result<CompileResult>;
/// Analyze without compiling pub fn analyze_tsx( entry: &Path, verbose: bool ) -> Result<(ComplexityMetrics, RuntimeVariant)>;
/// Quick compilation check pub fn can_compile(entry: &Path) -> bool;
```

### CompileResult

```rust
pub struct CompileResult { pub runtime_variant: RuntimeVariant, pub metrics: ComplexityMetrics, pub htip_path: PathBuf, pub templates_path: PathBuf, pub rust_path: Option<PathBuf>, pub compile_time_ms: u128, pub total_size: u64, }
```

## Development

### Building

```bash


# Build all crates


cargo build --release


# Build with OXC parser (faster)


cargo build --release --features oxc


# Run tests


cargo test


# Run benchmarks


cargo bench ```


### Dev Server


```bash

# Start development server with hot reload

dx dev --entry pages --port 3000 ```

### Project Structure

@tree:crates/dx-www[]

## Binary Dawn Features (25 Revolutionary Features)

dx-www now includes 25 binary-first features with 328 passing tests, delivering unprecedented performance:

### Performance Highlights

+--------------+-------------+----------------+
| Feature      | Performance | Comparison     |
+==============+=============+================+
| Compile-Time | Reactivity  | 0.001ms/update |
+--------------+-------------+----------------+



### Complete Feature List

+---+--------------+------------+-----------------+
| # | Feature      | Module     | Description     |
+===+==============+============+=================+
| 1 | Compile-Time | Reactivity | `reactivity.rs` |
+---+--------------+------------+-----------------+



### Test Coverage

```
running 328 tests test result: ok. 328 passed; 0 failed; 0 ignored ```
All 39 correctness properties validated with property-based testing using `proptest`.


## Roadmap



### Completed ✅


- TSX to binary compilation pipeline
- Micro/Macro runtime selection
- HTIP binary protocol
- Template deduplication
- Auto-import linker
- Hot reload dev server
- 38 ecosystem crates
- Binary Dawn Features (25 features, 328 tests)


### In Progress 🚧


- OXC parser integration (faster parsing)
- Full JSX AST support
- Source maps for debugging
- Edge deployment (Cloudflare Workers)


### Planned 📋


- dx-openapi (Auto Swagger generation)
- dx-admin (CRUD dashboard generator)
- dx-actuator (Health checks, metrics)
- Visual Studio Code extension


## Comparison with Frameworks


+---------+--------+------------+--------+-------+
| Feature | dx-www | React      | Svelte | Qwik  |
+=========+========+============+========+=======+
| Bundle  | Size   | 338B-7.5KB | 45KB   | 7.3KB |
+---------+--------+------------+--------+-------+


## License


Licensed under either of: -Apache License, Version 2.0 (LICENSE-APACHE (LICENSE-APACHE) or //www.apache.org/licenses/LICENSE-2.0) -MIT License (LICENSE-MIT (LICENSE-MIT) or //opensource.org/licenses/MIT) at your option.
