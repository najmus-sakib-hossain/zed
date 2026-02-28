use dx_markdown::convert::doc_to_human;
use dx_markdown::markdown::MarkdownParser;

fn main() {
    let markdown = r#"# Test Header

This is a paragraph.

## Second Header

Another paragraph.
"#;

    // Parse markdown directly
    let doc = MarkdownParser::parse(markdown).unwrap();

    // Convert to human format
    let human = doc_to_human(&doc);

    println!("{}", human);
}
