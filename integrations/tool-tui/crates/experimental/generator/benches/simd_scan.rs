//! SIMD Placeholder Scanning Benchmarks
//!
//! Benchmarks for AVX2 vs scalar placeholder detection.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dx_generator::PlaceholderScanner;

/// Generate test input with placeholders.
fn generate_input(size: usize, placeholder_density: f64) -> String {
    let mut result = String::with_capacity(size);
    let placeholder = "{{placeholder}}";
    let filler = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";

    let mut current_size = 0;
    let mut use_placeholder = false;

    while current_size < size {
        if use_placeholder && (rand::random::<f64>() < placeholder_density) {
            result.push_str(placeholder);
            current_size += placeholder.len();
        } else {
            let remaining = size - current_size;
            let chunk = if remaining >= filler.len() {
                filler
            } else {
                &filler[..remaining]
            };
            result.push_str(chunk);
            current_size += chunk.len();
        }
        use_placeholder = !use_placeholder;
    }

    result
}

/// Benchmark scalar scanning.
fn bench_scalar_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_scan");

    for size in [1024, 4096, 16384, 65536] {
        let input = generate_input(size, 0.1);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("size", size), input.as_bytes(), |b, data| {
            let scanner = PlaceholderScanner::new();
            b.iter(|| scanner.scan(black_box(data)))
        });
    }

    group.finish();
}

/// Benchmark with varying placeholder density.
fn bench_density_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("density_impact");

    let size = 16384;

    for density in [0.01, 0.05, 0.10, 0.20, 0.50] {
        let input = generate_input(size, density);
        let label = format!("{:.0}%", density * 100.0);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("density", &label), input.as_bytes(), |b, data| {
            let scanner = PlaceholderScanner::new();
            b.iter(|| scanner.scan(black_box(data)))
        });
    }

    group.finish();
}

/// Benchmark placeholder extraction.
fn bench_placeholder_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("placeholder_extraction");

    let inputs = [
        ("simple", "Hello, {{name}}!"),
        ("multiple", "{{greeting}}, {{name}}! Count: {{count}}"),
        ("nested_like", "{{outer{{inner}}end}}"),
        ("dense", "{{a}}{{b}}{{c}}{{d}}{{e}}{{f}}{{g}}{{h}}"),
    ];

    for (name, input) in inputs {
        group.bench_with_input(BenchmarkId::new("input", name), input.as_bytes(), |b, data| {
            let scanner = PlaceholderScanner::new();
            b.iter(|| scanner.scan(black_box(data)))
        });
    }

    group.finish();
}

/// Benchmark static segment extraction.
fn bench_static_segments(c: &mut Criterion) {
    let mut group = c.benchmark_group("static_segments");

    let template = r#"
        pub struct {{name}} {
            {{#each fields}}
            pub {{field_name}}: {{field_type}},
            {{/each}}
        }
        
        impl {{name}} {
            pub fn new({{#each fields}}{{field_name}}: {{field_type}},{{/each}}) -> Self {
                Self {
                    {{#each fields}}
                    {{field_name}},
                    {{/each}}
                }
            }
            
            pub fn update(&mut self, {{#each fields}}{{field_name}}: {{field_type}},{{/each}}) {
                {{#each fields}}
                self.{{field_name}} = {{field_name}};
                {{/each}}
            }
        }
    "#;

    group.bench_function("complex_template", |b| {
        let scanner = PlaceholderScanner::new();
        b.iter(|| scanner.scan(black_box(template.as_bytes())))
    });

    group.finish();
}

// Simple random implementation for benchmarks
mod rand {
    use std::cell::Cell;

    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0x853c49e6748fea9b) };
    }

    pub fn random<T: Random>() -> T {
        T::random()
    }

    fn next_u64() -> u64 {
        STATE.with(|state| {
            let mut s = state.get();
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            state.set(s);
            s.wrapping_mul(0x2545f4914f6cdd1d)
        })
    }

    pub trait Random {
        fn random() -> Self;
    }

    impl Random for f64 {
        fn random() -> Self {
            (next_u64() as f64) / (u64::MAX as f64)
        }
    }
}

criterion_group!(
    benches,
    bench_scalar_scan,
    bench_density_impact,
    bench_placeholder_extraction,
    bench_static_segments,
);

criterion_main!(benches);
