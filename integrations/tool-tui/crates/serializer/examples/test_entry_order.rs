use serializer::llm_to_document;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = fs::read_to_string("../../dx.llm")?;
    let doc = llm_to_document(&llm)?;

    println!("=== Entry Order from dx.llm ===");
    for (i, entry_ref) in doc.entry_order.iter().enumerate() {
        match entry_ref {
            serializer::llm::types::EntryRef::Context(key) => {
                println!("  {}: Context({})", i, key);
            }
            serializer::llm::types::EntryRef::Section(id) => {
                let name = doc.section_names.get(id).map(|s| s.as_str()).unwrap_or("?");
                println!("  {}: Section('{}' = {})", i, id, name);
            }
        }
    }

    println!("\n=== Context Keys (in IndexMap order) ===");
    for (i, key) in doc.context.keys().enumerate() {
        println!("  {}: {}", i, key);
    }

    println!("\n=== Section IDs (in IndexMap order) ===");
    for (i, (id, _)) in doc.sections.iter().enumerate() {
        let name = doc.section_names.get(id).map(|s| s.as_str()).unwrap_or("?");
        println!("  {}: '{}' = {}", i, id, name);
    }

    Ok(())
}
