//! Template Rendering Benchmarks
//!
//! Benchmarks for micro and macro rendering modes.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use dx_generator::{BinaryTemplate, Compiler, Generator, Parameters, RenderMode, Renderer};

/// Benchmark micro renderer (direct patching).
fn bench_micro_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("micro_render");

    // Create a simple template
    let template = BinaryTemplate::builder("greeting").build();

    let params = Parameters::new().set("name", "World").set("count", 42i32);

    let mut renderer = Renderer::with_mode(RenderMode::Micro);

    group.bench_function("simple_template", |b| {
        b.iter(|| renderer.render(black_box(&template), black_box(&params)))
    });

    group.finish();
}

/// Benchmark macro renderer (bytecode interpreter).
fn bench_macro_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("macro_render");

    let template = BinaryTemplate::builder("component").build();

    let params = Parameters::new()
        .set("name", "Counter")
        .set("initial_value", 0i32)
        .set("with_state", true);

    let mut renderer = Renderer::with_mode(RenderMode::Macro);

    group.bench_function("component_template", |b| {
        b.iter(|| renderer.render(black_box(&template), black_box(&params)))
    });

    group.finish();
}

/// Benchmark template compilation.
fn bench_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("compilation");

    let source_small = "Hello, {{name}}!";
    let source_medium = r#"
        pub struct {{name}} {
            {{#each fields}}
            pub {{name}}: {{type}},
            {{/each}}
        }
        
        impl {{name}} {
            pub fn new() -> Self {
                Self {
                    {{#each fields}}
                    {{name}}: Default::default(),
                    {{/each}}
                }
            }
        }
    "#;

    let compiler = Compiler::new();

    group.bench_with_input(BenchmarkId::new("source", "small"), source_small, |b, src| {
        b.iter(|| compiler.compile(black_box(src)))
    });

    group.bench_with_input(BenchmarkId::new("source", "medium"), source_medium, |b, src| {
        b.iter(|| compiler.compile(black_box(src)))
    });

    group.finish();
}

/// Benchmark full generation pipeline.
fn bench_full_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_generation");

    let mut generator = Generator::new();
    generator.compile_template("test", "Hello, {{name}}!").unwrap();

    let params = Parameters::new().set("name", "World");

    group.bench_function("generate", |b| {
        b.iter(|| generator.generate(black_box("test"), black_box(&params)))
    });

    group.finish();
}

/// Benchmark parameter encoding/decoding.
fn bench_params(c: &mut Criterion) {
    let mut group = c.benchmark_group("parameters");

    let params = Parameters::new()
        .set("name", "Counter")
        .set("count", 42i32)
        .set("enabled", true)
        .set("ratio", 3.14f64)
        .set("description", "A simple counter component");

    group.bench_function("encode", |b| b.iter(|| params.encode()));

    let encoded = params.encode();

    group.bench_function("decode", |b| b.iter(|| Parameters::decode(black_box(&encoded))));

    group.finish();
}

criterion_group!(
    benches,
    bench_micro_render,
    bench_macro_render,
    bench_compilation,
    bench_full_generation,
    bench_params,
);

criterion_main!(benches);
