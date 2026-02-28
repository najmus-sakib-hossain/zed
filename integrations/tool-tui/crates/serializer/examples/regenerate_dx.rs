use serializer::{document_to_human, llm_to_document};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = fs::read_to_string("../../dx.llm")?;
    let doc = llm_to_document(&llm)?;
    let human = document_to_human(&doc);
    fs::write("../../dx.human", human)?;

    println!("✓ Regenerated dx.human from dx.llm");
    Ok(())
}
