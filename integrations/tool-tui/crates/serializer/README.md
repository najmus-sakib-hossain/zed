# DX Serializer

Token-optimized serialization format for AI context windows with 52-73% token savings vs JSON and pure RKYV binary format.

### Machine Format Performance

**DX-Machine uses pure RKYV** - identical performance:
- Single serialize: ~48-51ns (RKYV: ~48ns, within 6% variance)
- Batch 100: ~7.5µs (RKYV: ~7.9µs, actually 5% faster)
- Zero-copy deserialization (identical to RKYV)
- Production-ready and battle-tested
**Implementation**: Zero-overhead wrapper with `#[inline(always)]` that compiles to identical machine code as RKYV.

## Three Formats

DX Serializer uses a revolutionary 3-format system:

### Human Format (.sr files on disk)

Beautiful, readable format that developers edit directly:
- TOML/INI-like syntax with aligned `=` at column 28
- **Lives on real disk** where you work (e.g., `dx`, `package.sr`)
- Easy to read, write, and version control
- This is the **source of truth** - you edit these files

### LLM Format (.llm in .dx/serializer/)

Token-optimized format for AI context windows:
- 52-73% token savings vs JSON
- Compact notation with schema headers
- **Auto-generated** in `.dx/serializer/*.llm` folder
- Never edit manually - regenerated from human format

### Machine Format (.machine in .dx/serializer/)

Pure RKYV binary format for maximum performance:
- Zero-copy deserialization
- ~48-51ns serialize time
- **Auto-generated** in `.dx/serializer/*.machine` folder
- Identical to RKYV wire format

**Architecture**: Human format files live on disk. When you save a `dx` file (or any file with DX serializer syntax), the extension automatically generates the `.llm` and `.machine` versions in the `.dx/serializer/` folder. The `.dx/` folder is gitignored as it contains generated files.

**Note**: DX-Machine IS RKYV. We use RKYV's wire format directly with no modifications.

## Usage

```bash
# Human format files live on disk (you edit these)
# Example: dx, package.sr

# When you save a file, the extension auto-generates:
# .dx/serializer/dx.llm       (LLM-optimized, 52-73% token savings)
# .dx/serializer/dx.machine   (binary, zero-copy)

# CLI usage (if needed manually):
dx serializer dx          # Process single file
dx serializer .                  # Process directory recursively
dx serializer src/               # Process specific directory
```

**Workflow**:
1. Edit human format files on disk (e.g., `dx`, `package.sr`)
2. Save the file
3. Extension automatically generates `.llm` and `.machine` in `.dx/serializer/`
4. `.dx/` folder is gitignored (contains generated files)
5. Only commit the human format files

## LLM Format
```
author=essensefromexistence
version=0.0.1
name=dx
description="Orchestrate dont just own your code"
title="Enhanced Developing Experience"
driven(path=@/driven)
editors(default=neovim items=[neovim zed vscode cursor antigravity replit "firebase studio"])
forge(repository="https://dx.vercel.app/essensefromexistence/dx" container=none pipeline=none tasks=none tools=[cli docs examples packages scripts style tests])
dependencies[name version](
dx-package-1 0.0.1
dx-package-2 0.0.1
)
js.dependencies(next=16.0.1 react=19.0.1)
```

### Syntax Features
- **Scalars**: `key=value` (no spaces around `=`)
- **Objects**: `name(key=val key2=val2)` (parentheses, space-separated pairs)
- **Arrays**: `key=[item1 item2 item3]` (square brackets, space-separated)
- **Tables**: `name[col1 col2 col3](row1_val1 row1_val2 row1_val3\nrow2_val1...)` (headers in brackets, rows in parentheses)
- **Strings**: Use quotes `"..."` for multi-word strings
- **Booleans**: `true`/`false`
- **Null**: `null`

### Objects
```dsr
config(host=localhost port=5432 debug=true)
server(url="https://api.example.com" timeout=30)
```

### Arrays
```dsr
tags=[rust performance serialization]
editors=[neovim zed "firebase studio"]
```

### Tables
```dsr
users[id name email](
1 Alice alice@ex.com
2 Bob bob@ex.com
)
```

### Nested Sections
Use dot notation:
```dsr
js.dependencies[react=19.0.1,next=16.0.1]
i18n.locales[path=@/locales,default=en-US]
```

### Complete Example
```dsr
name=dx
version=0.0.1
title="Enhanced Developing Experience"
workspace(paths=[@/www @/backend])
editors(items=[neovim zed vscode] default=neovim)
forge(repository="https://github.com/user/repo" tools=[cli docs tests])
js.dependencies(react=19.0.1 next=16.0.1)
```

## Human Format Example
```
author = essensefromexistence
version = 0.0.1
name = dx
description = Orchestrate dont just own your code
title = Enhanced Developing Experience

[driven]
path = @/driven

[editors]
default = neovim
items:
- neovim
- zed
- vscode
- cursor
- antigravity
- replit
- firebase-studio

[workspace]
paths:
- @/www
- @/backend

[dependencies:1]
name = dx-package-1
version = 0.0.1

[dependencies:2]
name = dx-package-2
version = 0.0.1
```

### Syntax Features
- **Scalars**: `key = value` (spaces around `=` for readability)
- **Sections**: `[section]` headers (TOML/INI-like)
- **Arrays**: `key:` followed by `- item` lines
- **Nested Sections**: `[section.subsection]`
- **Strings**: Use quotes for multi-word strings: `title = "My Title"`
- **Alignment**: Keys padded with spaces for column alignment (typically at column 28)

### Scalars
```dx
key = value
title = "My Title"
```

### Arrays
```dx
key:
- item1
- item2
- item3
```

### Sections
```dx
[section]
key = value

[section.subsection]
key = value
```

### Complete Example
```dx
name = dx
version = 0.0.1
title = "Enhanced Developing Experience"

[workspace]
paths:
- @/www
- @/backend

[editors]
items:
- neovim
- zed
- vscode
default = neovim

[forge]
repository = https://github.com/user/repo
tools:
- cli
- docs
- tests

[js.dependencies]
react = 19.0.1
next = 16.0.1
```

## Format Locations

**Architecture Overview**:

- **Human format** - Lives on **real disk**, you edit these files directly
  - Examples: `dx`, `package.sr`
  - Source of truth, version controlled in git
  - TOML/INI-like syntax with aligned `=` at column 28

- **LLM format** (.llm) - **Auto-generated** in `.dx/serializer/` folder
  - Never edit manually
  - Regenerated automatically when human format changes
  - 52-73% token savings vs JSON

- **Machine format** (.machine) - **Auto-generated** in `.dx/serializer/` folder
  - Binary format (pure RKYV)
  - Zero-copy deserialization
  - ~48-51ns serialize time

The `.dx/` folder is gitignored as it contains generated files. Only commit human format files.

## Machine Format (RKYV)

**DX-Machine IS RKYV** - we use RKYV directly:
- Pure RKYV wire format (no modifications)
- Zero-overhead wrapper with `#[inline(always)]`
- Identical performance: ~48-51ns single, ~7.5µs batch 100
- Zero-copy deserialization
- Production-ready

The machine format is a binary representation using RKYV's archived data structures. It provides the fastest serialization/deserialization with zero-copy access to data.

```rust
use serializer::machine::{serialize, deserialize};
// Serialize (calls rkyv::to_bytes directly)
let bytes = serialize(&data)?;
// Deserialize (calls rkyv::access_unchecked directly)
let archived = unsafe { deserialize::<MyType>(&bytes) };
```

### Key Characteristics
- **Binary Format**: Pure binary data, not human-readable
- **Zero-Copy**: Direct memory access without copying data
- **Performance**: Sub-nanosecond access times
- **Safety**: Uses RKYV's compile-time validation
- **Compatibility**: Identical to RKYV's wire format

## Machine Format Compression

DX Machine format supports optional compression using LZ4 and ZSTD algorithms to reduce wire size while maintaining fast decompression.

### Compression Algorithms

#### LZ4 Compression (Default)
- **Speed**: Extremely fast compression/decompression
- **Ratio**: Good compression for structured data
- **Use Case**: Network transfer, storage where speed is critical
- **Pure Rust**: No C dependencies (`lz4_flex` crate)

#### ZSTD Compression
- **Speed**: Fast compression, very fast decompression
- **Ratio**: Excellent compression ratios (better than LZ4)
- **Use Case**: Maximum size reduction, archival storage
- **Levels**: 1 (fast), 3 (balanced), 19 (maximum compression)

### Usage

```rust
use serializer::machine::compress::{DxCompressed, CompressionLevel};

// Compress data with LZ4 (fast, default)
let compressed = DxCompressed::compress(b"your binary data here");

// Compress with specific level
let compressed = DxCompressed::compress_level(b"data", CompressionLevel::High);

// Get compression stats
println!("Original: {} bytes", compressed.original_size());
println!("Compressed: {} bytes", compressed.compressed_size());
println!("Ratio: {:.2%}", compressed.ratio());
println!("Space saved: {:.1}%", compressed.savings() * 100.0);

// Decompress (lazy - first access triggers decompression)
let data = compressed.decompress()?;

// Check if already decompressed (cached)
if compressed.is_cached() {
    println!("Data is cached in memory");
}
```

### Streaming Compression

For large datasets, use streaming compression to process data in chunks:

```rust
use serializer::machine::compress::StreamCompressor;

// Create streaming compressor (64KB chunks)
let mut compressor = StreamCompressor::default_chunk();

// Write data in chunks
compressor.write(&large_data_chunk_1);
compressor.write(&large_data_chunk_2);

// Finish and get compressed chunks
let chunks = compressor.finish();

// Each chunk is individually compressed
for chunk in chunks {
    println!("Chunk: {} → {} bytes", 
        chunk.original_size(), chunk.compressed_size());
}
```

### Cargo Features

Enable compression features in your `Cargo.toml`:

```toml
[dependencies]
dx-serializer = { version = "0.1", features = ["compression"] }

# Or enable specific algorithms:
dx-serializer = { version = "0.1", features = ["compression-lz4", "compression-zstd"] }
```

### Performance Characteristics

| Algorithm | Compression Speed | Decompression Speed | Ratio | Use Case |
|-----------|-------------------|---------------------|-------|----------|
| LZ4 | ~500 MB/s | ~2000 MB/s | 50-70% | Network, real-time |
| ZSTD-1 | ~300 MB/s | ~1000 MB/s | 60-80% | Fast compression |
| ZSTD-3 | ~100 MB/s | ~800 MB/s | 70-85% | Balanced |
| ZSTD-19 | ~10 MB/s | ~500 MB/s | 75-90% | Maximum compression |

### Wire Format

Compressed data includes size prepending for safe decompression:

```
[original_size: u32][compressed_data...]
```

This allows safe decompression without knowing the original size in advance.

## Conversion Rules

### LLM → Human
- Objects `name(key=val)` become `[name]` sections with key-value pairs
- Arrays `key=[item1 item2]` become `key:` followed by `- item` lines
- Keys are padded for alignment
- Nested sections use dot notation

### Human → LLM
- `[section]` headers with key-value pairs become `section(key=val)`
- `key:` followed by `- item` lines become `key=[item1 item2]`
- All whitespace padding is removed
- Numbered sections combine into tables

## Why DX Beats TOON

- No indentation - TOON requires 2 spaces per level
- Inline objects - `section:count[key=value]` vs nested YAML
- Space-separated arrays - No commas needed
- Tabular data - `name:count(schema)[rows]` for structured data
- Prefix elimination - `@prefix` removes repeated prefixes

## Quick Start
```rust
use serializer::{json_to_dx, dx_to_json};
let json = r#"{"name": "app", "version": "1.0"}"#;
let dx = json_to_dx(json)?;
```

## Features
```toml
[dependencies]
dx-serializer = { version = "0.1", features = ["tiktoken"] }
```
+--------------+----------------+
| Feature      | Description    |
+==============+================+
| `converters` | JSON/YAML/TOML |
+--------------+----------------+

## Documentation

- [Syntax Reference](docs/SYNTAX.md) - Complete LLM and Human format syntax
- [API Reference](docs/API.md) - Rust API documentation
- [Benchmarks](docs/BENCHMARKS.md) - Performance comparisons
- [Migration Guide](docs/MIGRATION.md) - Upgrading from previous versions

## License

MIT / Apache-2.0
