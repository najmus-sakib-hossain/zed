
# dx-style Architecture

This document describes the architecture of the `dx-style` crate, a binary-first CSS engine with zero-copy parsing and DX Serializer output.

## Overview

dx-style is a high-performance CSS generation engine that transforms utility class names into CSS rules. It features: -Incremental parsing for efficient re-processing of changed content -Auto-grouping for automatic class deduplication -Binary Dawn format for zero-copy CSS loading in production -Pipeline architecture for testable, maintainable code

## Module Structure

@tree:crates/style/src[]

## Data Flow

@flow:TD[]

## Pipeline Phases

The rebuild pipeline is decomposed into distinct phases for testability and maintainability:

### 1. HTML Parser (`html_parser.rs`)

Input: Raw HTML bytes, IncrementalParser instance, capacity hint Output: `ExtractedContent` containing: -`classes`: Set of unique class names -`group_events`: Events for auto-grouping analysis -`content_hash`: Hash for change detection Responsibility: Extract all class names from HTML using SIMD-accelerated parsing. Supports grouping syntax like `@group(class1 class2)`.

### 2. Group Analyzer (`group_analyzer.rs`)

Input: `ExtractedContent`, StyleEngine, RebuildConfig Output: `GroupAnalysis` containing: -`registry`: GroupRegistry with analyzed groups -`processed_classes`: Classes after group processing -`suggested_renames`: Optimization suggestions Responsibility: Analyze class patterns and suggest group optimizations based on similarity thresholds.

### 3. HTML Rewriter (`html_rewriter.rs`)

Input: HTML bytes, `GroupAnalysis`, RebuildConfig Output: `Option<RewritePlan>` containing: -`html`: Rewritten HTML bytes -`groups`: Modified group information -`modified`: Whether changes were made Responsibility: Apply group renames and optimizations to HTML. Returns `None` if no changes needed.

### 4. CSS Generator (`css_generator.rs`)

Input: Class set, GroupRegistry, StyleEngine Output: `GeneratedCss` containing: -`rules`: Vector of `CssRule` (class_name, css) -`total_bytes`: Total CSS size Responsibility: Generate CSS rules for all classes using the style engine and group registry.

### 5. Output Writer (`output_writer.rs`)

Input: `GeneratedCss`, WriteMode (Full or Incremental) Output: CSS bytes, `WriteStats` Responsibility: Prepare CSS content for writing. Supports full rebuild or incremental updates.

## Key Components

### AppState

Runtime state for the CSS generation pipeline:
```rust
pub struct AppState { pub html_hash: u64, // Change detection pub class_cache: AHashSet<String>, pub css_out: CssOutput, pub css_index: AHashMap<String, RuleMeta>, pub group_registry: GroupRegistry, pub incremental_parser: IncrementalParser, // State flags (moved from global statics)
pub base_layer_present: bool, pub properties_layer_present: bool, pub first_log_done: bool, }
```

### RebuildConfig

Typed configuration replacing environment variables:
```rust
pub struct RebuildConfig { pub force_full: bool, pub force_format: bool, pub debug: bool, pub silent: bool, pub disable_incremental: bool, pub group_rename_threshold: f64, pub aggressive_rewrite: bool, pub utility_overlap_threshold: f64, pub cache_dir: String, pub style_bin_path: String, }
```

### StyleEngine

The core engine for generating CSS from utility classes. Lazily initialized via `AppState::engine()`.

### GroupRegistry

Registry for auto-grouped classnames. Manages group definitions and generates CSS for grouped classes.

## Error Handling

All errors flow through `StyleError`:
```rust
pub enum StyleError { InputReadError { path: PathBuf, source: io::Error }, OutputWriteError { path: PathBuf, source: io::Error }, EngineNotInitialized, ParseError { message: String, line: usize, column: usize }, BinaryError(BinaryDawnError), ThemeError(String), InvalidPropertyByte(u8), ConfigError { message: String }, MutexPoisoned, HtmlParseError { message: String }, }
```

## Binary Dawn Format

Zero-copy CSS format for production deployments: -Sub-microsecond style lookups -Binary search for O(log n) access -Compact encoding with varint compression

## Thread Safety

- `AppState` is wrapped in `Arc<Mutex<AppState>>`
- `StyleEngine` uses `OnceLock` for lazy initialization
- `MutexExt` trait provides safe mutex handling with poison recovery

## Configuration

Configuration is loaded from: -`.dx/config.sr` (DX Serializer format, preferred) -`.dx/config.toml` (legacy TOML format) Environment variables are converted to `RebuildConfig` at the CLI entry point only.
