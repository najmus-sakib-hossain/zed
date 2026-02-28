//! Benchmarks for DX Markdown Context Compiler.
//!
//! Run with: cargo bench -p dx-markdown
//!
//! Performance targets:
//! - Parse throughput: 100 MB/s
//! - Compile throughput: 50 MB/s
//! - Token counting: 10 MB/s
//! - Memory usage: 2x input size

#![allow(clippy::expect_used, clippy::unwrap_used)] // Benchmarks can use expect/unwrap

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dx_markdown::{
    compiler::DxMarkdown,
    simd,
    tokenizer::Tokenizer,
    types::{CompilerConfig, TokenizerType},
};

/// Sample markdown content for benchmarking.
fn sample_readme() -> String {
    r#"# DX Markdown

[![Build Status](https://img.shields.io/badge/build-passing-green)](https://example.com)
[![License](https://img.shields.io/badge/license-MIT-blue)](https://example.com)

DX Markdown is a **Context Compiler** for the AI era. It transforms standard Markdown into token-optimized output for LLMs.

## Features

- 25-50% token reduction on typical documentation
- SIMD-accelerated parsing
- Multiple optimization modes
- Streaming support for large files

## Installation

```bash
cargo add dx-markdown
```

## Usage

```rust
use dx_markdown::DxMarkdown;

let compiler = DxMarkdown::default_compiler()?;
let result = compiler.compile(input)?;
println!("Saved {}% tokens", result.savings_percent());
```

## API Reference

| Function | Description | Returns |
|----------|-------------|---------|
| `compile` | Compile markdown | `CompileResult` |
| `compile_streaming` | Stream compile | `CompileResult` |
| `strip_urls` | Remove URLs | `String` |
| `strip_images` | Remove images | `String` |

## Configuration

The compiler supports multiple modes:

- **full**: Apply all optimizations (default)
- **code**: Keep code, minimal prose
- **docs**: Keep explanations, minimal code
- **data**: Keep tables/lists, strip narrative
- **aggressive**: Maximum compression

## Performance

DX Markdown achieves excellent performance through:

1. SIMD-accelerated byte searching
2. Zero-copy parsing where possible
3. Efficient memory allocation
4. Parallel processing for repositories

For more information, see the [documentation](https://docs.example.com/dx-markdown).

## License

MIT License - see [LICENSE](LICENSE) for details.
"#.to_string()
}

/// Generate large markdown content for throughput testing.
fn generate_large_content(size_kb: usize) -> String {
    let base = sample_readme();
    let mut content = String::with_capacity(size_kb * 1024);
    while content.len() < size_kb * 1024 {
        content.push_str(&base);
        content.push_str("\n\n---\n\n");
    }
    content.truncate(size_kb * 1024);
    content
}

/// Benchmark compiler throughput.
fn bench_compile_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile_throughput");

    for size_kb in [1, 10, 100, 1000] {
        let content = generate_large_content(size_kb);
        group.throughput(Throughput::Bytes(content.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("compile", format!("{}KB", size_kb)),
            &content,
            |b, content| {
                let compiler = DxMarkdown::default_compiler().unwrap();
                b.iter(|| black_box(compiler.compile(black_box(content)).unwrap()));
            },
        );
    }

    group.finish();
}

/// Benchmark token counting.
fn bench_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_counting");

    for size_kb in [1, 10, 100] {
        let content = generate_large_content(size_kb);
        group.throughput(Throughput::Bytes(content.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("count_tokens", format!("{}KB", size_kb)),
            &content,
            |b, content| {
                let tokenizer = Tokenizer::new(TokenizerType::Cl100k).unwrap();
                b.iter(|| black_box(tokenizer.count(black_box(content))));
            },
        );
    }

    group.finish();
}

/// Benchmark SIMD operations.
fn bench_simd_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_operations");

    let content = generate_large_content(100);
    let bytes = content.as_bytes();
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("find_newline", |b| {
        b.iter(|| black_box(simd::find_newline(black_box(bytes))));
    });

    group.bench_function("find_pipe", |b| {
        b.iter(|| black_box(simd::find_pipe(black_box(bytes))));
    });

    group.bench_function("count_newlines", |b| {
        b.iter(|| black_box(simd::count_byte(black_box(bytes), b'\n')));
    });

    group.bench_function("find_code_fence", |b| {
        b.iter(|| black_box(simd::find_code_fence(black_box(bytes))));
    });

    group.finish();
}

/// Benchmark different optimization modes.
fn bench_optimization_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimization_modes");

    let content = sample_readme();

    for mode in ["full", "code", "docs", "data", "aggressive"] {
        let config = match mode {
            "full" => CompilerConfig::default(),
            "code" => CompilerConfig::code(),
            "docs" => CompilerConfig::docs(),
            "data" => CompilerConfig::data(),
            "aggressive" => CompilerConfig::aggressive(),
            _ => unreachable!(),
        };

        group.bench_with_input(BenchmarkId::new("mode", mode), &content, |b, content| {
            let compiler = DxMarkdown::new(config.clone()).unwrap();
            b.iter(|| black_box(compiler.compile(black_box(content)).unwrap()));
        });
    }

    group.finish();
}

/// Benchmark individual optimizations.
fn bench_individual_optimizations(c: &mut Criterion) {
    let mut group = c.benchmark_group("individual_optimizations");

    let content = sample_readme();

    // URL stripping
    group.bench_function("strip_urls", |b| {
        b.iter(|| black_box(dx_markdown::strip_urls(black_box(&content))));
    });

    // Image stripping
    group.bench_function("strip_images", |b| {
        b.iter(|| black_box(dx_markdown::strip_images(black_box(&content))));
    });

    // Badge stripping
    group.bench_function("strip_badges", |b| {
        b.iter(|| black_box(dx_markdown::strip_badges(black_box(&content))));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_compile_throughput,
    bench_token_counting,
    bench_simd_operations,
    bench_optimization_modes,
    bench_individual_optimizations,
);

criterion_main!(benches);
