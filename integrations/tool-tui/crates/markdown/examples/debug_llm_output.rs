use std::fs;

fn main() {
    println!("Debugging LLM output from markdown compiler\n");

    let content = fs::read_to_string("../../crates/check/README.md").expect("Failed to read file");

    // Convert to LLM format
    use dx_markdown::convert::human_to_llm;

    match human_to_llm(&content) {
        Ok(llm) => {
            println!("LLM format (first 2000 chars):");
            println!("{}", &llm[..llm.len().min(2000)]);

            // Save to file for inspection
            fs::write("debug_llm_output.txt", &llm).expect("Failed to write file");
            println!("\n✓ Full output saved to debug_llm_output.txt");
        }
        Err(e) => {
            println!("✗ Conversion failed: {:?}", e);
        }
    }
}
