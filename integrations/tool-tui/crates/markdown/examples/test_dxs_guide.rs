use dx_markdown::convert::human_to_llm;
use std::fs;

fn main() {
    let path = "../../crates/check/docs/DXS_FILES_GUIDE.md";

    println!("Reading {}...", path);
    let content = fs::read_to_string(path).expect("Failed to read file");
    println!("File size: {} bytes, {} lines", content.len(), content.lines().count());

    println!("\nConverting to LLM format...");
    let llm = match human_to_llm(&content) {
        Ok(l) => {
            println!("✓ LLM conversion successful: {} bytes", l.len());

            // Save LLM output for debugging
            fs::write("../../crates/markdown/debug_llm_output.txt", &l).ok();

            l
        }
        Err(e) => {
            eprintln!("✗ LLM conversion failed: {:?}", e);
            return;
        }
    };

    println!("\nSkipping machine conversion (causes crash)");
    println!("LLM output saved to debug_llm_output.txt");
}
