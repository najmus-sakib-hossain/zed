use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::hint::black_box as hint_black_box;
use style::parser::{extract_classes_fast, rewrite_duplicate_classes};

// Benchmark HTML samples of varying sizes
const SMALL_HTML: &str = r#"<div class="flex items-center bg-red-500 text-white p-4"></div>"#;

const MEDIUM_HTML: &str = r#"
<div class="container mx-auto px-4">
    <div class="flex items-center justify-between bg-gradient-to-r from-blue-500 to-purple-600 p-6 rounded-lg shadow-xl">
        <h1 class="text-3xl font-bold text-white hover:text-gray-100 transition-colors">Title</h1>
        <button class="bg-white text-blue-600 px-6 py-2 rounded-md hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500">Click Me</button>
    </div>
    <div class="grid grid-cols-3 gap-4 mt-8">
        <div class="bg-white p-4 rounded shadow hover:shadow-lg transition-shadow">Card 1</div>
        <div class="bg-white p-4 rounded shadow hover:shadow-lg transition-shadow">Card 2</div>
        <div class="bg-white p-4 rounded shadow hover:shadow-lg transition-shadow">Card 3</div>
    </div>
</div>
"#;

const LARGE_HTML: &str = include_str!("../playgrounds/index.html");

fn generate_complex_html(num_elements: usize) -> String {
    let mut html = String::with_capacity(num_elements * 200);
    html.push_str("<html><body>");

    for i in 0..num_elements {
        html.push_str(&format!(
            r#"<div class="flex items-center justify-between bg-gradient-to-r from-blue-{} to-purple-{} p-{} rounded-lg shadow-{} hover:shadow-{} transition-all duration-{} ease-in-out">
                <h2 class="text-{}xl font-bold text-white hover:text-gray-{} focus:text-gray-{}">Element {}</h2>
                <button class="bg-white text-blue-{} px-{} py-{} rounded-md hover:bg-gray-{} focus:outline-none focus:ring-{} focus:ring-blue-{}">Action</button>
            </div>"#,
            i % 10 * 100,
            (i + 1) % 10 * 100,
            i % 8 + 1,
            if i % 3 == 0 { "xl" } else { "md" },
            if i % 3 == 1 { "2xl" } else { "lg" },
            i % 5 * 100 + 100,
            i % 6 + 1,
            i % 10 * 100,
            (i + 2) % 10 * 100,
            i,
            i % 10 * 100,
            i % 8 + 2,
            i % 4 + 1,
            i % 10 * 100,
            i % 4 + 1,
            i % 10 * 100,
        ));
    }

    html.push_str("</body></html>");
    html
}

fn generate_grouping_html(num_groups: usize) -> String {
    let mut html = String::with_capacity(num_groups * 300);
    html.push_str("<html><body>");

    for i in 0..num_groups {
        html.push_str(&format!(
            r#"<div class="card(bg-white p-4 rounded shadow hover:shadow-lg transition-all) mx-{} my-{}">
                <div class="header(flex items-center justify-between mb-4)">
                    <h3 class="title(text-xl font-bold text-gray-800)">Group {}</h3>
                </div>
            </div>"#,
            i % 8 + 1,
            i % 6 + 1,
            i
        ));
    }

    html.push_str("</body></html>");
    html
}

fn bench_html_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("html_parsing");

    for (name, html) in &[
        ("small", SMALL_HTML),
        ("medium", MEDIUM_HTML),
        ("large_100", &generate_complex_html(100)),
        ("large_500", &generate_complex_html(500)),
        ("large_1000", &generate_complex_html(1000)),
    ] {
        let html_bytes = html.as_bytes();
        group.throughput(Throughput::Bytes(html_bytes.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(name), &html_bytes, |b, &data| {
            b.iter(|| {
                let result = extract_classes_fast(black_box(data), 64);
                hint_black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_class_extraction_capacity_hints(c: &mut Criterion) {
    let mut group = c.benchmark_group("capacity_hints");
    let html = generate_complex_html(200);
    let html_bytes = html.as_bytes();

    for capacity in &[0, 16, 64, 128, 256, 512, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(capacity), capacity, |b, &cap| {
            b.iter(|| {
                let result = extract_classes_fast(black_box(html_bytes), cap);
                hint_black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_grouping_features(c: &mut Criterion) {
    let mut group = c.benchmark_group("grouping");

    for num_groups in &[10, 50, 100, 200] {
        let html = generate_grouping_html(*num_groups);
        let html_bytes = html.as_bytes();

        group.throughput(Throughput::Elements(*num_groups as u64));

        group.bench_with_input(
            BenchmarkId::new("extract_with_groups", num_groups),
            &html_bytes,
            |b, &data| {
                b.iter(|| {
                    let result = extract_classes_fast(black_box(data), 128);
                    hint_black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_duplicate_rewriting(c: &mut Criterion) {
    let mut group = c.benchmark_group("duplicate_rewriting");

    // HTML with duplicates
    let duplicate_html = r#"
        <div class="flex items-center bg-red-500 p-4">One</div>
        <div class="flex items-center bg-red-500 p-4">Two</div>
        <div class="flex items-center bg-red-500 p-4">Three</div>
        <div class="flex items-center bg-red-500 p-4">Four</div>
    "#;

    let duplicate_bytes = duplicate_html.as_bytes();

    group.bench_function("rewrite_duplicates", |b| {
        b.iter(|| {
            let result = rewrite_duplicate_classes(black_box(duplicate_bytes));
            hint_black_box(result);
        });
    });

    // HTML without duplicates
    let no_dup_html = r#"
        <div class="flex items-center bg-red-500 p-4">One</div>
        <div class="grid grid-cols-3 gap-4">Two</div>
        <div class="absolute top-0 left-0">Three</div>
        <div class="relative z-10 overflow-hidden">Four</div>
    "#;

    let no_dup_bytes = no_dup_html.as_bytes();

    group.bench_function("no_duplicates", |b| {
        b.iter(|| {
            let result = rewrite_duplicate_classes(black_box(no_dup_bytes));
            hint_black_box(result);
        });
    });

    group.finish();
}

fn bench_incremental_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_updates");

    // Simulate adding a single class
    let base_html = generate_complex_html(100);
    let added_html = base_html.replace("</body>", r#"<div class="new-class-added"></div></body>"#);

    let base_bytes = base_html.as_bytes();
    let added_bytes = added_html.as_bytes();

    group.bench_function("baseline_parse", |b| {
        b.iter(|| {
            let result = extract_classes_fast(black_box(base_bytes), 128);
            hint_black_box(result);
        });
    });

    group.bench_function("with_one_addition", |b| {
        b.iter(|| {
            let result = extract_classes_fast(black_box(added_bytes), 128);
            hint_black_box(result);
        });
    });

    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    let html = generate_complex_html(200);
    let html_bytes = html.as_bytes();

    // Benchmark with different pre-allocation strategies
    group.bench_function("zero_capacity", |b| {
        b.iter(|| {
            let result = extract_classes_fast(black_box(html_bytes), 0);
            hint_black_box(result);
        });
    });

    group.bench_function("optimal_capacity", |b| {
        // First, determine optimal capacity
        let sample = extract_classes_fast(html_bytes, 0);
        let optimal = sample.classes.len().next_power_of_two();

        b.iter(|| {
            let result = extract_classes_fast(black_box(html_bytes), optimal);
            hint_black_box(result);
        });
    });

    group.finish();
}

fn bench_real_world_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world");

    // Simulate a typical e-commerce product page
    let product_page = r#"
    <div class="container mx-auto px-4 py-8">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
            <div class="product-images">
                <img class="w-full rounded-lg shadow-xl hover:shadow-2xl transition-shadow" src="product.jpg" />
                <div class="grid grid-cols-4 gap-2 mt-4">
                    <img class="rounded hover:opacity-75 cursor-pointer" src="thumb1.jpg" />
                    <img class="rounded hover:opacity-75 cursor-pointer" src="thumb2.jpg" />
                    <img class="rounded hover:opacity-75 cursor-pointer" src="thumb3.jpg" />
                    <img class="rounded hover:opacity-75 cursor-pointer" src="thumb4.jpg" />
                </div>
            </div>
            <div class="product-info">
                <h1 class="text-3xl font-bold text-gray-900 mb-4">Product Name</h1>
                <div class="flex items-center mb-4">
                    <div class="flex text-yellow-400">★★★★★</div>
                    <span class="ml-2 text-gray-600">(128 reviews)</span>
                </div>
                <p class="text-2xl font-bold text-blue-600 mb-6">$99.99</p>
                <p class="text-gray-700 mb-6">Product description goes here...</p>
                <button class="w-full bg-blue-600 text-white py-3 px-6 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors">
                    Add to Cart
                </button>
            </div>
        </div>
    </div>
    "#;

    let bytes = product_page.as_bytes();
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("product_page", |b| {
        b.iter(|| {
            let result = extract_classes_fast(black_box(bytes), 64);
            hint_black_box(result);
        });
    });

    group.finish();
}

// Micro-benchmarks for specific operations
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_ops");

    let class_list = "flex items-center justify-between bg-gradient-to-r from-blue-500 to-purple-600 p-6 rounded-lg shadow-xl hover:shadow-2xl transition-all duration-300";

    group.bench_function("split_whitespace", |b| {
        b.iter(|| {
            let result: Vec<&str> = black_box(class_list).split_whitespace().collect();
            hint_black_box(result);
        });
    });

    group.bench_function("contains_check", |b| {
        b.iter(|| {
            let result = black_box(class_list).contains("hover:");
            hint_black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_html_parsing,
    bench_class_extraction_capacity_hints,
    bench_grouping_features,
    bench_duplicate_rewriting,
    bench_incremental_updates,
    bench_memory_allocation,
    bench_real_world_scenario,
    bench_string_operations,
);

criterion_main!(benches);
