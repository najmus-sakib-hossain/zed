#!/usr/bin/env bash

# Quick performance test script
# Usage: ./scripts/quick_bench.sh

echo "Building in release mode..."
cargo build --release 2>&1 | tail -5

echo ""
echo "Running quick benchmark..."
echo "========================================"

# Create a temporary HTML file for testing
TEMP_HTML=$(mktemp)
cat > "$TEMP_HTML" << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Performance Test</title></head>
<body>
  <div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold text-gray-900 mb-6">Performance Test Page</h1>
    
    <!-- Generate many elements with classes -->
EOF

# Add 1000 elements with various classes
for i in {1..1000}; do
    cat >> "$TEMP_HTML" << EOF
    <div class="flex items-center justify-between bg-gradient-to-r from-blue-$(($i % 10 * 100)) to-purple-$(($i % 10 * 100)) p-$((i % 8 + 1)) rounded-lg shadow-xl hover:shadow-2xl transition-all duration-$((i % 5 * 100 + 100))">
      <span class="text-$(($i % 6 + 1))xl font-bold">Element $i</span>
    </div>
EOF
done

cat >> "$TEMP_HTML" << 'EOF'
  </div>
</body>
</html>
EOF

echo "Test HTML created: $(wc -l < "$TEMP_HTML") lines"
echo "File size: $(du -h "$TEMP_HTML" | cut -f1)"
echo ""

# Copy to playgrounds directory if it exists
if [ -d "playgrounds" ]; then
    cp "$TEMP_HTML" playgrounds/perf_test.html
    echo "Test file copied to: playgrounds/perf_test.html"
fi

# Create a simple Rust test program
cat > /tmp/perf_test.rs << 'EOF'
use std::time::Instant;
use std::fs;

fn main() {
    let html = fs::read_to_string(std::env::args().nth(1).expect("Need HTML file path")).unwrap();
    let html_bytes = html.as_bytes();
    
    println!("HTML size: {} bytes", html_bytes.len());
    println!("Running 100 iterations...\n");
    
    // Warm up
    for _ in 0..10 {
        let _ = style::parser::extract_classes_fast(html_bytes, 128);
    }
    
    // Benchmark original
    let mut total_original = std::time::Duration::ZERO;
    for _ in 0..100 {
        let start = Instant::now();
        let result = style::parser::extract_classes_fast(html_bytes, 128);
        total_original += start.elapsed();
        std::hint::black_box(result);
    }
    let avg_original = total_original / 100;
    
    // Benchmark optimized
    let mut total_optimized = std::time::Duration::ZERO;
    for _ in 0..100 {
        let start = Instant::now();
        let result = style::parser::optimized::extract_classes_optimized(html_bytes, 128);
        total_optimized += start.elapsed();
        std::hint::black_box(result);
    }
    let avg_optimized = total_optimized / 100;
    
    println!("Results:");
    println!("  Original:  {:?} per iteration", avg_original);
    println!("  Optimized: {:?} per iteration", avg_optimized);
    
    let speedup = avg_original.as_nanos() as f64 / avg_optimized.as_nanos() as f64;
    let improvement = ((speedup - 1.0) * 100.0);
    
    if speedup > 1.0 {
        println!("\n✓ Optimized is {:.2}x faster ({:.1}% improvement)", speedup, improvement);
    } else {
        println!("\n✗ Original is {:.2}x faster", 1.0 / speedup);
    }
    
    // Verify correctness
    let result_orig = style::parser::extract_classes_fast(html_bytes, 128);
    let result_opt = style::parser::optimized::extract_classes_optimized(html_bytes, 128);
    
    if result_orig.classes == result_opt.classes {
        println!("✓ Results match - {} classes extracted", result_orig.classes.len());
    } else {
        println!("✗ Results differ!");
        println!("  Original: {} classes", result_orig.classes.len());
        println!("  Optimized: {} classes", result_opt.classes.len());
    }
}
EOF

# Compile and run the test
echo "Compiling test program..."
rustc --edition 2021 -L target/release/deps -L target/release --extern style=target/release/libstyle.rlib /tmp/perf_test.rs -o /tmp/perf_test -C opt-level=3 2>&1 | tail -5

if [ -f /tmp/perf_test ]; then
    echo ""
    echo "Running performance test..."
    echo "========================================"
    /tmp/perf_test "$TEMP_HTML"
    echo "========================================"
else
    echo "Failed to compile test program. Falling back to criterion benchmarks..."
    echo ""
    echo "Running subset of benchmarks..."
    cargo bench --bench style_benchmark -- html_parsing/small --quick
fi

# Cleanup
rm -f "$TEMP_HTML" /tmp/perf_test.rs /tmp/perf_test

echo ""
echo "Done! For full benchmarks, run: cargo bench"
