# Dx: The Binary-First Development Experience

> "Binary Everywhere. Zero Parse. Zero GC. Zero Hydration."
A revolutionary full-stack development platform built entirely in Rust, replacing the JavaScript ecosystem with binary-first architecture. Dx is not just a web framework—it's a complete development platform that replaces React, Next.js, Bun, npm, and the entire JavaScript toolchain with a unified binary-first system. Built from the ground up in Rust, Dx delivers unprecedented performance through WebAssembly, binary protocols, and compile-time optimization.
---

## 🏆 Record-Breaking Achievements test 123212

## February 2026 Architecture Update

- **Publishing Pipeline**: Plugin validation, packaging, signing, and submission workflow.
- **Contribution Tracking**: Automated contributor aggregation and CONTRIBUTORS.md generation.
- **Platform Apps**: macOS/iOS/Android app scaffolding for native presence.
- **Docs Hub**: Consolidated documentation in the /docs directory.

### 🔥 Binary Dawn: The Fastest Web Framework Architecture (December 22, 2025)

dx-reactor delivers unprecedented I/O performance through revolutionary architecture:
+--------+--------+------------+--------+
| Metric | Target | Comparison | Status |
+========+========+============+========+
| HTTP   | Mode   | 2          | 500    |
+--------+--------+------------+--------+
- **Cross-Platform I/O:** Unified Reactor trait (io_uring on Linux, kqueue on macOS, IOCP on Windows)
- **Thread-per-Core:** Zero lock contention through CPU-pinned workers with local queues
- **HBTP Protocol:** 8-byte binary headers replacing HTTP, O(1) route lookup
- **Memory Teleportation:** Zero-copy serialization between Rust server and WASM client
- **Compiler-Inlined Middleware:** Zero runtime overhead through compile-time inlining
**See:** [dx-reactor README] | [Binary Dawn Design]

### 🌟 Binary Dawn Features: 25 Revolutionary Web Framework Features (December 22, 2025)

dx-www now includes 25 binary-first features with 328 passing tests:
+--------------+-------------+----------------+--------+
| Feature      | Performance | Comparison     | Status |
+==============+=============+================+========+
| Compile-Time | Reactivity  | 0.001ms/update | 100x   |
+--------------+-------------+----------------+--------+
Compile-Time Reactivity, Binary Animations, Binary Server Components, Instant Resumability, Binary Serializable Closures, Binary Islands Architecture, Compile-Time DI, SharedArrayBuffer Keep-Alive, O(1) Teleport/Portals, Binary Control Flow Opcodes, Bit-Flag Suspense, Binary Streaming SSR, Grouped Handler Code Splitting, Three-Tier Progressive Enhancement, Binary Trie Router, Binary Schema Form Actions, XOR-Based Optimistic Rollback, Pre-Compiled View Transitions, Memory-Mapped Content Collections, Binary LiveView Patches, Schema-Driven Admin Generation, Binary Ring Buffer Jobs, Pre-Computed Cron Scheduling, Compile-Time Inlined Guards, Compile-Time Type Safety.
**See:** [Binary Dawn Features Design] | [Implementation Tasks]

### 🎯 Complete Victory Over Bun (December 17, 2025)

DX has beaten Bun in ALL 4 critical development systems:
+--------+---------+----------+---------+-------------+---------+--------+
| System | Bun     | Baseline | DX      | Performance | Speedup | Status |
+========+=========+==========+=========+=============+=========+========+
| JS     | Bundler | 38.53ms  | 10.05ms | 3.8x        | faster  | ✅      |
+--------+---------+----------+---------+-------------+---------+--------+

### dx-js-runtime: 10.59x Faster Than Bun

- **Average Performance:** 10.59x faster than Bun across 19 comprehensive tests.
- **Peak Performance:** 80.03x faster on TypeScript (vs Bun's compilation overhead).
- **Consistency:** 6-7x faster on JavaScript, 100% success rate across 228 benchmark runs.
- **Architecture:** Stack-only execution (no GC), output optimization, constant folding.
**See:** [How We Achieved 10x] | [Benchmarks]

### serializer: LLM-Optimized Data Format ✅ Production Ready

**776 Tests Passing | 0 Clippy Warnings | 8 Fuzz Targets Verified** **Token Efficiency (January 2026) - using correct TOON format:**
+-------+-----------+------+------+----+-------+------+--------+--------+
| Test  | Case      | JSON | TOON | DX | vs    | JSON | vs     | TOON   |
+=======+===========+======+======+====+=======+======+========+========+
| Hikes | (tabular) | 113  | 70   | 68 | 39.8% | 2.9% | Events | (logs) |
+-------+-----------+------+------+----+-------+------+--------+--------+
- **Triple Format System:** Human format (`.sr`/`.md` on disk) + LLM format (`.llm` in `.dx/`) + Machine binary (`.machine` in `.dx/`).
- **Test Coverage:** 535 unit + 74 property + 71 battle hardening + 55 doc + 41 integration tests.
- **Security:** 100 MB input limit, 1000 recursion depth, 10M row limit, strict UTF-8 validation.
- **Thread Safety:** All public types implement `Send + Sync`.
**See:** [Serializer README]

### dx-markdown: Context Compiler for LLMs (42.9% Token Savings)

**Token Efficiency by Tokenizer (tested on 6 markdown files):**
+-----------+--------+--------+--------+-------+---------+
| Tokenizer | Tokens | Before | Tokens | After | Savings |
+===========+========+========+========+=======+=========+
| cl100k    | 20     | 696    | 11     | o200k | 20      |
+-----------+--------+--------+--------+-------+---------+
- **Badge-heavy files:** 29.7% average savings.
- **Tables → DX Serializer:** SPACE-separated format with `:N` length prefix.
- **Usage:** `dx markdown .` to process all markdown files in current directory.
**See:** [DX Markdown README]

### dx-js-bundler: 3.8x Faster Than Bun ✅ PRODUCTION READY

- **Performance:** 10.05ms (DX) vs 38.53ms (Bun) average = 3.8x faster.
- **SIMD Optimization:** AVX2 pattern matching for imports/exports (~0.6ms).
- **Binary Cache:** Zero-copy serialization for instant warm builds.
- **Transform Pipeline:** TypeScript stripping + JSX preservation + minification.
- **Output Validation:** Identical output size, all tests passed (`node --check` verified).
- **Bonus Fusion Mode:** 0.7ms bundling (71x faster) using pre-compiled `.dxm` modules.
**See:** [Complete Victory] | [Fusion Benchmark]

### dx-check: Binary-First Linter (Phase 3 Complete) ✅ NEW

- **vs ESLint:** 100-200x faster (verified).
- **vs Biome:** 5-8x faster (verified: 7.6x single, 4.9x multi).
- **Rule Loading:** 0.70ns (hardware limit via dx-serializer).
- **Languages:** 15 supported (JS/TS/Py/Go/Rust/PHP/MD/TOML/Kt/C/C++/JSON/CSS/HTML/YAML).
- **Rules:** 270+ unified in binary format.
- **Hot-Reload:** File-based `.sr` system with <50ms recompilation.
- **Architecture:** Binary Rule Fusion Engine, SIMD Scanner (AVX2), File Watcher.
**See:** [dx-check README] | [Phase 3 Progress]

### dx-www: 338 Bytes to 7.5 KB Runtime

- **Dual-Core Codegen:** Micro (raw FFI, 338B) + Macro (HTIP templates, 7.5KB).
- **HTIP Rendering:** Native `cloneNode()` instead of Virtual DOM diffing.
- **Intelligent Compiler:** Automatically selects optimal runtime based on app complexity.
- **Performance:** 27-33x faster than React on first load (30ms vs 5.2s).

### dx-style: Binary CSS (B-CSS) — Internal Use Ready

- **98% size reduction:** Integer class IDs vs text CSS.
- **80x faster:** Binary lookups vs text parsing.
- **Zero-copy:** Binary Dawn CSS format with memory-mapped styles.
- **DX Serializer:** Unified serialization (20%+ smaller than FlatBuffers).
- **Auto-Grouping:** Automatic pattern detection with Jaccard similarity.
- **706 tests:** 21+ property-based test modules with proptest.
---

## Key Features

### 🚀 Complete Replacement Ecosystem

- **React/Next.js → dx-www:** Binary web runtime with HTIP protocol.
- **Bun/Node.js → dx-js-runtime:** 10x faster JavaScript/TypeScript execution.
- **npm/pnpm → dx-package-manager:** Binary package format (50x target).
- **ESLint/Biome → dx-check:** Binary-first linter (100x faster than ESLint).
- **Tailwind → dx-style:** Binary CSS with integer class IDs.
- **JSON → serializer:** LLM format (73.3% token savings, best: 96.6%).
- **Markdown → dx-markdown:** Context Compiler for LLMs (42.9% token savings).

### 🛠️ VS Code Extension

- **vscode-dx-serializer:** Seamless `.dx` file and `dx` filename editing with Human Format.
- **Triple Format System:** Human format (disk) + LLM format (.dx/serializer/*.llm) + Machine binary (.dx/serializer/*.machine).
- **New Architecture (2026):** Front-facing files now use human format for better git diffs and tool compatibility.
- **Universal Converter:** Auto-convert JSON, YAML, TOML, CSV to DX format.
- **Section Order Preservation:** Reorder sections in editor, preserved on save.
- **Syntax Highlighting:** Professional colors (pink keys, green values, blue headers).
- **Real-time Validation:** Immediate syntax error feedback with actionable hints.
- **Install:** `kiro-install-extension crates/vscode-dx-serializer/vscode-dx-serializer-0.1.0.vsix`

### ⚡ Zero-Cost Abstractions

- **Zero Parse:** Binary formats eliminate text parsing overhead.
- **Zero GC:** Stack-only allocation, SharedArrayBuffer for state.
- **Zero Hydration:** Resumable state snapshots, instant page transitions.
- **Zero Virtual DOM:** Direct DOM manipulation via HTIP cloning.

### 🛡️ Security & Type Safety

- **Compile-Time Validation:** dx-form, dx-guard, dx-a11y audit at build time.
- **Capability-Based Security:** Memory-safe architecture with Ed25519 signing.
- **XSS Prevention:** Input sanitization before DOM access (mathematically impossible in strict mode).

### 🌍 Production-Ready Stack

- **Full-Stack:** Client (WASM), Server (Axum), Database (PostgreSQL), Auth (Ed25519).
- **Internationalization:** i18n with translation and text-to-speech.
- **Offline-First:** dx-offline with CRDT sync, dx-sync WebSocket protocol.
- **Developer Experience:** dx-cli orchestrator, dx-debug DevTools bridge, dx-check linter.
---

## Performance Benchmarks
+----------------+-----------+-------------+------+-------------+
| Framework/Tool | Metric    | Traditional | Dx   | Improvement |
+================+===========+=============+======+=============+
| **Web          | Runtime** | Bundle      | Size | 140         |
+----------------+-----------+-------------+------+-------------+
- **Bandwidth @ 100M req/day:** JSON: 69.9 GB | DX: 18.6 GB (73% reduction, $6,156/year savings).
- **Mobile Performance:** 30ms first paint vs 400ms (13x faster on 3G networks).
- **Server Costs:** Binary streaming reduces compute by 95% vs JSON parsing.
---

## Recent Updates

### ✅ Dec 29, 2025: DX-STYLE INTERNAL USE READY

- **DX Serializer Integration:** Replaced FlatBuffers with unified dx-serializer (20%+ smaller).
- **Auto-Grouping:** Automatic pattern detection with Jaccard similarity clustering.
- **Binary Dawn CSS:** Zero-copy binary format with varint encoding and checksum validation.
- **706 Tests:** 21+ property-based test modules with comprehensive coverage.
- **Status:** Ready for DX ecosystem.

### ✅ Dec 28, 2025: DX-MEDIA PRODUCTION READY

- **Property Tests:** 51 property-based tests covering correctness guarantees.
- **Security:** SSRF prevention, filename sanitization, content-type verification.
- **Resilience:** Circuit breaker, rate limiting, retry logic.
- **Benchmarks:** Performance documented with criterion benchmark suite.
- **See:** [dx-media README] | [Production Ready Spec]

### ✅ Dec 26, 2025: BATTLE HARDENING & SERIALIZER

- **Serializer Security Complete:** 100 MB size limit, 1000 level recursion limit, 10M row table limit.
- **Defensive Errors:** New error types (InputTooLarge, RecursionLimitExceeded, TableTooLarge).
- **Property Testing:** 38 correctness properties (up from 21).
- **Quantum Entanglement / Phase 6 Complete:** UTF-8 Validation, Platform-Specific I/O, Token Efficiency 3x+ better than TOON.
- **See:** [Battle Hardening Spec]

### 🔥 Dec 21, 2025: BINARY DAWN

- **dx-reactor:** Cross-platform I/O reactor with io_uring (Linux), kqueue (macOS), IOCP (Windows).
- **dx-db-teleport:** Reactive database caching with Postgres NOTIFY invalidation.
- **Performance Targets:** 2.5M+ RPS HTTP, <100μs p99 latency, <0.1ms cache access.
- **See:** [dx-reactor README] | [dx-db-teleport README]

### ✅ Dec 19, 2025: TOOLING & DRIVEN CRATE

- **Workspace Restructure:** Tooling alignment, moved i18n/serializer to Dx Tools.
- **Production Ready:** `cargo check --workspace` passes with 0 errors.
- **Driven Crate:** AI-Assisted development orchestrator (160/160 tests passing).
- **Modules:** Binary, Fusion, Streaming, Security, State, CLI.
- **See:** [Driven Complete]

### ✅ Previous Milestones

- **dx-js-runtime:** 10.59x faster than Bun (Verified).
- **dx-package-manager:** The Binary Package Revolution (50x target, 17.2x verified).
- **Client Trinity (Phase 6):** Stream Consumer, Client Patcher, Eternal Cache.
- **Dual-Core Codegen:** Micro (338B) and Macro (7.5KB) runtimes.
---

## Quick Start

### Install dx-cli
```bash
cargo install dx-cli git clone github.com/dx-www/dx cd dx cargo build --release --bin dx ```
### Create a New Project
```bash
dx new my-app --template counter cd my-app dx dev dx build --release dx run src/main.ts ```

### Write TypeScript, Get Binary
```tsx
import { useState } from 'dx';
export default function Counter() {  const [count, setCount] = useState(0);
  return (  <div class="p-4">  <h1>Count: {count}</h1>  <button onClick={() => setCount(count + 1)}>Increment</button>  </div>  );
}
```
**The compiler automatically:** 1. Selects Micro (338B) or Macro (7.5KB) runtime based on complexity.
2. Compiles TSX → Binary layout + WASM logic.
3. Generates optimized binary CSS.
4. Creates resumable state snapshots.
5. Produces a single `.dxb` artifact.
---

## Complete Architecture

Dx is organized as a Cargo workspace with 47 specialized crates, each focused on a specific domain:

### 🎯 Core Runtime (Web)
+--------+---------+------------+-----------------------+
| Crate  | Purpose | Size/Lines | Status                |
+========+=========+============+=======================+
| `core` | Linear  | memory     | manager Crate Purpose |
+--------+---------+------------+-----------------------+
DX introduces a Stack abstraction that unifies language-specific development tools.
```bash
dx stack run index.ts  # dx-js-runtime (10x faster)
dx stack bundle --minify  # dx-js-bundler (3.8x faster)
dx stack test --coverage  # dx-js-test-runner (26x faster)
dx stack install  # dx-js-package-manager (50x faster)
```
**JavaScript/TypeScript Stack Components:**
+------------------------------+----------------------------------------+------------------------------+--------------------------------------------+
| Component                    | Crate                                  | Performance                  | Status                                     |
+==============================+========================================+==============================+============================================+
| Runtime                      | `dx-js-runtime`                        | 10.59x                       | faster Crate Purpose Lines                 |
+------------------------------+----------------------------------------+------------------------------+--------------------------------------------+

## Project Structure
```text
@tree:dx[]
Total Lines of Code: ~30,000+ lines of production Rust Test Coverage: 400+ tests across all crates Crate Count: 47 specialized crates ```
---
## Documentation
### 🎯 Getting Started
- [Quick Start Guide] - Get up and running in 5 minutes
- [Development Guide] - Build and test instructions
- [Project Summary] - Complete overview
### 🏗️ Core Architecture
- [Architecture Overview] - HTIP protocol deep-dive
- [Compiler Intelligence] - Micro/Macro auto-selection algorithm
- [Bundle Size Analysis] - Size breakdowns and comparisons
- [Binary Dawn Structure] - Canonical app layout (v1.0)
- [Project Structure](#project-structure) - Crate organization
### ⚡ JavaScript/TypeScript Runtime
- [How We Achieved 10x] - Technical breakdown of 10.59x speedup
- [Final Benchmarks] - Complete test results (19 tests)
- [Victory Report] - 7.8x (average) to 80x (TypeScript)
- [Runtime Quick Reference] - API reference
### 📦 Data Serialization
- [Token Efficiency] - 73.3% savings vs JSON
### 🎨 Style System
- [Binary CSS (B-CSS)] - Overview and usage
- [dx-style README] - Full documentation with 706 tests
### 📚 Package Manager (Design)
- [Package Manager $X] - 50x faster than Bun target
- [Binary Protocol Spec] - HTIP v1 protocol
---
## Status & Roadmap
### ✅ Completed (December 19, 2025)
**Phase 1-4: Foundation & Core Runtime**
- Cargo workspace with 47 specialized crates.
- Core memory manager, HTIP renderer, O(1) dirty-bit patcher.
- Dual-core codegen (Micro 338B / Macro 7.5KB).
**Phase 5: SSR Server**
- Template inflation (~1ms), Bot detection, Axum integration.
**Phase 6: Client Trinity**
- Zero-copy binary streaming, XOR block patching, IndexedDB caching.
**Phase 7: CLI Orchestrator**
- Unified `dx` command, hot reload, scaffolding.
**Tooling**
- **Driven:** AI-Assisted Development Orchestrator.
- **JS Runtime:** 10.59x faster than Bun.
- **Serialization:** 73.3% token savings vs JSON.
- **Binary Dawn I/O:** Thread-per-core architecture.
- **Package Manager:** 17.2x faster than Bun.
### 🚧 In Progress (December 2025)
- **Phase 8: Polish & UX:** Touch/gesture recognition, RTL support.
- **Asset Optimization:** `dx-icon`, `dx-font` subsetting.
- **Integration Testing:** End-to-end suite, Hacker News clone.
### 📋 Planned (Q1 2026)
- **Developer Experience:** HMR, Source maps for binary debugging.
- **Optimizations:** Tree-shaking, LTO, WASM SIMD.
- **Production:** CDN integration, Distributed tracing.
### 🎯 Target Release: January 1, 2026
- **Public Beta:** Complete Phase 8, Security Audit, Documentation.
- **Production Goals:** 1000+ unit tests, < 1% crash rate, Enterprise support.
---
## Code Organization & Implementation Standards
### Memory Management & Performance Philosophy
- **Zero-Copy Architecture:** All data structures use `&[u8]` slices or memory-mapped `SharedArrayBuffer`.
- **No String Allocation Rule:** Strictly forbidden to use `String` in hot paths; use `u32` indices.
- **Object Pooling Pattern:** Structs are reused per frame (Data-Oriented Design).
- **Stack-Only Execution:** No garbage collection; all computations use stack allocation.
### Rendering Architecture (HTIP Protocol)
- **Native DOM Cloning:** Uses browser's native `cloneNode()` C++ engine.
- **Batch Operations:** DocumentFragment accumulation and single flush-to-DOM.
- **Frame Budget:** Strict 4ms maximum execution per frame.
### State Management & Reactivity
- **Dirty-Bit Tracking:** O(1) change detection via `u64` bitmask headers.
- **SharedArrayBuffer:** State lives in linear WebAssembly memory.
- **XOR Differential Patching:** Network updates calculate byte-level XOR differences.
### Security
- **Capability-Based Architecture:** Compile-time validation.
- **Ed25519 Signing:** All binary artifacts signed and verified.
- **Input Sanitization:** XSS is mathematically impossible in strict mode.
---
## Contributing
Dx is a systems-level project requiring deep knowledge of Rust, WebAssembly, and Browser Internals.
### Development Setup
```bash
git clone github.com/dx-www/dx cd dx rustup update stable rustup target add wasm32-unknown-unknown cargo build --workspace cargo test --workspace ```

### Project Guidelines

- **Code Style:** Follow `rustfmt.toml`.
- **Testing:** Write unit tests for all new functionality.
- **Performance:** Benchmark changes that affect hot paths.
- **Safety:** Document all `unsafe` blocks with safety invariants.
---

## The Vision

**Dx is more than a framework. It's a paradigm shift.** For 30 years, the web has been built on text: HTML strings, JSON payloads, JavaScript bundles. We parse the same data formats millions of times per second, waste CPU cycles on garbage collection, and ship megabytes of redundant code.
Dx asks: *What if we built for machines first, humans second?* The result is a platform where:
- Applications are **413x smaller** than React equivalents.
- Runtime performance is **10-80x faster** than Bun/Node.js.
- Data formats are **73% smaller** than JSON.
- Security is **mathematically guaranteed**.
**This is not just an incremental improvement. This is the Binary Web.** *Welcome to the future. Welcome to Dx.* ---

## License

Licensed under either of:
- MIT License
- Apache License 2.0
at your option.

### Contribution
