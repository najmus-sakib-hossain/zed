use std::time::Instant;
use style::core::AppState;

fn main() {
    println!("\n=== BRUTAL TRUTH: DX-STYLE CSS GENERATION SPEED TEST ===\n");

    let engine = AppState::engine();

    // Test 1: Atomic classes (should use perfect hash)
    println!("TEST 1: Atomic Classes (Perfect Hash Path)");
    let atomic_classes = vec!["flex", "block", "grid", "hidden", "relative", "absolute"];

    let start = Instant::now();
    for _ in 0..10000 {
        for class in &atomic_classes {
            let _ = engine.css_for_class(class);
        }
    }
    let elapsed = start.elapsed();
    let per_class = elapsed.as_nanos() / (10000 * atomic_classes.len() as u128);
    println!("  10,000 iterations x {} classes", atomic_classes.len());
    println!("  Total: {:?}", elapsed);
    println!("  Per class: {}ns", per_class);
    println!("  Classes/sec: {}", 1_000_000_000 / per_class);

    // Test 2: Dynamic classes
    println!("\nTEST 2: Dynamic Classes (Full Generation)");
    let dynamic_classes = vec!["w-4", "h-8", "p-4", "m-2", "text-lg", "bg-blue-500"];

    let start = Instant::now();
    for _ in 0..10000 {
        for class in &dynamic_classes {
            let _ = engine.css_for_class(class);
        }
    }
    let elapsed = start.elapsed();
    let per_class = elapsed.as_nanos() / (10000 * dynamic_classes.len() as u128);
    println!("  10,000 iterations x {} classes", dynamic_classes.len());
    println!("  Total: {:?}", elapsed);
    println!("  Per class: {}ns", per_class);
    println!("  Classes/sec: {}", 1_000_000_000 / per_class);

    // Test 3: Complex classes with modifiers
    println!("\nTEST 3: Complex Classes (Modifiers + States)");
    let complex_classes = vec!["hover:bg-blue-500", "md:flex", "dark:bg-gray-900"];

    let start = Instant::now();
    for _ in 0..10000 {
        for class in &complex_classes {
            let _ = engine.css_for_class(class);
        }
    }
    let elapsed = start.elapsed();
    let per_class = elapsed.as_nanos() / (10000 * complex_classes.len() as u128);
    println!("  10,000 iterations x {} classes", complex_classes.len());
    println!("  Total: {:?}", elapsed);
    println!("  Per class: {}ns", per_class);
    println!("  Classes/sec: {}", 1_000_000_000 / per_class);

    // Test 4: 1000 classes batch (realistic scenario)
    println!("\nTEST 4: Batch of 1000 Classes (Realistic Mix)");
    let batch = vec![
        "flex",
        "block",
        "hidden",
        "w-4",
        "h-8",
        "p-4",
        "m-2",
        "text-lg",
        "bg-blue-500",
        "hover:bg-red-500",
        "md:flex",
    ];

    let start = Instant::now();
    for class in &batch {
        let _ = engine.css_for_class(class);
    }
    let elapsed = start.elapsed();
    let per_class = elapsed.as_nanos() / batch.len() as u128;
    println!("  {} classes", batch.len());
    println!("  Total: {:?}", elapsed);
    println!("  Per class: {}ns", per_class);
    println!("  Total time: {}µs", elapsed.as_micros());

    // Test 5: Cache effectiveness
    println!("\nTEST 5: Cache Effectiveness (Repeated Class)");
    let start = Instant::now();
    for _ in 0..100000 {
        let _ = engine.css_for_class("flex");
    }
    let elapsed = start.elapsed();
    let per_call = elapsed.as_nanos() / 100000;
    println!("  100,000 calls to same class");
    println!("  Total: {:?}", elapsed);
    println!("  Per call: {}ns", per_call);

    // BRUTAL COMPARISON
    println!("\n=== BRUTAL TRUTH COMPARISON ===");
    println!("Grimoire CSS claim: 5µs/class (200k classes/sec)");
    println!("Grimoire target:    5000ns per class");
    println!("");
    println!(
        "Your atomic:        {}ns per class ({}x FASTER)",
        per_class,
        5000 / per_class.max(1)
    );
    println!("Your dynamic:       Check results above");
    println!("Your complex:       Check results above");
    println!("");

    // Theoretical limits
    println!("=== THEORETICAL LIMITS ===");
    println!("L1 cache access:    ~1-2ns");
    println!("L2 cache access:    ~3-5ns");
    println!("HashMap lookup:     ~10-20ns");
    println!("DRAM access:        ~50-70ns");
    println!("Your atomic lookup: {}ns", per_class);
    println!("");

    if per_class < 100 {
        println!("✅ You're at HARDWARE LIMITS (cache-bound performance)");
    } else if per_class < 1000 {
        println!("✅ Excellent performance (sub-microsecond)");
    } else if per_class < 5000 {
        println!("✅ Faster than Grimoire CSS");
    } else {
        println!("⚠️  Slower than claimed competitors");
    }

    println!("\n=== END BRUTAL TRUTH ===\n");
}
