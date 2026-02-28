
# dx-www API Documentation

This directory contains API documentation for the dx-www framework.

## Generating Documentation

To generate the full API documentation, run:
```bash
cargo doc --workspace --no-deps --open ```
This will build documentation for all crates and open it in your browser.


## Crate Overview



### Core Crates


+-------------------+-------------+
| Crate             | Description |
+===================+=============+
| `dx-www-compiler` | TSX/JSX     |
+-------------------+-------------+


### Feature Crates


+---------------+---------------+
| Crate         | Description   |
+===============+===============+
| `dx-www-a11y` | Accessibility |
+---------------+---------------+


### Utility Crates


+----------------+-------------+
| Crate          | Description |
+================+=============+
| `dx-www-error` | Error       |
+----------------+-------------+


## Key Types



### Parser Types


```rust
// Parsed module representation pub struct ParsedModule { pub path: PathBuf, pub imports: Vec<ImportDecl>, pub exports: Vec<ExportDecl>, pub components: Vec<Component>, pub hash: String, }
// Component definition pub struct Component { pub name: String, pub props: Vec<PropDef>, pub state: Vec<StateDef>, pub jsx_body: String, pub hooks: Vec<HookCall>, pub is_async: bool, pub has_children: bool, }
```


### Compilation Types


```rust
// Compilation result pub struct CompileResult { pub runtime_variant: RuntimeVariant, pub metrics: ComplexityMetrics, pub htip_path: PathBuf, pub templates_path: PathBuf, pub rust_path: Option<PathBuf>, pub compile_time_ms: u128, pub total_size: u64, }
// Runtime selection pub enum RuntimeVariant { Micro, // 338 bytes - simple apps Macro, // 7.5 KB - complex apps }
```


### Binary Protocol Types


```rust
// HTIP opcodes pub enum HtipOp { Clone(u8), // Clone template PatchText(u8, String), // Update text content PatchAttr(u8, String, String), // Update attribute ClassToggle(u8, String, bool), // Toggle class Remove(u8), // Remove node Event(u8, u8, u16), // Register event handler Eof, // End of stream }
// Delta patch instructions pub enum DeltaInstruction { Copy(u32), // Copy block from base Literal(Vec<u8>), // Insert literal bytes }
```


## Common Patterns



### Compiling TSX


```rust
use dx_www_compiler::{compile_tsx, CompileResult};
use std::path::Path;
fn compile_app() -> anyhow::Result<CompileResult> { let entry = Path::new("src/App.tsx");
let output = Path::new("dist");
compile_tsx(entry, output, true) // verbose=true }
```


### Analyzing Without Compiling


```rust
use dx_www_compiler::{analyze_tsx, RuntimeVariant};
use std::path::Path;
fn check_complexity() -> anyhow::Result<()> { let entry = Path::new("src/App.tsx");
let (metrics, variant) = analyze_tsx(entry, false)?;
println!("Components: {}", metrics.component_count);
println!("Runtime: {:?}", variant);
Ok(())
}
```


### Server-Side Rendering


```rust
use dx_www_server::{Server, SsrConfig};
async fn start_server() -> anyhow::Result<()> { let config = SsrConfig { htip_path: "dist/app.htip".into(), port: 3000, ssr_enabled: true, };
Server::new(config).run().await }
```


## Error Handling


All fallible operations return `Result<T, E>` where `E` implements `std::error::Error`.
```rust
use dx_www_compiler::compile_tsx;
use std::path::Path;
fn main() { let result = compile_tsx( Path::new("src/App.tsx"), Path::new("dist"), false, );
match result { Ok(compile_result) => { println!("Compiled to: {}", compile_result.htip_path.display());
}
Err(e) => { eprintln!("Compilation failed: {}", e);
std::process::exit(1);
}
}
}
```


## Feature Flags


Many crates support feature flags for optional functionality:
```toml
[dependencies]
dx-www-compiler = { version = "0.1", features = ["oxc"] }
dx-www-reactor = { version = "0.1", features = ["io_uring"] }
dx-www-client = { version = "0.1", features = ["wasm"] }
```


## See Also


- Getting Started Guide (../getting-started.md)
- Examples (../examples/README.md)
- Architecture Guide (../architecture.md)
