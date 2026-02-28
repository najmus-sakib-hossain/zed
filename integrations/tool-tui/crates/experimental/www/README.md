
# dx-www

(LICENSE) A high-performance web framework with a transpiler-to-binary pipeline that converts TSX to optimized binary formats.

## Overview

`dx-www` is the core web framework for the DX ecosystem. It provides a complete solution for building modern web applications with a focus on performance through binary compilation and efficient reactivity.

## Subcrates

t:0(Crate,Description)[]

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
dx-www = "1.0.0"
```

## Usage

```rust
use dx_compiler::prelude::*;
fn main() -> anyhow::Result<()> { // Compile TSX to binary let compiler = Compiler::new();
compiler.compile("./src/app.tsx", "./dist")?;
Ok(())
}
```

## Features

- TSX to binary compilation
- Server-side rendering
- Reactive state management
- Form handling with validation
- Data fetching with caching
- Accessibility built-in
- Offline support

## License

This project is dual-licensed under MIT OR Apache-2.0.
