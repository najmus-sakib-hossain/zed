
# Using dx-forge as a library in your DX tools

This document explains how to use the `dx-forge` crate from other DX tools. It covers adding the crate as a dependency, the most useful APIs (orchestrator, watchers, storage), examples for common integration patterns, and build tips for development. Target audience: authors of DX tools (plugins, CLIs, and integrations) who want to reuse Forge's orchestration, detection, and storage pieces without running the CLI binary.

## Quick summary

- Crate name: `dx-forge` (see `Cargo.toml` at the repository root).
- Rust Edition: 2021, minimum toolchain: 1.70.
- No special cargo features are required to use the core APIs. You can depend on `dx-forge` from crates.io (if published), a Git repository, or locally during development.

## Add as a dependency

Choose one option depending on whether you depend on a released crate or a local copy: -crates.io (if published):
```toml
[dependencies]
dx-forge = "0.0.2"
```
- Git dependency (useful for tracking `main` or a branch/tag):
```toml
[dependencies]
dx-forge = { git = "https://github.com/najmus-sakib-hossain/forge", branch = "main" }
```
- Local path dependency (recommended while hacking locally):
```toml
[dependencies]
dx-forge = { path = "../forge" }
```
If you include `dx-forge` in a workspace, prefer the `path` or workspace member approach so versions stay in sync.

## What you get (high-level API)

`dx-forge` exposes a number of modules and high-level types designed to be reused by DX tools. The main entry points you will likely need: -Orchestration -`Orchestrator`, `DxTool`, `ExecutionContext`, `ToolOutput`, `OrchestratorConfig` -Change detection -`DualWatcher`, `FileWatcher`, `LspWatcher`, `FileChange`, `ChangeKind`, `ChangeSource` -Storage and persistence -`Database`, `OperationLog`, blob helpers, `time_travel` utilities -CRDT primitives -`Operation`, `OperationType`, `Position` -Patterns, injection and tooling helpers -`PatternDetector`, `InjectionManager`, `ToolRegistry`, `ToolInfo` These types are re-exported from the crate root so you can import them like `use dx_forge::Orchestrator;`.

## Example 1 — Implement a small DX tool and register with the Orchestrator

This is the canonical way to build a small dx-tool that cooperates with Forge's orchestrator.
```rust
use anyhow::Result;
use dx_forge::{Orchestrator, DxTool, ExecutionContext, ToolOutput};
struct MyDxTool;
impl DxTool for MyDxTool { fn name(&self) -> &str { "dx-mytool" }
fn version(&self) -> &str { "0.1.0" }
fn priority(&self) -> u32 { 50 }
fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> { // Inspect changed files from the execution context for f in &ctx.changed_files { println!("Tool sees changed file: {}", f.display());
}
// Do work (formatting, injection, codegen, etc.)
Ok(ToolOutput::success())
}
}
fn main() -> Result<()> { // Create orchestrator for the repository root let mut orch = Orchestrator::new(".")?;
// Register your tool orch.register_tool(Box::new(MyDxTool))?;
// Execute all registered tools let outputs = orch.execute_all()?;
println!("Executed {} tools", outputs.len());
Ok(())
}
```
Notes: -`Orchestrator::new` takes a repository root (path where `.dx/forge` will live). -Tools are executed synchronously by default in priority order; `OrchestratorConfig` can be used to tune parallelism.

## Example 2 — Subscribe to file changes with `DualWatcher` (recommended)

Use the `DualWatcher` when you want your tool to react to live edits (LSP preferred) and fallback to file system events.
```rust
use anyhow::Result;
use dx_forge::{DualWatcher, ChangeKind};


#[tokio::main]


async fn main() -> Result<()> { // Create a watcher let mut watcher = DualWatcher::new()?;
// Start watching the repository root (async)
watcher.start(".").await?;
// Subscribe to change events let mut rx = watcher.receiver();
// Loop and handle changes loop { match rx.recv().await { Ok(change) => { match change.kind { ChangeKind::Created => println!("Created: {}", change.path.display()), ChangeKind::Modified => println!("Modified: {}", change.path.display()), ChangeKind::Deleted => println!("Deleted: {}", change.path.display()), _ => {}
}
}
Err(_) => break, }
}
Ok(())
}
```
Use `DualWatcher::analyze_patterns(change)` to run the built-in `PatternDetector` against a change (it reads content when needed).

## Caveats & implementation notes

- LSP integration: The `LspWatcher` in this repository contains a lightweight scaffold and example flow (it logs that it is running in "mock mode"). A production integration requires wiring the LSP JSON-RPC stream (or editor integration) into the `LspWatcher::process_lsp_event` hook so that real `textDocument/didChange` events are delivered.
- Async vs sync: `DualWatcher` and many watcher-related APIs are asynchronous and expect a Tokio runtime (`#[tokio::main]` or equivalent). The `Orchestrator` and `DxTool` trait are synchronous in the current implementation — if your tool needs async work, run a Tokio runtime inside your tool or adapt the orchestrator to execute async tasks.
- Optional pieces: If you only need pattern detection or injection (for example, in a small in-editor helper), you can depend only on `PatternDetector` and `InjectionManager` (they are exported by the crate) to keep your dependency footprint smaller.

## Example 3 — Persist operations with `OperationLog`

Forge provides an `OperationLog` backed by a `Database` (SQLite). Use it to persist operation-level edits.
```rust
use anyhow::Result;
use std::sync::Arc;
use dx_forge::storage::Database;
use dx_forge::storage::OperationLog;
use dx_forge::crdt::{Operation, OperationType};
fn main() -> Result<()> { // Create/open the database under the forge path let forge_path = std::path::Path::new(".dx/forge");
let db = Database::new(forge_path)?;
db.initialize()?;
// Create an OperationLog writer let oplog = OperationLog::new(Arc::new(db));
// Build an operation (example: file create)
let op = Operation::new( "src/example.rs".to_string(), OperationType::FileCreate { content: "fn main() {}".to_string() }, "actor-1".to_string(), );
// Append (async persistence happens in a background thread)
let appended = oplog.append(op)?;
println!("operation appended: {}", appended);
Ok(())
}
```
Notes: -`Database::new` expects the `forge_path` (e.g. `.dx/forge`). Call `initialize()` once to create tables. -`OperationLog::append` enqueues operations for background persistence and returns whether the op was new.

## Logging and debug

Forge uses `tracing`/`tracing-subscriber`. To see debug output from forge components, set the `RUST_LOG` environment variable and initialize a subscriber in your binary (or rely on the crate's own initialization when using the CLI). Example (run-time):
```bash
RUST_LOG=dx_forge=debug cargo run --bin your-tool ```
If you embed `dx-forge` in a larger application, initialize a `tracing-subscriber` early in `main`:
```rust
use tracing_subscriber::fmt::layer;
use tracing_subscriber::EnvFilter;
tracing_subscriber::registry()
.with(EnvFilter::from_default_env())
.with(layer())
.init();
```


## API Quick Reference


- `Orchestrator::new(repo_root)` — create an orchestrator for the repo
- `orchestrator.register_tool(Box::new(your_tool))` — register a `DxTool`
- `orchestrator.execute_all()` — run registered tools
- `DualWatcher::new()` / `start(path)` / `receiver()` — watch and subscribe to file changes
- `Database::new(&forge_path)` / `db.initialize()` — initialize storage
- `OperationLog::new(Arc::new(db))` / `oplog.append(op)` — persist operations Refer to the in-code docs and the `src/` module re-exports for a full list of types.


## Build tips & platform notes


- Windows: building the project may require `pkg-config` and Visual C++ Build Tools for native dependencies (libgit2). If you run into `libgit2-sys` or `pkg-config` errors, install `pkg-config` (choco/scoop/MSYS2) and the MSVC toolchain.
- Use local `path` dependencies while developing tools together with `dx-forge` for faster iteration.
- If you only need detection (pattern matching and injection) but not persistence, consider using `PatternDetector` and `InjectionManager` directly to avoid pulling large native dependencies.


## Contributing & support


If you need extra library-level helper functions (for example, utilities to run the orchestrator as a long-running service, or more ergonomic watcher adapters) please open an issue or a PR in the repository. License: MIT OR Apache-2.0
