use serializer::llm::human_parser::HumanParser;
use serializer::llm::serializer::LlmSerializer;

#[test]
fn test_dots_in_section_names() {
    let input = r#"
[js.dependencies]
next = 16.0.1
react = 19.0.1

[python.dependencies]
django = latest
numpy = latest
"#;

    let parser = HumanParser::new();
    let doc = parser.parse(input).unwrap();

    // Check context has dots preserved
    assert!(doc.context.contains_key("js.dependencies"));
    assert!(doc.context.contains_key("python.dependencies"));

    let serializer = LlmSerializer::new();
    let output = serializer.serialize(&doc);

    println!("Output:\n{}", output);

    // Check output has dots preserved
    assert!(
        output.contains("js.dependencies"),
        "Output should contain 'js.dependencies' but got:\n{}",
        output
    );
    assert!(
        output.contains("python.dependencies"),
        "Output should contain 'python.dependencies' but got:\n{}",
        output
    );
    assert!(
        !output.contains("js_dependencies"),
        "Output should NOT contain 'js_dependencies'"
    );
    assert!(
        !output.contains("python_dependencies"),
        "Output should NOT contain 'python_dependencies'"
    );
}
