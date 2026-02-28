use std::fs;

fn main() {
    let content =
        fs::read_to_string("../../.github/copilot-instructions.md").expect("Failed to read file");

    println!("File size: {} bytes", content.len());
    println!("Lines: {}", content.lines().count());

    println!("\nConverting to LLM format...");
    use dx_markdown::convert::human_to_llm;

    match human_to_llm(&content) {
        Ok(llm) => {
            println!("✓ Success!");
            println!("LLM size: {} bytes", llm.len());
            println!("First 500 chars: {}", &llm[..llm.len().min(500)]);
        }
        Err(e) => {
            println!("✗ Error: {:?}", e);
        }
    }
}
