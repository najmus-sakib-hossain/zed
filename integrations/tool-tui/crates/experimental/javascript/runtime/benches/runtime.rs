//! Benchmarks for dx-js-runtime

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dx_js_runtime::{compiler::OptLevel, Compiler, CompilerConfig, DxRuntime};

fn bench_cold_start(c: &mut Criterion) {
    c.bench_function("cold_start", |b| {
        b.iter(|| {
            let runtime = DxRuntime::new().unwrap();
            black_box(runtime)
        })
    });
}

fn bench_parse_simple(c: &mut Criterion) {
    let source = r#"
        function add(a, b) {
            return a + b;
        }
        add(1, 2);
    "#;

    c.bench_function("parse_simple", |b| {
        b.iter(|| {
            let result = dx_js_runtime::compiler::parser::parse(black_box(source), "bench.js");
            black_box(result)
        })
    });
}

fn bench_compile_simple(c: &mut Criterion) {
    let mut compiler = Compiler::new(CompilerConfig {
        type_check: false,
        optimization_level: OptLevel::Basic,
    })
    .unwrap();

    let source = r#"
        function add(a, b) {
            return a + b;
        }
        add(1, 2);
    "#;

    c.bench_function("compile_simple", |b| {
        b.iter(|| {
            let module = compiler.compile(black_box(source), "bench.js").unwrap();
            black_box(module)
        })
    });
}

fn bench_fibonacci_source(c: &mut Criterion) {
    let source = r#"
        function fib(n) {
            if (n <= 1) return n;
            return fib(n - 1) + fib(n - 2);
        }
        fib(20);
    "#;

    let mut runtime = DxRuntime::new().unwrap();

    c.bench_function("fibonacci_20", |b| {
        b.iter(|| {
            let result = runtime.run_sync(black_box(source), "fib.js").unwrap();
            black_box(result)
        })
    });
}

fn bench_parse_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_sizes");

    for size in [100, 1000, 10000].iter() {
        // Generate source of given size
        let source = generate_source(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| dx_js_runtime::compiler::parser::parse(black_box(source), "bench.js"))
        });
    }

    group.finish();
}

fn generate_source(num_functions: usize) -> String {
    let mut source = String::new();
    for i in 0..num_functions {
        source.push_str(&format!("function f{}(x) {{ return x + {}; }}\n", i, i));
    }
    source.push_str("f0(42);\n");
    source
}

criterion_group!(
    benches,
    bench_cold_start,
    bench_parse_simple,
    bench_compile_simple,
    bench_fibonacci_source,
    bench_parse_sizes,
);

criterion_main!(benches);
