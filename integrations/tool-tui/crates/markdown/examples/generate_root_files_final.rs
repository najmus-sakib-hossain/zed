use dx_markdown::convert::{human_to_llm, human_to_machine, machine_to_llm};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    println!("=== Generating Machine Files for Root Markdown Files ===\n");

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let output_dir = dx_root.join(".dx/markdown");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let files = vec![
        "README.md",
        "HUMAN_FORMAT.md",
        "LLM_FORMAT.md",
        "MACHINE_FORMAT.md",
        "MARKDOWN.md",
        "MARKDOWN_NOISE.md",
        "MARKDOWN_VISUAL.md",
        "ZED.md",
    ];

    let mut total_human = 0;
    let mut total_llm = 0;
    let mut total_machine = 0;
    let mut serialize_times = Vec::new();
    let mut deserialize_times = Vec::new();

    println!("Processing files...\n");

    for filename in &files {
        let path = dx_root.join(filename);
        if !path.exists() {
            continue;
        }

        let file_stem = path.file_stem().unwrap().to_string_lossy();
        let human_content = fs::read_to_string(&path).expect("Failed to read");

        // Convert to LLM
        let llm_content = human_to_llm(&human_content).expect("Failed to convert to LLM");

        // Benchmark serialization
        let start = Instant::now();
        let machine_bytes = human_to_machine(&human_content).expect("Failed to serialize");
        serialize_times.push(start.elapsed());

        // Benchmark deserialization
        let start = Instant::now();
        let _ = machine_to_llm(&machine_bytes).expect("Failed to deserialize");
        deserialize_times.push(start.elapsed());

        // Save files
        let human_path = output_dir.join(format!("{}.human", file_stem));
        let machine_path = output_dir.join(format!("{}.machine", file_stem));

        fs::write(&human_path, &human_content).expect("Failed to write human");
        fs::write(&machine_path, &machine_bytes).expect("Failed to write machine");

        total_human += human_content.len();
        total_llm += llm_content.len();
        total_machine += machine_bytes.len();

        println!(
            "✓ {:<25} {:>7} → {:>7} → {:>7} bytes",
            filename,
            human_content.len(),
            llm_content.len(),
            machine_bytes.len()
        );
    }

    // Calculate stats
    let avg_serialize_ns = serialize_times.iter().sum::<std::time::Duration>().as_nanos()
        / serialize_times.len() as u128;
    let avg_deserialize_ns = deserialize_times.iter().sum::<std::time::Duration>().as_nanos()
        / deserialize_times.len() as u128;

    println!("\n=== Summary ===");
    println!("Files: {}", files.len());
    println!("\nSizes:");
    println!("  Human:   {:>8} bytes ({:.2} KB)", total_human, total_human as f64 / 1024.0);
    println!(
        "  LLM:     {:>8} bytes ({:.2} KB, {:.1}% of human)",
        total_llm,
        total_llm as f64 / 1024.0,
        (total_llm as f64 / total_human as f64) * 100.0
    );
    println!(
        "  Machine: {:>8} bytes ({:.2} KB, {:.1}% of human)",
        total_machine,
        total_machine as f64 / 1024.0,
        (total_machine as f64 / total_human as f64) * 100.0
    );

    println!("\nPerformance:");
    println!("  Serialize:   {:.2} µs avg", avg_serialize_ns as f64 / 1000.0);
    println!("  Deserialize: {:.2} µs avg", avg_deserialize_ns as f64 / 1000.0);
    println!(
        "  Round-trip:  {:.2} µs avg",
        (avg_serialize_ns + avg_deserialize_ns) as f64 / 1000.0
    );

    // Detailed benchmark
    println!("\n=== Detailed Benchmark (1000 iterations) ===");
    let test_file = dx_root.join("README.md");
    let content = fs::read_to_string(&test_file).unwrap();
    let machine = human_to_machine(&content).unwrap();

    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = machine_to_llm(&machine).unwrap();
    }
    let duration = start.elapsed();
    let avg = duration.as_nanos() / iterations as u128;

    println!("README.md deserialization:");
    println!("  {} iterations in {:?}", iterations, duration);
    println!("  Average: {} ns ({:.2} µs)", avg, avg as f64 / 1000.0);
    println!("  Throughput: {:.0} ops/sec", 1_000_000_000.0 / avg as f64);

    println!("\n✓ Machine format generation complete!");
    println!("  Files saved to: .dx/markdown/");
}
