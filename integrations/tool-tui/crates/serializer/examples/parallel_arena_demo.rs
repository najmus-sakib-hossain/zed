//! Parallel Arena Serialization Demo
//!
//! Demonstrates zero-contention parallel serialization using thread-local arenas.
//!
//! Run with: cargo run --example parallel_arena_demo --features parallel,converters

#[cfg(feature = "parallel")]
use serializer::machine::parallel::{par_serialize, par_serialize_chunked, par_serialize_indexed};

#[cfg(feature = "parallel")]
fn main() {
    println!("=== Parallel Arena Serialization Demo ===\n");

    // Example 1: Basic parallel serialization
    println!("1. Basic Parallel Serialization");
    let items: Vec<u64> = (0..10).collect();
    let results = par_serialize(&items, |arena, &item| {
        arena.write_header(0);
        let mut writer = arena.writer();
        writer.write_u64::<0>(item * 2);
        arena.advance(8);
        arena.to_vec()
    });
    println!("   Serialized {} items in parallel", results.len());
    println!("   First item size: {} bytes\n", results[0].len());

    // Example 2: Indexed parallel serialization
    println!("2. Indexed Parallel Serialization");
    let items = vec!["Alice", "Bob", "Charlie"];
    let results = par_serialize_indexed(&items, |arena, idx, &name| {
        arena.write_header(0);
        let mut writer = arena.writer();
        writer.write_u64::<0>(idx as u64);
        arena.advance(8);
        (idx, name, arena.to_vec())
    });
    for (idx, name, bytes) in &results {
        println!("   [{}] {} -> {} bytes", idx, name, bytes.len());
    }
    println!();

    // Example 3: Chunked parallel serialization
    println!("3. Chunked Parallel Serialization");
    let items: Vec<u64> = (0..100).collect();
    let results = par_serialize_chunked(&items, 10, |arena, chunk| {
        arena.write_header(0);
        for &item in chunk {
            let mut writer = arena.writer();
            writer.write_u64::<0>(item);
            arena.advance(8);
        }
        arena.to_vec()
    });
    println!("   Processed {} items in {} chunks", items.len(), results.len());
    println!("   Chunk size: {} bytes\n", results[0].len());

    // Example 4: Large-scale parallel processing
    println!("4. Large-Scale Parallel Processing");
    let large_dataset: Vec<u64> = (0..100_000).collect();

    let start = std::time::Instant::now();
    let results = par_serialize(&large_dataset, |arena, &item| {
        arena.write_header(0);
        let mut writer = arena.writer();
        writer.write_u64::<0>(item);
        writer.write_u64::<8>(item * item);
        arena.advance(16);
        arena.to_vec()
    });
    let duration = start.elapsed();

    println!("   Serialized {} items in {:?}", results.len(), duration);
    println!(
        "   Throughput: {:.2} items/ms",
        results.len() as f64 / duration.as_millis() as f64
    );
    println!("   Average item size: {} bytes", results[0].len());

    println!("\n=== Demo Complete ===");
}

#[cfg(not(feature = "parallel"))]
fn main() {
    println!("This example requires the 'parallel' feature.");
    println!("Run with: cargo run --example parallel_arena_demo --features parallel,converters");
}
