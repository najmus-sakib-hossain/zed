use dx_markdown::human_formatter::HumanFormatter;
use dx_markdown::markdown::MarkdownParser;

fn main() {
    let md = r#"t:2(Name,Age)[Alice,30;Bob,25]"#;

    let doc = MarkdownParser::parse(md).unwrap();

    eprintln!("Parsed {} nodes", doc.nodes.len());
    for (i, node) in doc.nodes.iter().enumerate() {
        eprintln!("Node {}: {:?}", i, std::mem::discriminant(node));
    }

    let mut formatter = HumanFormatter::new();
    let human = formatter.format(&doc);

    println!("{}", human);
}
