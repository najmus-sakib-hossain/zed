use serializer::{document_to_llm, human_to_document};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let human = fs::read_to_string("../../dx.human")?;
    let doc = human_to_document(&human)?;
    let llm = document_to_llm(&doc);
    fs::write("../../dx_from_human.llm", &llm)?;

    println!("✓ Converted dx.human → dx_from_human.llm");
    println!("\n{}", llm);
    Ok(())
}
