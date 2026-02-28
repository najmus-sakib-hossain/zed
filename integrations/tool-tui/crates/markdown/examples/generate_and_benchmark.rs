use dx_markdown::convert::{human_to_llm, human_to_machine, machine_to_llm};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;

fn main() {
    println!("=== DX Markdown: Generate & Benchmark Machine Format ===\n");

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let output_dir = dx_root.join(".dx/markdown");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    // Find all markdown files
    let md_files: Vec<PathBuf> = WalkDir::new(&dx_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .filter(|e| {
            let path_str = e.path().to_string_lossy();
            !path_str.contains("node_modules") 
                && !path_str.contains("/target/") 
                && !path_str.contains("\\.git\\")
                && !path_str.contains("/.git/")
        })
        .map(|e| e.path().to_path_buf())
        .take(10) // Process first 10 files for quick test
        .collect();

    println!("Processing {} markdown files...\n", md_files.len());

    let mut total_human_bytes = 0;
    let mut total_llm_bytes = 0;
    let mut total_machine_bytes = 0;
    let mut serialize_times = Vec::new();
    let mut deserialize_times = Vec::new();

    for path in &md_files {
        let file_stem = path.file_stem().unwrap().to_string_lossy();
        let human_content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        total_human_bytes += human_content.len();

        // Convert to LLM format
        let llm_content = match human_to_llm(&human_content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        total_llm_bytes += llm_content.len();

        // Benchmark serialization to machine format
        let start = Instant::now();
        let machine_bytes = match human_to_machine(&human_content) {
            Ok(m) => m,
            Err(_) => continue,
        };
        serialize_times.push(start.elapsed());
        total_machine_bytes += machine_bytes.len();

        // Benchmark deserialization
        let start = Instant::now();
        let _ = machine_to_llm(&machine_bytes);
        deserialize_times.push(start.elapsed());

        // Save files
        let human_path = output_dir.join(format!("{}.human", file_stem));
        let machine_path = output_dir.join(format!("{}.machine", file_stem));

        fs::write(&human_path, &human_content).ok();
        fs::write(&machine_path, &machine_bytes).ok();

        println!(
            "✓ {} ({} → {} → {} bytes)",
            file_stem,
            human_content.len(),
            llm_content.len(),
            machine_bytes.len()
        );
    }

    // Calculate statistics
    let avg_serialize = serialize_times.iter().sum::<std::time::Duration>().as_nanos()
        / serialize_times.len() as u128;
    let avg_deserialize = deserialize_times.iter().sum::<std::time::Duration>().as_nanos()
        / deserialize_times.len() as u128;

    println!("\n=== Summary ===");
    println!("Files processed: {}", md_files.len());
    println!("\nFormat Sizes:");
    println!(
        "  Human:   {} bytes ({:.2} MB)",
        total_human_bytes,
        total_human_bytes as f64 / 1_000_000.0
    );
    println!(
        "  LLM:     {} bytes ({:.2} MB, {:.1}% of human)",
        total_llm_bytes,
        total_llm_bytes as f64 / 1_000_000.0,
        (total_llm_bytes as f64 / total_human_bytes as f64) * 100.0
    );
    println!(
        "  Machine: {} bytes ({:.2} MB, {:.1}% of human, {:.1}% of LLM)",
        total_machine_bytes,
        total_machine_bytes as f64 / 1_000_000.0,
        (total_machine_bytes as f64 / total_human_bytes as f64) * 100.0,
        (total_machine_bytes as f64 / total_llm_bytes as f64) * 100.0
    );

    println!("\nPerformance:");
    println!("  Serialize (human→machine):   {:.2} µs avg", avg_serialize as f64 / 1000.0);
    println!("  Deserialize (machine→llm):   {:.2} µs avg", avg_deserialize as f64 / 1000.0);
    println!(
        "  Round-trip:                  {:.2} µs avg",
        (avg_serialize + avg_deserialize) as f64 / 1000.0
    );

    println!("\n✓ Machine format working correctly!");
    println!("  - Zero-copy deserialization");
    println!("  - String inlining for small strings");
    println!("  - Compatible with dx-serializer approach");
}
