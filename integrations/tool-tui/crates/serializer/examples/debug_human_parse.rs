use serializer::human_to_document;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let human = fs::read_to_string("../../dx.human")?;
    let doc = human_to_document(&human)?;

    println!("=== Parsed Document ===");
    println!("Context keys: {:?}", doc.context.keys().collect::<Vec<_>>());
    println!("Section IDs: {:?}", doc.sections.keys().collect::<Vec<_>>());
    println!("Section names: {:?}", doc.section_names);

    println!("\n=== Entry Order ===");
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

    println!("\n=== Sections ===");
    for (id, section) in &doc.sections {
        let name = doc.section_names.get(id).map(|s| s.as_str()).unwrap_or("?");
        println!("Section '{}' ({}):", id, name);
        println!("  Schema: {:?}", section.schema);
        println!("  Rows: {}", section.rows.len());
    }

    Ok(())
}
