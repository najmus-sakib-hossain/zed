//! String interning example
//!
//! Demonstrates how string interning reduces serialized size for data with repeated strings.

use serializer::machine::{InterningDeserializer, InterningSerializer};

fn main() {
    println!("=== String Interning Example ===\n");

    // Create a serializer with string interning
    let mut serializer = InterningSerializer::new();

    // Simulate log data with repeated strings
    let log_entries = vec![
        ("ERROR", "Failed to connect to database"),
        ("INFO", "Connection established"),
        ("ERROR", "Failed to connect to database"), // Duplicate
        ("WARN", "High memory usage detected"),
        ("INFO", "Connection established"),         // Duplicate
        ("ERROR", "Failed to connect to database"), // Duplicate
        ("DEBUG", "Processing request"),
        ("INFO", "Connection established"), // Duplicate
    ];

    println!("Original log entries:");
    for (level, msg) in &log_entries {
        println!("  [{level}] {msg}");
    }

    // Intern all strings
    let mut interned_entries = Vec::new();
    for (level, msg) in &log_entries {
        let level_idx = serializer.intern(level);
        let msg_idx = serializer.intern(msg);
        interned_entries.push((level_idx, msg_idx));
    }

    println!("\n=== Interning Statistics ===");
    println!("Total strings: {}", log_entries.len() * 2);
    println!("Unique strings: {}", serializer.pool().len());
    println!(
        "Deduplication ratio: {:.1}%",
        (1.0 - serializer.pool().len() as f64 / (log_entries.len() * 2) as f64) * 100.0
    );

    // Serialize the pool
    let pool_bytes = serializer.serialize_pool();
    println!("\nSerialized pool size: {} bytes", pool_bytes.len());

    // Calculate original size (rough estimate)
    let original_size: usize = log_entries.iter().map(|(level, msg)| level.len() + msg.len()).sum();
    println!("Original string data size: ~{} bytes", original_size);
    println!(
        "Size reduction: {:.1}%",
        (1.0 - pool_bytes.len() as f64 / original_size as f64) * 100.0
    );

    // Deserialize and verify
    println!("\n=== Deserialization ===");
    let (deserializer, consumed) = InterningDeserializer::new(&pool_bytes).unwrap();
    println!("Consumed {} bytes from buffer", consumed);

    println!("\nReconstructed log entries:");
    for (level_idx, msg_idx) in &interned_entries {
        let level = deserializer.get(*level_idx).unwrap();
        let msg = deserializer.get(*msg_idx).unwrap();
        println!("  [{level}] {msg}");
    }

    // Verify correctness
    println!("\n=== Verification ===");
    let mut all_correct = true;
    for (i, ((level_idx, msg_idx), (orig_level, orig_msg))) in
        interned_entries.iter().zip(&log_entries).enumerate()
    {
        let level = deserializer.get(*level_idx).unwrap();
        let msg = deserializer.get(*msg_idx).unwrap();

        if level != *orig_level || msg != *orig_msg {
            println!("❌ Entry {} mismatch!", i);
            all_correct = false;
        }
    }

    if all_correct {
        println!("✅ All entries match original data!");
    }

    println!("\n=== Use Cases ===");
    println!("String interning is ideal for:");
    println!("  • Log aggregation (repeated log levels, messages)");
    println!("  • Configuration files (repeated keys, values)");
    println!("  • API responses (repeated field names)");
    println!("  • Database exports (repeated enum values)");
    println!("\nExpected size reduction: 50-90% for typical workloads");
}
