//! # SSR Benchmarks
//!
//! Benchmarks for server-side rendering (template inflation).
//!
//! Run with: `cargo bench --bench ssr_benchmarks -p dx-www-benchmarks`

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dx_www_packet::{SlotDef, SlotType, Template};
use dx_www_server::{StateData, inflate_html, inflate_page};

/// Generate a template with the given number of slots
fn generate_template(num_slots: usize) -> Template {
    let mut html = String::from("<div class=\"container\">");
    let mut slots = Vec::with_capacity(num_slots);

    for i in 0..num_slots {
        html.push_str(&format!(
            r#"<div class="item-{}"><span class="label">Item {}:</span><span class="value"><!--SLOT_{}--></span></div>"#,
            i, i, i
        ));
        slots.push(SlotDef {
            slot_id: i as u32,
            slot_type: SlotType::Text,
            path: vec![i as u32],
        });
    }

    html.push_str("</div>");

    Template {
        id: 1,
        html,
        slots,
        hash: "benchmark".to_string(),
    }
}

/// Generate state data for the given number of slots
fn generate_state(num_slots: usize) -> StateData {
    let mut state = StateData::new();
    for i in 0..num_slots {
        state.set(
            i as u32,
            format!("Value for slot {} with some additional text to make it realistic", i),
        );
    }
    state
}

/// Benchmark basic HTML inflation
fn bench_inflate_html(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssr_inflate_html");

    let slot_counts = [5, 20, 50, 100];

    for num_slots in slot_counts {
        let template = generate_template(num_slots);
        let state = generate_state(num_slots);

        // Calculate expected output size for throughput
        let output = inflate_html(&template, &state);
        let output_size = output.len();

        group.throughput(Throughput::Bytes(output_size as u64));

        group.bench_with_input(
            BenchmarkId::new("slots", num_slots),
            &(template, state),
            |b, (template, state)| {
                b.iter(|| black_box(inflate_html(black_box(template), black_box(state))));
            },
        );
    }

    group.finish();
}

/// Benchmark full page inflation
fn bench_inflate_page(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssr_inflate_page");

    let slot_counts = [5, 20, 50];

    for num_slots in slot_counts {
        let template = generate_template(num_slots);
        let state = generate_state(num_slots);

        let meta_tags = vec![
            (
                "description".to_string(),
                "A benchmark test page for SSR performance".to_string(),
            ),
            ("keywords".to_string(), "benchmark, ssr, dx-www, performance".to_string()),
            ("author".to_string(), "DX Team".to_string()),
            ("viewport".to_string(), "width=device-width, initial-scale=1.0".to_string()),
        ];

        let scripts = vec![
            "window.__DX_HYDRATION_DATA__ = {};".to_string(),
            "console.log('Page loaded');".to_string(),
        ];

        let output = inflate_page(&template, &state, "Benchmark Page", &meta_tags, &scripts);
        let output_size = output.len();

        group.throughput(Throughput::Bytes(output_size as u64));

        group.bench_with_input(
            BenchmarkId::new("full_page", num_slots),
            &(template, state, meta_tags.clone(), scripts.clone()),
            |b, (template, state, meta_tags, scripts)| {
                b.iter(|| {
                    black_box(inflate_page(
                        black_box(template),
                        black_box(state),
                        black_box("Benchmark Page"),
                        black_box(meta_tags),
                        black_box(scripts),
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pages per second (realistic workload)
fn bench_pages_per_second(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssr_pages_per_second");

    // Simulate a realistic dashboard page
    let dashboard_template = Template {
        id: 1,
        html: r#"
            <div class="dashboard">
                <header class="header">
                    <h1><!--SLOT_0--></h1>
                    <nav class="nav"><!--SLOT_1--></nav>
                </header>
                <main class="main">
                    <section class="stats">
                        <div class="stat-card"><span class="label">Users</span><span class="value"><!--SLOT_2--></span></div>
                        <div class="stat-card"><span class="label">Revenue</span><span class="value"><!--SLOT_3--></span></div>
                        <div class="stat-card"><span class="label">Orders</span><span class="value"><!--SLOT_4--></span></div>
                        <div class="stat-card"><span class="label">Growth</span><span class="value"><!--SLOT_5--></span></div>
                    </section>
                    <section class="chart"><!--SLOT_6--></section>
                    <section class="table"><!--SLOT_7--></section>
                </main>
                <footer class="footer"><!--SLOT_8--></footer>
            </div>
        "#.to_string(),
        slots: (0..9).map(|i| SlotDef {
            slot_id: i,
            slot_type: SlotType::Text,
            path: vec![i],
        }).collect(),
        hash: "dashboard".to_string(),
    };

    let mut dashboard_state = StateData::new();
    dashboard_state.set(0, "Dashboard".to_string());
    dashboard_state.set(
        1,
        "<a href='/'>Home</a> | <a href='/users'>Users</a> | <a href='/settings'>Settings</a>"
            .to_string(),
    );
    dashboard_state.set(2, "12,345".to_string());
    dashboard_state.set(3, "$98,765.43".to_string());
    dashboard_state.set(4, "1,234".to_string());
    dashboard_state.set(5, "+15.3%".to_string());
    dashboard_state.set(6, "<canvas id='chart'></canvas>".to_string());
    dashboard_state
        .set(7, "<table><tr><th>ID</th><th>Name</th><th>Status</th></tr></table>".to_string());
    dashboard_state.set(8, "© 2025 DX Team".to_string());

    group.throughput(Throughput::Elements(1)); // 1 page per iteration

    group.bench_function("dashboard_page", |b| {
        b.iter(|| {
            black_box(inflate_html(black_box(&dashboard_template), black_box(&dashboard_state)))
        });
    });

    // Simulate a blog post page
    let blog_template = Template {
        id: 2,
        html: r#"
            <article class="blog-post">
                <header>
                    <h1 class="title"><!--SLOT_0--></h1>
                    <div class="meta">
                        <span class="author">By <!--SLOT_1--></span>
                        <span class="date"><!--SLOT_2--></span>
                        <span class="reading-time"><!--SLOT_3--> min read</span>
                    </div>
                </header>
                <div class="content"><!--SLOT_4--></div>
                <footer>
                    <div class="tags"><!--SLOT_5--></div>
                    <div class="share"><!--SLOT_6--></div>
                </footer>
            </article>
        "#
        .to_string(),
        slots: (0..7)
            .map(|i| SlotDef {
                slot_id: i,
                slot_type: SlotType::Text,
                path: vec![i],
            })
            .collect(),
        hash: "blog".to_string(),
    };

    let mut blog_state = StateData::new();
    blog_state.set(0, "Understanding Web Performance".to_string());
    blog_state.set(1, "Jane Developer".to_string());
    blog_state.set(2, "January 8, 2026".to_string());
    blog_state.set(3, "5".to_string());
    blog_state.set(4, "<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p><p>Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>".to_string());
    blog_state.set(5, "<span class='tag'>performance</span><span class='tag'>web</span><span class='tag'>optimization</span>".to_string());
    blog_state.set(
        6,
        "<button>Share on Twitter</button><button>Share on LinkedIn</button>".to_string(),
    );

    group.bench_function("blog_post_page", |b| {
        b.iter(|| black_box(inflate_html(black_box(&blog_template), black_box(&blog_state))));
    });

    group.finish();
}

/// Benchmark with varying content sizes
fn bench_content_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ssr_content_sizes");

    let content_sizes = [100, 1000, 10000, 50000]; // bytes of content

    for content_size in content_sizes {
        let content: String = (0..content_size).map(|i| ((i % 26) as u8 + b'a') as char).collect();

        let template = Template {
            id: 1,
            html: "<div class=\"content\"><!--SLOT_0--></div>".to_string(),
            slots: vec![SlotDef {
                slot_id: 0,
                slot_type: SlotType::Text,
                path: vec![0],
            }],
            hash: "content".to_string(),
        };

        let mut state = StateData::new();
        state.set(0, content);

        let output = inflate_html(&template, &state);
        group.throughput(Throughput::Bytes(output.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("content_bytes", content_size),
            &(template, state),
            |b, (template, state)| {
                b.iter(|| black_box(inflate_html(black_box(template), black_box(state))));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_inflate_html,
    bench_inflate_page,
    bench_pages_per_second,
    bench_content_sizes
);

criterion_main!(benches);
