use dx_markdown::convert::{human_to_llm, llm_to_machine};
use std::fs;

fn main() {
    let content = fs::read_to_string("../../crates/check/docs/DXS_FILES_GUIDE.md")
        .expect("Failed to read file");

    println!("File size: {} bytes", content.len());

    println!("\nConverting to LLM...");
    let llm = match human_to_llm(&content) {
        Ok(l) => {
            println!("LLM output: {} bytes", l.len());

            // Save for inspection
            fs::write("../../crates/markdown/debug_dxs_llm.txt", &l)
                .expect("Failed to write LLM file");
            l
        }
        Err(e) => {
            eprintln!("Error converting to LLM: {}", e);
            return;
        }
    };

    println!("\nConverting to machine...");
    match llm_to_machine(&llm) {
        Ok(machine) => {
            println!("Machine output: {} bytes", machine.len());
            fs::write("../../crates/markdown/debug_dxs_machine.bin", machine)
                .expect("Failed to write machine file");
        }
        Err(e) => {
            eprintln!("Error converting to machine: {}", e);
        }
    }
}
