# DX Serializer Documentation

A token-efficient serialization format optimized for LLM context windows with high-performance binary encoding.

## Quick Start

```rust
use serializer::machine::{DxMachineBuilder, from_bytes};

// Serialize
let mut buffer = Vec::new();
let mut builder = DxMachineBuilder::new(&mut buffer, 2, 1);
builder.write_u64(0, 42);
builder.write_string(1, "hello");
builder.finish();

// Deserialize
let value: u64 = from_bytes(&buffer, 0)?;
```

## Core Features

### Three Format Architecture

DX Serializer provides three complementary formats:

1. **Human Format** (.toml, .sr) - Source of truth on disk
   - Easy to read and edit
   - Version control friendly
   - Standard TOML-like syntax

2. **LLM Format** (.llm) - Token-efficient for AI context
   - 52-73% token savings vs JSON
   - Compact representation
   - Auto-generated in `.dx/serializer/`

3. **Machine Format** (.machine) - Binary for performance
   - Zero-copy deserialization
   - Minimal overhead
   - Auto-generated in `.dx/serializer/`

### LZ4 Compression

Automatic compression for large objects:

```rust
use serializer::machine::{StreamCompressor, CompressionLevel};

let mut compressor = StreamCompressor::new(CompressionLevel::Fast);
compressor.write(&data)?;
let compressed = compressor.finish()?; // ~70% smaller
```

Best for: Network transmission, disk storage.

### Parallel Processing

Process data in parallel with Rayon:

```rust
use rayon::prelude::*;

items.par_iter()
    .map(|item| serialize(item))
    .collect()
```

Best for: Bulk data processing, batch operations.

## Best Practices

### 1. Choose the Right Format

- **Human format**: Source files, configuration, version control
- **LLM format**: AI context windows, token-efficient transmission
- **Machine format**: Runtime performance, zero-copy access

### 2. Minimize Allocations

```rust
// Reuse buffers when possible
let mut buffer = Vec::with_capacity(1024);
for item in items {
    buffer.clear();
    serialize(&item, &mut buffer)?;
    process(&buffer);
}
```

### 3. Use Type Hints

```rust
// Explicit types enable zero-copy paths
builder.write_u64(0, value); // Not write_generic()
```

### 4. Profile Before Optimizing

```bash
cargo bench -p dx-serializer
```

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Serialize | 10-20 ns | Small objects |
| Deserialize | 5-10 ns | Zero-copy |
| LLM Format | 52-73% | Token savings vs JSON |

## Thread Safety

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `DxMachineBuilder` | ✓ | ✗ | Mutable state |
| `DxMachine` | ✓ | ✓ | Immutable |

## Examples

See `crates/serializer/examples/` for complete examples:
- `basic_example.rs` - Getting started
- `compress_example.rs` - LZ4 compression
- `parallel_example.rs` - Batch processing

## Migration from RKYV

DX-Machine maintains wire format compatibility:

```rust
// RKYV
let bytes = rkyv::to_bytes(&data)?;

// DX-Machine (same format)
let mut buffer = Vec::new();
let mut builder = DxMachineBuilder::new(&mut buffer, 1, 1);
builder.write_u64(0, data.value);
builder.finish();
```

## Troubleshooting

### "Alignment error"

Ensure proper buffer alignment for direct I/O:

```rust
use serializer::machine::AlignedBuffer;
let buffer = AlignedBuffer::new(4096);
```

### "Compression failed"

Check input size (LZ4 requires >64 bytes):

```rust
if data.len() > 64 {
    compress(&data)?;
}
```

## See Also

- [API Reference](API.md)
- [Benchmarks](BENCHMARKS.md)
- [DX Zero Specification](DX_ZERO_SPECIFICATION.md)

## Future Roadmap

The following features are planned for future releases:

- **Platform-Native Async I/O**: io_uring (Linux), IOCP (Windows), kqueue (macOS)
- **Memory-Mapped Files**: Zero-copy file access for large datasets
- **Arena Allocator**: Batch processing with memory reuse
- **Quantum Field Access**: Partial deserialization for specific fields
- **SIMD Operations**: Vectorized batch processing
- **String Interning**: Size reduction for repeated strings
