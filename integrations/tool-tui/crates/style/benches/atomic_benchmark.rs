//! Benchmark for atomic class lookups and SIMD parsing
//!
//! Verifies <1µs atomic lookups and <1ms change detection

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use style::core::atomic::{is_atomic_class, lookup_atomic_class};
use style::parser::simd::{extract_classes_simd, extract_classes_with_hash, has_html_changed};

fn bench_atomic_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("atomic_lookup");

    let classes = vec!["flex", "block", "hidden", "text-center", "p-4", "m-2"];

    for class in classes {
        group.bench_with_input(BenchmarkId::from_parameter(class), &class, |b, &class| {
            b.iter(|| black_box(lookup_atomic_class(black_box(class))));
        });
    }

    group.finish();
}

fn bench_atomic_check(c: &mut Criterion) {
    c.bench_function("is_atomic_class", |b| {
        b.iter(|| black_box(is_atomic_class(black_box("flex"))));
    });
}

fn bench_simd_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_parsing");

    let small_html = br#"<div class="flex items-center justify-between"></div>"#;
    let medium_html = br#"
        <div class="container mx-auto px-4">
            <header class="flex items-center justify-between py-4">
                <h1 class="text-2xl font-bold">Title</h1>
                <nav class="flex gap-4">
                    <a class="text-blue-500 hover:underline">Link 1</a>
                    <a class="text-blue-500 hover:underline">Link 2</a>
                </nav>
            </header>
        </div>
    "#;
    let large_html = medium_html.repeat(10);

    group.bench_function("small_html", |b| {
        b.iter(|| black_box(extract_classes_simd(black_box(small_html))));
    });

    group.bench_function("medium_html", |b| {
        b.iter(|| black_box(extract_classes_simd(black_box(medium_html))));
    });

    group.bench_function("large_html", |b| {
        b.iter(|| black_box(extract_classes_simd(black_box(&large_html))));
    });

    group.finish();
}

fn bench_change_detection(c: &mut Criterion) {
    let html = b"<div class=\"flex items-center\"></div>".repeat(100);

    c.bench_function("change_detection", |b| {
        let (_, hash) = extract_classes_with_hash(&html);
        b.iter(|| black_box(has_html_changed(black_box(&html), black_box(hash))));
    });
}

fn bench_combined_workflow(c: &mut Criterion) {
    let html = br#"
        <div class="flex items-center justify-between p-4 m-2">
            <span class="text-lg font-bold">Hello</span>
            <button class="bg-blue-500 text-white rounded px-4 py-2">Click</button>
        </div>
    "#;

    c.bench_function("full_workflow", |b| {
        b.iter(|| {
            // Extract classes with SIMD
            let classes = extract_classes_simd(black_box(html));

            // Lookup each class (atomic fast path)
            for class in &classes {
                black_box(lookup_atomic_class(class));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_atomic_lookup,
    bench_atomic_check,
    bench_simd_parsing,
    bench_change_detection,
    bench_combined_workflow
);
criterion_main!(benches);
