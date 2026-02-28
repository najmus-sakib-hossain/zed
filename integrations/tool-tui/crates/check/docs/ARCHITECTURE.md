
# DX-Check Architecture

## Overview

DX-Check is a high-performance, multi-language code linter and formatter built in Rust. It uses binary rule serialization and SIMD-accelerated pattern scanning to achieve exceptional performance.

## Architecture Diagram

@tree[]

## Core Components

### 1. CLI Layer (`src/cli.rs`, `src/main.rs`)

The CLI layer provides the user interface for dx-check. It uses `clap` for argument parsing and supports multiple commands: -`check` - Run both formatting checks and lint rules -`lint` - Run lint rules only -`format` - Format files -`watch` - Watch for file changes and recompile rules -`lsp` - Start the Language Server Protocol server -`ci` - Generate CI configuration files

### 2. Engine Layer (`src/engine.rs`)

The `CheckerEngine` is the core orchestrator that: -Discovers files to process using the `ignore` crate -Parses source files using language-specific parsers -Executes lint rules in parallel using `rayon` -Collects and reports diagnostics -Applies auto-fixes when requested ```rust pub struct Checker {config:CheckerConfig,registry:RuleRegistry,cache:Option<AstCache>,}
```


### 3. Rule System (`src/rules/`)



#### Rule Registry (`src/rules/registry.rs`)


Manages all available lint rules and their configurations:
```rust
pub struct RuleRegistry {rules_by_id:HashMap<u16,Box<dyn Rule>>,rules_by_name:HashMap<String,u16>,enabled:HashMap<u16,Severity>,}
```


#### Rule Trait (`src/rules/mod.rs`)


All rules implement the `Rule` trait:
```rust
pub trait Rule:Send +Sync {fn meta(&self)->RuleMeta;fn check(&self,ctx:&mut RuleContext<'_>);
fn check_end(&self, _ctx: &mut RuleContext<'_>){}}
```


#### Built-in Rules (`src/rules/builtin/`)


Organized by category: -Correctness: `no-empty`, `no-duplicate-keys`, `no-unreachable` -Style: `consistent-quotes`, `indent`, `max-line-length` -Security: `no-eval`, `no-alert` -Performance: `no-console`, `no-debugger`, `prefer-const`


### 4. Language Handlers (`src/languages/`)


Each supported language has a dedicated handler implementing the `LanguageHandler` trait:
```rust
pub trait LanguageHandler:Send +Sync {fn name(&self)->&'static str;
fn extensions(&self) -> &[&'static str];fn format(&self,path:&Path,content:&str,write:bool)->Result<FileStatus,LangDiagnostic>;fn lint(&self,path:&Path,content:&str)->Result<Vec<LangDiagnostic>,LangDiagnostic>;}
```
Supported languages: -JavaScript/TypeScript: Uses `oxc_parser` for parsing -Python: Uses `rustpython-parser` -Go: Delegates to `gofmt` -Rust: Delegates to `rustfmt` -Markdown: Custom linting rules -TOML: Custom parser and formatter


### 5. AST Cache (`src/cache.rs`)


Caches parsed ASTs to avoid re-parsing unchanged files:
```rust
pub struct AstCache {cache_dir:PathBuf,index:RwLock<HashMap<String,CacheEntry>>,max_size:u64,}
```
Features: -Blake3 content hashing for cache keys -LRU eviction when cache exceeds max size -Persistent index stored on disk


### 6. Plugin System (`src/plugin.rs`)


Extensible plugin architecture supporting: -Native plugins: Rust shared libraries (`.dll`, `.so`, `.dylib`) -WASM plugins: Sandboxed WebAssembly modules -JavaScript plugins: Via dx-js-runtime (planned)
```rust
pub trait Plugin:Send +Sync {fn meta(&self)->PluginMeta;fn rules(&self)->Vec<Box<dyn Rule>>;}
```


### 7. LSP Server (`src/lsp/`)


Language Server Protocol implementation for IDE integration: -Real-time diagnostics on file open/change -Code actions for quick fixes -Hover information for rule documentation -Configuration file watching


### 8. Pattern Scanner (`src/scanner.rs`)


SIMD-accelerated pattern scanning for fast string matching:
```rust
pub struct PatternScanner {patterns:Vec<CompiledPattern>,}
```
Uses AVX2/NEON intrinsics when available for 10x+ speedup.


## Data Flow



### Check Command Flow


@tree:Input: dx-check check src[]


## Configuration


DX-Check uses `dx.toml` for configuration:
```toml
[rules]
recommended = true auto_fix = false
[format]
use_tabs = false indent_width = 2 line_width = 80
[cache]
enabled = true directory = ".dx-cache"
[parallel]
threads = 0 ```

## Performance Optimizations

- Parallel Processing: Uses `rayon` for parallel file processing
- AST Caching: Avoids re-parsing unchanged files
- SIMD Scanning: Uses AVX2/NEON for pattern matching
- Binary Rule Format: Fast rule loading via `dx-serializer`
- Incremental Checking: Only processes changed files

## Module Structure

@tree:src[]

## Security Considerations

- WASM Sandboxing: WASM plugins run in isolated environments
- File Access: Respects `.gitignore` and configured ignore patterns
- Safe Defaults: Conservative default configuration
- Input Validation: All user input is validated before processing
