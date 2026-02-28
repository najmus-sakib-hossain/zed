use std::time::Instant;
use style::parser::{extract_classes_fast, optimized::extract_classes_optimized};

#[test]
fn test_parser_correctness() {
    let html_samples = vec![
        r#"<div class="flex items-center"></div>"#,
        r#"<div class="bg-red-500 text-white p-4"></div>"#,
        r#"<div dx-text="card(bg-white p-4 rounded)"></div>"#,
        r#"<div class="hover:bg-blue-500 focus:outline-none"></div>"#,
    ];

    for html in html_samples {
        let html_bytes = html.as_bytes();
        let original = extract_classes_fast(html_bytes, 64);
        let optimized = extract_classes_optimized(html_bytes, 64);

        assert_eq!(original.classes, optimized.classes, "Classes should match for HTML: {}", html);

        assert_eq!(
            original.group_events.len(),
            optimized.group_events.len(),
            "Group events count should match for HTML: {}",
            html
        );
    }
}

#[test]
fn test_performance_baseline() {
    // Generate a realistic HTML sample
    let mut html = String::from("<html><body>");
    for i in 0..100 {
        html.push_str(&format!(
            r#"<div class="flex items-center bg-red-{} text-white p-{}">Element {}</div>"#,
            i % 10 * 100,
            i % 8 + 1,
            i
        ));
    }
    html.push_str("</body></html>");

    let html_bytes = html.as_bytes();

    // Warm up
    for _ in 0..5 {
        let _ = extract_classes_fast(html_bytes, 128);
    }

    // Measure performance
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let result = extract_classes_fast(html_bytes, 128);
        std::hint::black_box(result);
    }
    let duration = start.elapsed();
    let avg_micros = duration.as_micros() / iterations;

    println!("Average parse time: {}µs", avg_micros);
    println!("Classes extracted: {}", extract_classes_fast(html_bytes, 128).classes.len());

    // Performance assertion - should be well under 100µs for 100 elements
    assert!(avg_micros < 100, "Parser should take less than 100µs, got {}µs", avg_micros);
}

#[test]
fn test_optimized_vs_original_performance() {
    let mut html = String::from("<html><body>");
    for i in 0..500 {
        html.push_str(&format!(
            r#"<div class="flex-{} items-{} bg-color-{} p-{}">Item</div>"#,
            i % 10,
            i % 5,
            i % 20,
            i % 8 + 1
        ));
    }
    html.push_str("</body></html>");

    let html_bytes = html.as_bytes();

    // Warm up both
    for _ in 0..5 {
        let _ = extract_classes_fast(html_bytes, 256);
        let _ = extract_classes_optimized(html_bytes, 256);
    }

    // Measure original
    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let result = extract_classes_fast(html_bytes, 256);
        std::hint::black_box(result);
    }
    let original_time = start.elapsed();

    // Measure optimized
    let start = Instant::now();
    for _ in 0..iterations {
        let result = extract_classes_optimized(html_bytes, 256);
        std::hint::black_box(result);
    }
    let optimized_time = start.elapsed();

    let original_avg = original_time.as_micros() / iterations;
    let optimized_avg = optimized_time.as_micros() / iterations;

    println!("\nPerformance Comparison:");
    println!("  Original:  {}µs", original_avg);
    println!("  Optimized: {}µs", optimized_avg);

    if optimized_avg < original_avg {
        let speedup = original_avg as f64 / optimized_avg as f64;
        let improvement = (speedup - 1.0) * 100.0;
        println!("  Speedup:   {:.2}x ({:.1}% faster)", speedup, improvement);
    } else {
        println!("  Note: Optimized version may be slightly slower for small inputs");
    }
}

#[test]
fn test_large_html_performance() {
    // Test with a very large HTML file
    let mut html = String::from("<html><body>");
    for i in 0..2000 {
        html.push_str(&format!(
            r#"<div class="flex items-center justify-between bg-gradient-to-r from-blue-{} to-purple-{} p-{} rounded-lg shadow-xl hover:shadow-2xl transition-all duration-{}">
                <span class="text-{}xl font-bold">Element {}</span>
            </div>"#,
            i % 10 * 100,
            (i + 1) % 10 * 100,
            i % 8 + 1,
            i % 5 * 100 + 100,
            i % 6 + 1,
            i
        ));
    }
    html.push_str("</body></html>");

    let html_bytes = html.as_bytes();
    println!("Large HTML size: {} bytes", html_bytes.len());

    // Warm up
    for _ in 0..3 {
        let _ = extract_classes_fast(html_bytes, 512);
    }

    // Measure
    let iterations = 20;
    let start = Instant::now();
    for _ in 0..iterations {
        let result = extract_classes_fast(html_bytes, 512);
        std::hint::black_box(result);
    }
    let duration = start.elapsed();
    let avg_millis = duration.as_millis() / iterations;
    let avg_micros = duration.as_micros() / iterations;

    println!("Large HTML parse time: {}ms ({}µs)", avg_millis, avg_micros);

    // Should handle large files efficiently
    assert!(
        avg_millis < 10,
        "Large HTML parsing should take less than 10ms, got {}ms",
        avg_millis
    );
}

#[test]
fn test_grouping_performance() {
    let mut html = String::from("<html><body>");
    for i in 0..200 {
        html.push_str(&format!(
            r#"<div class="card(bg-white p-4 rounded shadow) mx-{} my-{}">
                <div class="header(flex items-center)">Group {}</div>
            </div>"#,
            i % 8 + 1,
            i % 6 + 1,
            i
        ));
    }
    html.push_str("</body></html>");

    let html_bytes = html.as_bytes();

    // Warm up
    for _ in 0..3 {
        let _ = extract_classes_fast(html_bytes, 128);
    }

    // Measure
    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let result = extract_classes_fast(html_bytes, 128);
        std::hint::black_box(result);
    }
    let duration = start.elapsed();
    let avg_micros = duration.as_micros() / iterations;

    println!("Grouping parse time: {}µs", avg_micros);

    let result = extract_classes_fast(html_bytes, 128);
    println!("Group events: {}", result.group_events.len());

    // Grouping should still be fast (allow up to 2ms for slower systems/CI)
    assert!(
        avg_micros < 2000,
        "Grouping parsing should take less than 2000µs, got {}µs",
        avg_micros
    );
}
