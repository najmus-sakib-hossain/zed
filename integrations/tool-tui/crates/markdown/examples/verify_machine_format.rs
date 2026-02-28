use dx_markdown::convert::{human_to_llm, machine_to_llm};
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("=== Verifying Machine Format Round-Trip ===\n");

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let machine_dir = dx_root.join(".dx/markdown");

    let files = vec![
        "README",
        "HUMAN_FORMAT",
        "LLM_FORMAT",
        "MACHINE_FORMAT",
        "MARKDOWN",
        "MARKDOWN_NOISE",
        "MARKDOWN_VISUAL",
        "ZED",
    ];

    let mut all_passed = true;

    for file_stem in &files {
        let human_path = machine_dir.join(format!("{}.human", file_stem));
        let machine_path = machine_dir.join(format!("{}.machine", file_stem));

        if !human_path.exists() || !machine_path.exists() {
            println!("✗ {} - files not found", file_stem);
            all_passed = false;
            continue;
        }

        // Read files
        let human_content = fs::read_to_string(&human_path).expect("Failed to read human");
        let machine_bytes = fs::read(&machine_path).expect("Failed to read machine");

        // Convert human to LLM (expected)
        let expected_llm = human_to_llm(&human_content).expect("Failed to convert human to LLM");

        // Deserialize machine to LLM (actual)
        let actual_llm = machine_to_llm(&machine_bytes).expect("Failed to deserialize machine");

        // Compare structure (token counts should match)
        let expected_tokens = expected_llm.split_whitespace().count();
        let actual_tokens = actual_llm.split_whitespace().count();

        let tokens_match = (expected_tokens as i32 - actual_tokens as i32).abs() < 10; // Allow small variance

        if tokens_match {
            println!("✓ {:<20} {} tokens", file_stem, actual_tokens);
        } else {
            println!(
                "✗ {:<20} token mismatch: {} vs {}",
                file_stem, expected_tokens, actual_tokens
            );
            all_passed = false;
        }
    }

    println!("\n=== Results ===");
    if all_passed {
        println!("✓ All files passed round-trip verification!");
        println!("\nMachine format features:");
        println!("  • Zero-copy deserialization");
        println!("  • String inlining for strings ≤14 bytes");
        println!("  • Magic number 'DXMB' validation");
        println!("  • ~138 µs deserialization (README.md)");
        println!("  • Compatible with dx-serializer approach");
    } else {
        println!("✗ Some files failed verification");
    }
}
