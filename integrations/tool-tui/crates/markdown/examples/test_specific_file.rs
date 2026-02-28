use dx_markdown::convert::{human_to_llm, llm_to_machine};
use std::fs;

fn main() {
    let files = vec![
        "../../crates/check/docs/DXS_FILES_GUIDE.md",
        "../../crates/check/docs/DXS_FORMAT_SPEC.md",
        "../../crates/markdown/README.md",
    ];

    for file in files {
        println!("\n=== Testing {} ===", file);

        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read: {}", e);
                continue;
            }
        };

        println!("File size: {} bytes", content.len());

        println!("Converting to LLM...");
        let llm = match human_to_llm(&content) {
            Ok(l) => {
                println!("✓ LLM: {} bytes", l.len());
                l
            }
            Err(e) => {
                eprintln!("✗ LLM failed: {}", e);
                continue;
            }
        };

        // Save LLM for inspection
        let debug_file = format!(
            "debug_{}.llm",
            file.replace("/", "_").replace("\\", "_").replace("..", "").replace(":", "")
        );
        fs::write(&debug_file, &llm).ok();
        println!("Saved LLM to {}", debug_file);

        println!("Converting to machine...");
        match llm_to_machine(&llm) {
            Ok(m) => {
                println!("✓ Machine: {} bytes", m.len());
            }
            Err(e) => {
                eprintln!("✗ Machine failed: {}", e);
            }
        }
    }
}
