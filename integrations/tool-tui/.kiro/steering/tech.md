
# Dx Technical Stack

## Language & Build System

- Primary Language: Rust (2024 Edition)
- Build System: Cargo workspace with 45+ crates
- Target Platforms: Native + WebAssembly
- Minimum Rust Version: 1.85+

## Key Dependencies

### Core

- `wasm-bindgen` / `web-sys` / `js-sys`
- WASM interop
- `tokio`
- Async runtime
- `serde` / `serde_json`
- Serialization
- `thiserror` / `anyhow`
- Error handling

### Parsing & Compilation

- `oxc_parser` / `oxc_ast`
- JavaScript/TypeScript parsing (fastest parser)
- `lightningcss`
- CSS parsing
- `syn` / `quote` / `proc-macro2`
- Code generation

### Performance

- `bytemuck` / `zerocopy`
- Zero-copy serialization
- `memmap2`
- Memory-mapped files
- `lz4_flex`
- Fast compression
- `rayon`
- Parallel processing

### Testing

- `proptest`
- Property-based testing
- `criterion`
- Benchmarking

## Shell Environment

- Default Terminal: Git Bash (REQUIRED)
- Command Style: ALWAYS use Unix-style bash commands
- Path Separators: Forward slashes (`/`)
- NO PowerShell, NO CMD commands

## Common Commands

```bash
# Build & Test
cargo build --workspace
cargo build --release
cargo test --workspace
cargo test -p dx-serializer
cargo check --workspace
cargo fmt --all
cargo clippy --workspace
cargo bench -p dx-serializer
cargo build --target wasm32-unknown-unknown -p dx-www-client

# Token Analysis
dx token <file>              # Analyze token count for any file across multiple LLM models
```


## Code Style


- Max line width: 100 characters
- Indentation: 4 spaces
- Edition: 2024
- Imports: Reordered automatically
- Use `use_field_init_shorthand` and `use_try_shorthand`


## Lint Configuration


Workspace-level clippy lints (in `Cargo.toml`): -`undocumented_unsafe_blocks = "warn"` - Require SAFETY comments -`unwrap_used = "warn"` - Discourage `.unwrap()` in production -`expect_used = "warn"` - Discourage `.expect()` in production -`correctness = "deny"` - Deny correctness issues -`perf = "warn"` - Warn on performance issues


## Release Profile


Optimized for minimal binary size: -`opt-level = "z"` - Size optimization -`lto = true` - Link-time optimization -`codegen-units = 1` - Single codegen unit -`panic = "abort"` - No stack unwinding -`strip = true` - Strip symbols
