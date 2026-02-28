use dx_markdown::convert::{human_to_llm, llm_to_machine};

fn main() {
    // Minimal table that might trigger the bug
    let markdown = r#"
# Test

| Col1 | Col2 | Col3 | Col4 |
|------|------|------|------|
| A | B | C | D |
| E | F | G | H |
"#;

    println!("Converting to LLM...");
    let llm = human_to_llm(markdown).expect("LLM conversion failed");
    println!("LLM output ({} bytes):\n{}", llm.len(), llm);

    println!("\nConverting to machine...");
    match llm_to_machine(&llm) {
        Ok(machine) => {
            println!("Machine output: {} bytes", machine.len());
        }
        Err(e) => {
            eprintln!("Machine conversion failed: {:?}", e);
        }
    }
}
