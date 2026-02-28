//! Example: Performance comparison with JSON

use serializer::*;
use std::time::Instant;

fn main() -> serializer::error::Result<()> {
    println!("=== DX vs JSON Performance Comparison ===\n");

    // Test data
    let dx_data = b"project:DX Runtime
version:0.1.0
team>alice|bob|charlie|diana
tasks=id%i name%s hours%f urgent%b
1 Parser 12.5 +
2 Encoder 8.0 +
3 Tests 6.5 -
4 Docs 4.0 -
_ Bench 3.5 +";

    let json_data = r#"{
  "project": "DX Runtime",
  "version": "0.1.0",
  "team": ["alice", "bob", "charlie", "diana"],
  "tasks": [
    {"id": 1, "name": "Parser", "hours": 12.5, "urgent": true},
    {"id": 2, "name": "Encoder", "hours": 8.0, "urgent": true},
    {"id": 3, "name": "Tests", "hours": 6.5, "urgent": false},
    {"id": 4, "name": "Docs", "hours": 4.0, "urgent": false},
    {"id": 5, "name": "Bench", "hours": 3.5, "urgent": true}
  ]
}"#;

    // Size comparison
    println!("📦 SIZE COMPARISON:");
    println!("  DX:   {} bytes", dx_data.len());
    println!("  JSON: {} bytes", json_data.len());
    println!(
        "  Compression: {:.1}% smaller\n",
        (1.0 - dx_data.len() as f64 / json_data.len() as f64) * 100.0
    );

    // Parse speed comparison
    println!("⚡ PARSE SPEED:");

    // Warm up
    for _ in 0..100 {
        let _ = parse(dx_data);
        let _ = serde_json::from_str::<serde_json::Value>(json_data);
    }

    // DX parsing
    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = parse(dx_data).unwrap();
    }
    let dx_duration = start.elapsed();
    let dx_per_op = dx_duration.as_nanos() / iterations;

    // JSON parsing
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = serde_json::from_str::<serde_json::Value>(json_data).unwrap();
    }
    let json_duration = start.elapsed();
    let json_per_op = json_duration.as_nanos() / iterations;

    println!("  DX:   {:.2}µs per parse", dx_per_op as f64 / 1000.0);
    println!("  JSON: {:.2}µs per parse", json_per_op as f64 / 1000.0);
    println!("  Speedup: {:.1}x faster\n", json_per_op as f64 / dx_per_op as f64);

    // Human format
    println!("👁️  HUMAN-READABLE FORMAT:");
    let parsed = parse(dx_data)?;
    let human = format_human(&parsed)?;
    println!("{}", human);

    Ok(())
}
