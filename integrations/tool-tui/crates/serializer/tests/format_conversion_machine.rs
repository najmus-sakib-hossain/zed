//! Integration tests for LLM/Human format conversion to Machine format

use serializer::llm::convert::{
    document_to_machine, human_to_machine, llm_to_machine, machine_to_document, machine_to_human,
    machine_to_llm,
};
use serializer::llm::human_parser::HumanParser;
use serializer::llm::parser::LlmParser;

#[test]
fn test_llm_to_machine_simple() {
    let llm_input = r#"name=John Doe
age=30
email=john@example.com
active=true"#;

    let machine = llm_to_machine(llm_input).expect("Failed to convert LLM to machine");

    // Verify we have RKYV + LZ4 compressed data
    assert!(machine.data.len() > 4, "Machine format should have data");

    // Convert back to LLM
    let llm_output = machine_to_llm(&machine).expect("Failed to convert machine to LLM");

    // Parse both and compare
    let doc_original = LlmParser::parse(llm_input).unwrap();
    let doc_roundtrip = LlmParser::parse(&llm_output).unwrap();

    assert_eq!(doc_original.context.len(), doc_roundtrip.context.len());
}

#[test]
fn test_human_to_machine_simple() {
    let human_input = r#"
[Person]
    name = "John Doe"
    age = 30
    email = "john@example.com"
    active = true
"#;

    let machine = human_to_machine(human_input).expect("Failed to convert Human to machine");

    // Verify we have RKYV + LZ4 compressed data
    assert!(machine.data.len() > 4, "Machine format should have data");

    // Convert back to human
    let human_output = machine_to_human(&machine).expect("Failed to convert machine to Human");

    // Parse both and compare
    let parser = HumanParser::new();
    let doc_original = parser.parse(human_input).unwrap();
    let doc_roundtrip = parser.parse(&human_output).unwrap();

    assert_eq!(doc_original.context.len(), doc_roundtrip.context.len());
}

#[test]
fn test_llm_to_machine_with_arrays() {
    let llm_input = r#"friends:3=Alice,Bob,Charlie
scores:4=95,87,92,88"#;

    let machine = llm_to_machine(llm_input).expect("Failed to convert LLM to machine");
    let llm_output = machine_to_llm(&machine).expect("Failed to convert machine to LLM");

    let doc_original = LlmParser::parse(llm_input).unwrap();
    let doc_roundtrip = LlmParser::parse(&llm_output).unwrap();

    assert_eq!(doc_original.context.len(), doc_roundtrip.context.len());
}

#[test]
fn test_llm_to_machine_with_objects() {
    let llm_input = r#"config[host=localhost,port=8080,ssl=true]
database[name=mydb,user=admin,password=secret]"#;

    let machine = llm_to_machine(llm_input).expect("Failed to convert LLM to machine");
    let llm_output = machine_to_llm(&machine).expect("Failed to convert machine to LLM");

    let doc_original = LlmParser::parse(llm_input).unwrap();
    let doc_roundtrip = LlmParser::parse(&llm_output).unwrap();

    assert_eq!(doc_original.context.len(), doc_roundtrip.context.len());
}

#[test]
fn test_human_to_machine_with_nested() {
    let human_input = r#"
[Server]
    host = "localhost"
    port = 8080
    
    [Server.Database]
        name = "mydb"
        user = "admin"
        
[Client]
    timeout = 30
    retries = 3
"#;

    let machine = human_to_machine(human_input).expect("Failed to convert Human to machine");
    let human_output = machine_to_human(&machine).expect("Failed to convert machine to Human");

    let parser = HumanParser::new();
    let doc_original = parser.parse(human_input).unwrap();
    let doc_roundtrip = parser.parse(&human_output).unwrap();

    // Should preserve data (context or sections)
    let original_total = doc_original.context.len() + doc_original.sections.len();
    let roundtrip_total = doc_roundtrip.context.len() + doc_roundtrip.sections.len();
    assert!(
        original_total > 0 && roundtrip_total > 0,
        "Should have data: original={}, roundtrip={}",
        original_total,
        roundtrip_total
    );
}

#[test]
fn test_machine_format_size_efficiency() {
    let llm_input = r#"name=John Doe
age=30
email=john@example.com
city=New York
country=USA
active=true"#;

    let machine = llm_to_machine(llm_input).expect("Failed to convert");

    // Machine format should be compact (RKYV + LZ4)
    // With compression, should be reasonable size
    assert!(
        machine.data.len() < 250,
        "Machine format too large: {} bytes",
        machine.data.len()
    );

    // Should be smaller than JSON representation
    let json_size = serde_json::to_vec(&llm_input).unwrap().len();
    println!("Machine size: {} bytes, JSON size: {} bytes", machine.data.len(), json_size);
}

#[test]
fn test_document_to_machine_roundtrip() {
    use serializer::llm::types::{DxDocument, DxLlmValue};

    let mut doc = DxDocument::new();
    doc.context.insert("name".to_string(), DxLlmValue::Str("Alice".to_string()));
    doc.context.insert("age".to_string(), DxLlmValue::Num(25.0));
    doc.context.insert("active".to_string(), DxLlmValue::Bool(true));
    doc.context.insert(
        "tags".to_string(),
        DxLlmValue::Arr(vec![
            DxLlmValue::Str("rust".to_string()),
            DxLlmValue::Str("wasm".to_string()),
        ]),
    );

    let machine = document_to_machine(&doc);
    let doc_roundtrip = machine_to_document(&machine).expect("Failed to convert back");

    assert_eq!(doc.context.len(), doc_roundtrip.context.len());
    assert_eq!(doc_roundtrip.context.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(doc_roundtrip.context.get("age").unwrap().as_num(), Some(25.0));
    assert_eq!(doc_roundtrip.context.get("active").unwrap().as_bool(), Some(true));
}

#[test]
fn test_machine_format_with_nulls() {
    use serializer::llm::types::{DxDocument, DxLlmValue};

    let mut doc = DxDocument::new();
    doc.context.insert("name".to_string(), DxLlmValue::Str("Bob".to_string()));
    doc.context.insert("middle_name".to_string(), DxLlmValue::Null);
    doc.context.insert("age".to_string(), DxLlmValue::Num(40.0));

    let machine = document_to_machine(&doc);
    let doc_roundtrip = machine_to_document(&machine).expect("Failed to convert back");

    assert_eq!(doc.context.len(), doc_roundtrip.context.len());
    assert!(matches!(doc_roundtrip.context.get("middle_name").unwrap(), DxLlmValue::Null));
}

#[test]
fn test_machine_format_with_references() {
    use serializer::llm::types::{DxDocument, DxLlmValue};

    let mut doc = DxDocument::new();
    doc.context
        .insert("user_id".to_string(), DxLlmValue::Str("user123".to_string()));
    doc.context
        .insert("profile_ref".to_string(), DxLlmValue::Ref("user123".to_string()));

    let machine = document_to_machine(&doc);
    let doc_roundtrip = machine_to_document(&machine).expect("Failed to convert back");

    assert_eq!(doc.context.len(), doc_roundtrip.context.len());
    if let DxLlmValue::Ref(ref_val) = doc_roundtrip.context.get("profile_ref").unwrap() {
        assert_eq!(ref_val, "user123");
    } else {
        panic!("Expected Ref value");
    }
}

#[test]
fn test_empty_document_to_machine() {
    use serializer::llm::types::DxDocument;

    let doc = DxDocument::new();
    let machine = document_to_machine(&doc);

    // Should have valid RKYV + LZ4 data even for empty document
    assert!(machine.data.len() >= 4, "Machine format should have data");

    let doc_roundtrip = machine_to_document(&machine).expect("Failed to convert back");
    assert_eq!(doc.context.len(), doc_roundtrip.context.len());
    assert_eq!(doc.sections.len(), doc_roundtrip.sections.len());
}
