# DX Forge

**Status: Alpha** - Core functionality works, but many features are experimental or unimplemented.

## What It Is

DX Forge is a build orchestration engine with file watching, event handling, and tool coordination. It provides infrastructure for building developer tools that need to:

- Watch files and respond to changes
- Coordinate multiple tools with priority-based execution
- Track generated code and manage file ownership
- Handle debounced and idle events
- Store content-addressably with SQLite

## What It's NOT (Yet)

- ❌ A Git replacement (wraps Git, doesn't replace it)
- ❌ A package manager (stubs only)
- ❌ A CI/CD system (stubs only)
- ❌ A collaborative editing platform (CRDT support is optional)
- ❌ Production-ready for all use cases

## Installation

```toml
[dependencies]
dx-forge = "0.1.0"
```

## Quick Start

```rust
use dx_forge::*;

fn main() -> anyhow::Result<()> {
    // Initialize forge
    initialize_forge()?;
    
    // Register your tool
    struct MyTool;
    impl DxTool for MyTool {
        fn name(&self) -> &str { "my-tool" }
        fn version(&self) -> &str { "1.0.0" }
        fn priority(&self) -> u32 { 50 }
        
        fn execute(&mut self, ctx: &ExecutionContext) -> anyhow::Result<ToolOutput> {
            // Your tool logic here
            Ok(ToolOutput::success())
        }
    }
    
    register_tool(Box::new(MyTool))?;
    execute_pipeline("default")?;
    shutdown_forge()?;
    
    Ok(())
}
```

## Core Features (Implemented)

### Tool Orchestration
- Priority-based execution
- Dependency resolution
- Tool registry with versioning

### File Watching
- Dual-watcher architecture (LSP + filesystem)
- Debounced event handling
- Pattern detection for DX tools

### Event System
- Realtime, debounced, and idle execution paths
- Global event bus
- Subscription-based notifications

### Storage
- Content-addressable blob storage
- SQLite backend
- Snapshot management

### Platform I/O
- Native backends: io_uring (Linux), kqueue (macOS), IOCP (Windows)
- Automatic fallback to tokio
- Batch operations

## Features (Partial/Experimental)

### Traffic Branch System
- Basic conflict detection
- Green/Yellow/Red safety levels
- Needs more testing

### Code Governance
- Track generated files
- File ownership claims
- Basic implementation only

### Configuration
- Config validation
- Template support (basic)
- Injection APIs are stubs

## Features (Not Implemented)

- R2 cloud sync
- Package registry search/install
- CI/CD pipeline triggers
- AI-powered suggestions
- Cart-based discovery
- Most "DX Experience" APIs

## API Status

Out of the claimed "132 functions":
- ~40 are fully implemented and tested
- ~30 are partially implemented
- ~60 are stubs or unimplemented

See `docs/API_IMPLEMENTATION_STATUS.md` for details.

## Architecture

```
forge/
├── core/          # Main Forge struct, lifecycle
├── orchestrator/  # Tool execution engine
├── watcher/       # File watching (dual-watcher)
├── storage/       # Content-addressable storage
├── version/       # Version management
├── daemon/        # Background service (optional)
├── server/        # HTTP/WebSocket server (optional)
└── platform_io/   # Native I/O backends
```

## Platform-Native I/O

```rust
use dx_forge::{create_platform_io, PlatformIO};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let io = create_platform_io();
    println!("Using: {}", io.backend_name());
    
    io.write_all("file.txt".as_ref(), b"Hello").await?;
    let content = io.read_all("file.txt".as_ref()).await?;
    
    Ok(())
}
```

## Configuration Validation

```rust
use dx_forge::{ConfigValidator, ForgeConfig};

let config = ForgeConfig::new(".");
let result = ConfigValidator::validate(&config);

if !result.is_valid() {
    for error in result.errors().unwrap() {
        eprintln!("{}: {}", error.field, error.message);
    }
}
```

## Known Issues

- Large files (6 files > 1000 lines) need refactoring
- 54 dependencies (could be reduced with feature flags)
- `watcher_legacy` module should be removed
- Many APIs are stubs
- Documentation overpromises

## Testing

```bash
cargo test --lib              # Run library tests (345 tests)
cargo clippy --lib            # Check for issues (0 warnings)
cargo run --example simple    # Run basic example
```

## Contributing

This is alpha software. Contributions welcome, but expect breaking changes.

See `CONTRIBUTING.md` for guidelines.

## License

Licensed under MIT OR Apache-2.0 (your choice).

## Roadmap

**v0.2.0** (Next)
- Remove unimplemented APIs or mark clearly
- Refactor large files
- Reduce dependencies
- Better documentation

**v1.0.0** (Future)
- Stable API
- Full test coverage
- Production-ready for build orchestration use case
- Clear scope (not trying to be everything)

## Honest Assessment

This crate has solid foundations (orchestration, file watching, storage) but suffers from scope creep. It tries to be a VCS, LSP server, package manager, and CI/CD system simultaneously. 

**Use it for**: Build tool orchestration, file watching, event coordination
**Don't use it for**: Git replacement, package management, production deployments (yet)

The code quality is good, but the product vision needs focus.
