//! Integration tests for dx-markdown
//!
//! These tests verify end-to-end workflows and real-world usage scenarios.

#![allow(clippy::expect_used, clippy::unwrap_used)] // Integration tests can use expect/unwrap

use markdown::{
    CompilerConfig, DxMarkdown, auto_parse, doc_to_human, doc_to_llm, human_to_llm, llm_to_human,
};

#[test]
fn test_end_to_end_compilation() {
    let input = r#"
# My Project

[![Build](https://img.shields.io/badge/build-passing-green.svg)](https://ci.example.com)

Check the [documentation](https://docs.example.com/very/long/url) for details.

| Feature | Status |
|---------|--------|
| Fast    | ✅     |
| Small   | ✅     |
"#;

    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");

    // Verify token reduction
    assert!(result.tokens_after < result.tokens_before, "Should reduce tokens");

    // Verify content preservation
    assert!(result.output.contains("My Project"));
    assert!(result.output.contains("Feature"));
    assert!(result.output.contains("Status"));
}

#[test]
fn test_format_round_trip_llm_human() {
    let llm_input = "1|Test Document\nThis is content.";

    // LLM → Human
    let human = llm_to_human(llm_input).expect("LLM to Human failed");
    assert!(human.contains("Test Document"));
    assert!(human.contains("content"));

    // Human → LLM
    let llm_back = human_to_llm(&human).expect("Human to LLM failed");
    assert!(llm_back.contains("Test Document"));
    assert!(llm_back.contains("content"));
}

#[test]
fn test_auto_parse_detection() {
    // Test LLM format detection
    let llm = b"1|Title\nContent";
    let doc = auto_parse(llm).expect("Auto parse LLM failed");
    assert_eq!(doc.nodes.len(), 2);

    // Test markdown parsing (not auto-parse, which expects specific formats)
    let markdown = "# Title\n\nContent";
    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result = compiler.compile(markdown).expect("Markdown compilation failed");
    assert!(result.output.contains("Title"));
}

#[test]
fn test_compiler_modes() {
    let input = r#"
# Documentation

This is some documentation with [links](https://example.com).

```rust
fn main() {
    println!("Hello");
}
```
"#;

    // Test code mode
    let config = CompilerConfig::code();
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");
    assert!(result.output.contains("main"));

    // Test docs mode
    let config = CompilerConfig::docs();
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");
    assert!(result.output.contains("Documentation"));
}

#[test]
fn test_table_conversion() {
    let input = r#"
| Name | Age |
|------|-----|
| Alice | 30 |
| Bob | 25 |
"#;

    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");

    // Should convert table to compact format
    assert!(result.tokens_after < result.tokens_before);
    assert!(result.output.contains("Alice"));
    assert!(result.output.contains("Bob"));
}

#[test]
fn test_url_stripping() {
    let input = "[docs](https://very-long-domain.com/path/to/documentation)";

    let config = CompilerConfig {
        strip_urls: true,
        ..Default::default()
    };
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");

    // Should strip https:// prefix
    assert!(!result.output.contains("https://"));
    assert!(result.output.contains("docs"));
}

#[test]
fn test_badge_removal() {
    let input =
        "[![Build](https://img.shields.io/badge/build-passing-green)](https://ci.example.com)";

    let config = CompilerConfig {
        strip_badges: true,
        ..Default::default()
    };
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");

    // Should remove badges
    assert!(result.output.is_empty() || !result.output.contains("shields.io"));
}

#[test]
fn test_code_minification() {
    let input = r#"
```javascript
// This is a comment
function hello() {
    console.log("Hello");  // Another comment
}
```
"#;

    let config = CompilerConfig {
        minify_code: true,
        ..Default::default()
    };
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");

    // Should reduce tokens by removing comments
    assert!(result.tokens_after < result.tokens_before);
}

#[test]
fn test_streaming_api() {
    use std::io::Cursor;

    let input = "# Title\n\nContent with [link](https://example.com)";
    let reader = Cursor::new(input.as_bytes());
    let mut output = Vec::new();

    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result = compiler
        .compile_streaming(reader, &mut output)
        .expect("Streaming compilation failed");

    assert!(result.tokens_after <= result.tokens_before);
    assert!(!output.is_empty());
}

#[test]
fn test_error_handling_input_too_large() {
    let huge_input = "x".repeat(101 * 1024 * 1024); // 101 MB

    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result = compiler.compile(&huge_input);

    assert!(result.is_err(), "Should fail on input too large");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("too large") || err_msg.contains("size"),
        "Error message should mention size: {}",
        err_msg
    );
}

#[test]
fn test_semantic_preservation() {
    let input = r#"
# Important Title

This is **critical** content that must be preserved.

- Item 1
- Item 2
- Item 3

> Important quote

[Essential link](https://example.com)
"#;

    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result = compiler.compile(input).expect("Compilation failed");

    // Verify all semantic content is preserved
    assert!(result.output.contains("Important Title"));
    assert!(result.output.contains("critical"));
    assert!(result.output.contains("Item 1"));
    assert!(result.output.contains("Item 2"));
    assert!(result.output.contains("Item 3"));
    assert!(result.output.contains("Important quote"));
    assert!(result.output.contains("Essential link"));
}

#[test]
fn test_idempotence() {
    let input = "# Title\n\nContent";

    let compiler = DxMarkdown::default_compiler().expect("Failed to create compiler");
    let result1 = compiler.compile(input).expect("First compilation failed");
    let result2 = compiler.compile(&result1.output).expect("Second compilation failed");

    // Second compilation should not reduce tokens further
    assert_eq!(result1.tokens_after, result2.tokens_after);
}

#[test]
fn test_format_conversions() {
    let llm = "1|Title\nContent";

    // LLM → Human → LLM
    let human = llm_to_human(llm).expect("LLM to Human failed");
    let llm_back = human_to_llm(&human).expect("Human to LLM failed");
    assert!(llm_back.contains("Title"));
    assert!(llm_back.contains("Content"));

    // Parse and convert
    let doc = auto_parse(llm.as_bytes()).expect("Parse failed");
    let human2 = doc_to_human(&doc);
    let llm2 = doc_to_llm(&doc);

    assert!(human2.contains("Title"));
    assert!(llm2.contains("Title"));
}
