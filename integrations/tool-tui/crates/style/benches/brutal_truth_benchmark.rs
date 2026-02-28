use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;
use style::core::AppState;

/// Brutal truth: Test ACTUAL CSS generation speed
/// No sugar-coating, no excuses - just raw numbers

fn benchmark_atomic_classes(c: &mut Criterion) {
    let engine = AppState::engine();

    let atomic_classes = vec![
        "block",
        "flex",
        "grid",
        "hidden",
        "inline",
        "inline-block",
        "relative",
        "absolute",
        "fixed",
        "sticky",
        "flex-row",
        "flex-col",
        "flex-wrap",
        "items-center",
        "items-start",
        "items-end",
        "justify-center",
        "justify-between",
        "justify-start",
    ];

    let mut group = c.benchmark_group("atomic_lookups");
    group.measurement_time(Duration::from_secs(10));

    for class in &atomic_classes {
        group.bench_with_input(BenchmarkId::from_parameter(class), class, |b, class| {
            b.iter(|| black_box(engine.css_for_class(black_box(class))));
        });
    }

    group.finish();
}

fn benchmark_dynamic_classes(c: &mut Criterion) {
    let engine = AppState::engine();

    let dynamic_classes = vec![
        "w-4",
        "h-8",
        "p-4",
        "m-2",
        "mt-4",
        "mb-8",
        "text-lg",
        "text-xl",
        "text-2xl",
        "bg-blue-500",
        "text-red-600",
        "border-gray-300",
        "rounded-lg",
        "shadow-md",
        "opacity-50",
    ];

    let mut group = c.benchmark_group("dynamic_generation");
    group.measurement_time(Duration::from_secs(10));

    for class in &dynamic_classes {
        group.bench_with_input(BenchmarkId::from_parameter(class), class, |b, class| {
            b.iter(|| black_box(engine.css_for_class(black_box(class))));
        });
    }

    group.finish();
}

fn benchmark_complex_classes(c: &mut Criterion) {
    let engine = AppState::engine();

    let complex_classes = vec![
        "hover:bg-blue-500",
        "md:flex",
        "lg:text-xl",
        "dark:bg-gray-900",
        "hover:dark:bg-gray-800",
        "md:hover:text-blue-500",
        "lg:dark:hover:bg-red-500",
    ];

    let mut group = c.benchmark_group("complex_generation");
    group.measurement_time(Duration::from_secs(10));

    for class in &complex_classes {
        group.bench_with_input(BenchmarkId::from_parameter(class), class, |b, class| {
            b.iter(|| black_box(engine.css_for_class(black_box(class))));
        });
    }

    group.finish();
}

fn benchmark_batch_generation(c: &mut Criterion) {
    let engine = AppState::engine();

    // Realistic batch: mix of atomic, dynamic, and complex classes
    let realistic_batch = vec![
        "flex",
        "items-center",
        "justify-between",
        "p-4",
        "bg-white",
        "rounded-lg",
        "shadow-md",
        "hover:shadow-lg",
        "transition-all",
        "w-full",
        "max-w-4xl",
        "mx-auto",
        "space-y-4",
        "text-gray-900",
        "dark:text-gray-100",
        "dark:bg-gray-800",
        "md:flex-row",
        "md:space-y-0",
        "md:space-x-4",
        "lg:max-w-6xl",
        "lg:p-6",
    ];

    let mut group = c.benchmark_group("batch_generation");
    group.throughput(Throughput::Elements(realistic_batch.len() as u64));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("realistic_component", |b| {
        b.iter(|| {
            for class in &realistic_batch {
                black_box(engine.css_for_class(black_box(class)));
            }
        });
    });

    group.finish();
}

fn benchmark_1000_classes(c: &mut Criterion) {
    let engine = AppState::engine();

    // Generate 1000 diverse classes
    let mut classes: Vec<String> = Vec::with_capacity(1000);

    // 300 atomic
    for _ in 0..100 {
        classes.push("flex".to_string());
        classes.push("block".to_string());
        classes.push("hidden".to_string());
    }

    // 400 dynamic
    for i in 0..100 {
        classes.push(format!("w-{}", i % 96));
        classes.push(format!("h-{}", i % 96));
        classes.push(format!("p-{}", i % 24));
        classes.push(format!("m-{}", i % 24));
    }

    // 300 complex
    for i in 0..100 {
        classes.push(format!("hover:bg-blue-{}", (i % 9 + 1) * 100));
        classes.push(format!("md:text-{}", if i % 2 == 0 { "lg" } else { "xl" }));
        classes.push(format!("dark:bg-gray-{}", (i % 9 + 1) * 100));
    }

    let mut group = c.benchmark_group("large_scale");
    group.throughput(Throughput::Elements(1000));
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("1000_classes", |b| {
        b.iter(|| {
            for class in &classes {
                black_box(engine.css_for_class(black_box(class)));
            }
        });
    });

    group.finish();
}

fn benchmark_cache_effectiveness(c: &mut Criterion) {
    let engine = AppState::engine();

    let repeated_class = "flex";

    let mut group = c.benchmark_group("cache_effectiveness");
    group.measurement_time(Duration::from_secs(10));

    // Repeated calls (hot cache)
    group.bench_function("repeated_call_hot", |b| {
        // Warm up cache
        for _ in 0..100 {
            engine.css_for_class(repeated_class);
        }

        b.iter(|| black_box(engine.css_for_class(black_box(repeated_class))));
    });

    group.finish();
}

fn benchmark_vs_grimoire_claim(c: &mut Criterion) {
    let engine = AppState::engine();

    // Grimoire CSS claims: 200k classes/second = 5µs per class
    // Let's see if we can beat that

    let test_classes = vec![
        "flex",
        "block",
        "w-4",
        "h-8",
        "p-4",
        "bg-blue-500",
        "text-lg",
        "rounded-lg",
        "shadow-md",
        "hover:bg-red-500",
    ];

    let mut group = c.benchmark_group("vs_grimoire");
    group.measurement_time(Duration::from_secs(15));

    // Grimoire's claimed speed: 5µs per class
    let grimoire_target = Duration::from_micros(5);

    group.bench_function("dx_style_average", |b| {
        b.iter(|| {
            for class in &test_classes {
                black_box(engine.css_for_class(black_box(class)));
            }
        });
    });

    println!("\n=== BRUTAL TRUTH COMPARISON ===");
    println!("Grimoire CSS claim: 5µs per class (200k/sec)");
    println!("Target to beat: {:?} per class", grimoire_target);
    println!("Run the benchmark to see if we beat it!");

    group.finish();
}

fn benchmark_theoretical_limit(c: &mut Criterion) {
    let engine = AppState::engine();

    // Theoretical limit: 10-20ns (L1/L2 cache bound)
    // Your atomic lookups: 60ns
    // Let's measure the actual overhead

    let mut group = c.benchmark_group("theoretical_limits");
    group.measurement_time(Duration::from_secs(10));

    // Pure hash lookup (no CSS generation)
    group.bench_function("hash_lookup_only", |b| {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("flex", "display: flex;");

        b.iter(|| black_box(map.get(black_box("flex"))));
    });

    // Atomic class lookup (with CSS generation)
    group.bench_function("atomic_with_css", |b| {
        b.iter(|| black_box(engine.css_for_class(black_box("flex"))));
    });

    // String allocation overhead
    group.bench_function("string_allocation", |b| {
        b.iter(|| black_box(String::from("display: flex;")));
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_atomic_classes,
    benchmark_dynamic_classes,
    benchmark_complex_classes,
    benchmark_batch_generation,
    benchmark_1000_classes,
    benchmark_cache_effectiveness,
    benchmark_vs_grimoire_claim,
    benchmark_theoretical_limit,
);

criterion_main!(benches);
