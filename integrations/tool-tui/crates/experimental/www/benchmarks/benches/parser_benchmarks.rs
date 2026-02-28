//! # Parser Benchmarks
//!
//! Benchmarks for TSX/JSX parsing throughput.
//!
//! Run with: `cargo bench --bench parser_benchmarks -p dx-www-benchmarks`

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

/// Generate a sample TSX component with the given complexity
fn generate_tsx_component(num_props: usize, num_state: usize, jsx_depth: usize) -> String {
    let mut tsx = String::new();

    // Imports
    tsx.push_str("import React, { useState, useEffect, useMemo } from 'react';\n");
    tsx.push_str("import { Button, Card, Input } from './components';\n");
    tsx.push_str("import { useAuth, useTheme } from './hooks';\n\n");

    // Interface for props
    tsx.push_str("interface Props {\n");
    for i in 0..num_props {
        tsx.push_str(&format!("  prop{}: string;\n", i));
    }
    tsx.push_str("  children?: React.ReactNode;\n");
    tsx.push_str("}\n\n");

    // Component function
    tsx.push_str("export default function BenchmarkComponent({\n");
    for i in 0..num_props {
        tsx.push_str(&format!("  prop{},\n", i));
    }
    tsx.push_str("  children,\n}: Props) {\n");

    // State declarations
    for i in 0..num_state {
        tsx.push_str(&format!(
            "  const [state{}, setState{}] = useState<string>('initial{}');\n",
            i, i, i
        ));
    }
    tsx.push('\n');
    tsx.push_str("  const { user, isAuthenticated } = useAuth();\n");
    tsx.push_str("  const { theme, toggleTheme } = useTheme();\n\n");
    tsx.push_str("  useEffect(() => {\n    console.log('Component mounted');\n");
    tsx.push_str("    return () => console.log('Component unmounted');\n  }, []);\n\n");
    tsx.push_str("  const computedValue = useMemo(() => state0.toUpperCase(), [state0]);\n\n");
    tsx.push_str("  const handleClick = () => setState0('clicked');\n\n");
    tsx.push_str("  return (\n");
    generate_jsx_tree(&mut tsx, jsx_depth, 4);
    tsx.push_str("  );\n}\n");
    tsx
}

/// Generate nested JSX tree
fn generate_jsx_tree(tsx: &mut String, depth: usize, indent: usize) {
    let spaces = " ".repeat(indent);

    if depth == 0 {
        tsx.push_str(&format!("{}<span>Leaf node</span>\n", spaces));
        return;
    }

    tsx.push_str(&format!("{}<div className=\"level-{}\">\n", spaces, depth));
    tsx.push_str(&format!(
        "{}  <h{}>Heading Level {}</h{}>\n",
        spaces,
        depth.min(6),
        depth,
        depth.min(6)
    ));
    tsx.push_str(&format!(
        "{}  <p className=\"description\">Description at level {}</p>\n",
        spaces, depth
    ));
    tsx.push_str(&format!("{}  <Button onClick={{handleClick}}>Click me</Button>\n", spaces));

    // Conditional rendering
    tsx.push_str(&format!("{}  {{isAuthenticated && (\n", spaces));
    tsx.push_str(&format!("{}    <Card>\n", spaces));
    tsx.push_str(&format!("{}      <span>Welcome, {{user?.name}}</span>\n", spaces));
    tsx.push_str(&format!("{}    </Card>\n", spaces));
    tsx.push_str(&format!("{}  )}}\n", spaces));

    // Nested children
    generate_jsx_tree(tsx, depth - 1, indent + 2);

    tsx.push_str(&format!("{}</div>\n", spaces));
}

/// Generate a simple TSX file
fn generate_simple_tsx() -> String {
    r#"
import React from 'react';

interface Props {
  name: string;
}

export default function Hello({ name }: Props) {
  return <div>Hello, {name}!</div>;
}
"#
    .to_string()
}

/// Generate a medium complexity TSX file
fn generate_medium_tsx() -> String {
    generate_tsx_component(5, 3, 3)
}

/// Generate a complex TSX file
fn generate_complex_tsx() -> String {
    generate_tsx_component(15, 10, 6)
}

/// Benchmark parsing throughput (lines per second)
fn bench_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_throughput");

    let test_cases = [
        ("simple", generate_simple_tsx()),
        ("medium", generate_medium_tsx()),
        ("complex", generate_complex_tsx()),
    ];

    for (name, source) in &test_cases {
        let line_count = source.lines().count();
        let byte_count = source.len();

        group.throughput(Throughput::Elements(line_count as u64));

        group.bench_with_input(BenchmarkId::new("lines", name), source, |b, source| {
            b.iter(|| {
                // Simulate parsing by processing the source
                let lines: Vec<&str> = black_box(source).lines().collect();
                black_box(lines.len())
            });
        });

        group.throughput(Throughput::Bytes(byte_count as u64));

        group.bench_with_input(BenchmarkId::new("bytes", name), source, |b, source| {
            b.iter(|| {
                let bytes = black_box(source).as_bytes();
                black_box(bytes.len())
            });
        });
    }

    group.finish();
}

/// Benchmark security validation
fn bench_security_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_security");

    let banned_keywords = [
        "eval",
        "innerHTML",
        "outerHTML",
        "document.write",
        "Function(",
        "dangerouslySetInnerHTML",
        "javascript:",
        "data:text/html",
    ];

    let safe_source = generate_complex_tsx();
    let source_size = safe_source.len();

    group.throughput(Throughput::Bytes(source_size as u64));

    group.bench_function("validate_safe_source", |b| {
        b.iter(|| {
            let source = black_box(&safe_source);
            for keyword in &banned_keywords {
                if source.contains(keyword) {
                    return black_box(false);
                }
            }
            black_box(true)
        });
    });

    let unsafe_source = format!("{}\n// eval('test')", safe_source);

    group.bench_function("validate_unsafe_source", |b| {
        b.iter(|| {
            let source = black_box(&unsafe_source);
            for keyword in &banned_keywords {
                if source.contains(keyword) {
                    return black_box(false);
                }
            }
            black_box(true)
        });
    });

    group.finish();
}

/// Benchmark import extraction (regex-based)
fn bench_import_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_imports");

    // Generate source with many imports
    let mut source = String::new();
    for i in 0..50 {
        source
            .push_str(&format!("import {{ Component{}, Helper{} }} from './module{}';\n", i, i, i));
    }
    source.push_str(&generate_medium_tsx());

    let import_count = 50;
    group.throughput(Throughput::Elements(import_count));

    group.bench_function("extract_imports", |b| {
        b.iter(|| {
            let source = black_box(&source);
            let mut count = 0;
            for line in source.lines() {
                if line.trim_start().starts_with("import ") {
                    count += 1;
                }
            }
            black_box(count)
        });
    });

    group.finish();
}

/// Benchmark hash computation for cache invalidation
fn bench_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_hash");

    let sources = [
        ("small", generate_simple_tsx()),
        ("medium", generate_medium_tsx()),
        ("large", generate_complex_tsx()),
    ];

    for (name, source) in &sources {
        let byte_count = source.len();
        group.throughput(Throughput::Bytes(byte_count as u64));

        group.bench_with_input(BenchmarkId::new("blake3", name), source, |b, source| {
            b.iter(|| {
                let hash = blake3::hash(black_box(source.as_bytes()));
                black_box(hash.to_hex().to_string())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parser_throughput,
    bench_security_validation,
    bench_import_extraction,
    bench_hash_computation
);

criterion_main!(benches);
