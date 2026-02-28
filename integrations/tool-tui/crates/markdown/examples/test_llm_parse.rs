use std::fs;

fn main() {
    let llm_file = "debug___crates_check_docs_DXS_FILES_GUIDE.md.llm";

    println!("Reading LLM file...");
    let llm = fs::read_to_string(llm_file).expect("Failed to read LLM file");
    println!("LLM size: {} bytes", llm.len());

    println!("\nParsing LLM format...");
    use dx_markdown::parser::DxmParser;
    let doc = match DxmParser::parse(&llm) {
        Ok(d) => {
            println!("✓ Parsed successfully");
            println!("  Nodes: {}", d.nodes.len());
            println!("  Refs: {}", d.refs.len());
            d
        }
        Err(e) => {
            eprintln!("✗ Parse failed: {:?}", e);
            return;
        }
    };

    // Inspect nodes
    println!("\nInspecting nodes:");
    for (i, node) in doc.nodes.iter().enumerate() {
        use dx_markdown::types::DxmNode;
        match node {
            DxmNode::Table(t) => {
                println!("  Node {}: Table - {} cols, {} rows", i, t.schema.len(), t.rows.len());
                if t.rows.len() > 1000 {
                    println!("    WARNING: Large table!");
                }
                // Check row sizes
                for (ri, row) in t.rows.iter().enumerate() {
                    if row.len() != t.schema.len() {
                        println!(
                            "    WARNING: Row {} has {} cells but schema has {} cols",
                            ri,
                            row.len(),
                            t.schema.len()
                        );
                    }
                    if row.len() > 1000 {
                        println!("    WARNING: Row {} has {} cells!", ri, row.len());
                    }
                }
            }
            DxmNode::List(l) => {
                println!("  Node {}: List - {} items", i, l.items.len());
                if l.items.len() > 1000 {
                    println!("    WARNING: Large list!");
                }
            }
            DxmNode::Header(h) => {
                println!("  Node {}: Header level {}", i, h.level);
            }
            DxmNode::Paragraph(p) => {
                println!("  Node {}: Paragraph - {} inlines", i, p.len());
                if p.len() > 1000 {
                    println!("    WARNING: Large paragraph!");
                }
            }
            DxmNode::CodeBlock(_) => {
                println!("  Node {}: CodeBlock", i);
            }
            _ => {
                println!("  Node {}: {:?}", i, node);
            }
        }
    }

    println!("\nNow attempting binary conversion...");
    use dx_markdown::binary::BinaryBuilder;
    match std::panic::catch_unwind(|| BinaryBuilder::build(&doc)) {
        Ok(Ok(binary)) => {
            println!("✓ Binary conversion successful: {} bytes", binary.len());
        }
        Ok(Err(e)) => {
            eprintln!("✗ Binary conversion error: {}", e);
        }
        Err(e) => {
            eprintln!("✗ Binary conversion panicked: {:?}", e);
        }
    }
}
