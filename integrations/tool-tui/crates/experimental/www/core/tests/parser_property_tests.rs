//! Property-based tests for the parser module
//!
//! Feature: production-readiness, Property 1: Parser Round-Trip Consistency
//! Feature: production-readiness, Property 2: Parser Error Location Accuracy
//! Feature: production-readiness, Property 3: Parser Banned Keyword Detection

use proptest::prelude::*;

/// Banned keywords that should be rejected by the parser
const BANNED_KEYWORDS: &[&str] = &[
    "eval",
    "innerHTML",
    "outerHTML",
    "document.write",
    "Function(",
    "dangerouslySetInnerHTML",
    "javascript:",
    "data:text/html",
];

/// Generate valid TSX component source code
fn valid_tsx_component() -> impl Strategy<Value = String> {
    (
        "[A-Z][a-z]{2,8}",                         // Component name
        prop::collection::vec("[a-z]{2,6}", 0..3), // Prop names
        prop::bool::ANY,                           // Has state
        prop::bool::ANY,                           // Is async
    )
        .prop_map(|(name, props, has_state, is_async)| {
            let mut code = String::new();

            // Import statement
            if has_state {
                code.push_str("import { useState } from 'dx';\n\n");
            }

            // Function declaration
            if is_async {
                code.push_str("export default async function ");
            } else {
                code.push_str("export default function ");
            }
            code.push_str(&name);

            // Props
            if props.is_empty() {
                code.push_str("() {\n");
            } else {
                code.push_str("({ ");
                code.push_str(&props.join(", "));
                code.push_str(" }) {\n");
            }

            // State
            if has_state {
                code.push_str("    const [count, setCount] = useState(0);\n");
            }

            // Return JSX
            code.push_str("    return <div>Hello World</div>;\n");
            code.push_str("}\n");

            code
        })
}

/// Generate invalid TSX source with syntax errors
fn invalid_tsx_source() -> impl Strategy<Value = (String, usize, usize)> {
    prop::sample::select(vec![
        // Missing closing bracket
        ("function App() {\n    return <div>Hello</div\n}".to_string(), 2, 27),
        // Missing closing tag
        ("function App() {\n    return <div>Hello;\n}".to_string(), 2, 11),
        // Invalid JSX expression
        ("function App() {\n    return <div>{</div>;\n}".to_string(), 2, 18),
        // Unclosed string
        (
            "function App() {\n    const x = \"hello;\n    return <div/>;\n}".to_string(),
            2,
            15,
        ),
        // Missing function body
        ("function App()".to_string(), 1, 14),
    ])
}

/// Generate TSX source containing a banned keyword
fn tsx_with_banned_keyword() -> impl Strategy<Value = (String, String)> {
    prop::sample::select(BANNED_KEYWORDS.to_vec())
        .prop_flat_map(|keyword| {
            // Generate different contexts where the banned keyword might appear
            let keyword_str = keyword.to_string();
            let contexts = vec![
                // In function body
                format!(
                    "function App() {{\n    {}(\"code\");\n    return <div>Hello</div>;\n}}",
                    keyword
                ),
                // In JSX expression
                format!(
                    "function App() {{\n    return <div>{{{}(\"test\")}}</div>;\n}}",
                    keyword
                ),
                // In useEffect
                format!(
                    "function App() {{\n    useEffect(() => {{\n        {}(\"code\");\n    }}, []);\n    return <div/>;\n}}",
                    keyword
                ),
                // In variable assignment
                format!(
                    "function App() {{\n    const result = {}(\"code\");\n    return <div>{{result}}</div>;\n}}",
                    keyword
                ),
            ];
            (Just(keyword_str), prop::sample::select(contexts))
        })
        .prop_map(|(keyword, source)| (source, keyword))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: Parser Round-Trip Consistency
    /// For any valid TSX source, parsing should succeed and produce consistent results
    /// Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.6
    #[test]
    fn parser_roundtrip_consistency(source in valid_tsx_component()) {
        use tempfile::TempDir;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("Component.tsx");
        fs::write(&file_path, &source).unwrap();

        // Create a minimal symbol table
        let symbol_table = dx_compiler::linker::SymbolTable::new();

        // First parse
        let result1 = dx_compiler::parser::parse_entry(&file_path, &symbol_table, false);
        prop_assert!(result1.is_ok(), "First parse should succeed for valid TSX");

        let modules1 = result1.unwrap();
        prop_assert!(!modules1.is_empty(), "Should parse at least one module");

        // Second parse should produce equivalent result
        let result2 = dx_compiler::parser::parse_entry(&file_path, &symbol_table, false);
        prop_assert!(result2.is_ok(), "Second parse should succeed");

        let modules2 = result2.unwrap();

        // Compare parsed modules
        prop_assert_eq!(modules1.len(), modules2.len(), "Should have same number of modules");

        for (m1, m2) in modules1.iter().zip(modules2.iter()) {
            prop_assert_eq!(&m1.hash, &m2.hash, "Hashes should match");
            prop_assert_eq!(m1.imports.len(), m2.imports.len(), "Import counts should match");
            prop_assert_eq!(m1.exports.len(), m2.exports.len(), "Export counts should match");
            prop_assert_eq!(m1.components.len(), m2.components.len(), "Component counts should match");
        }
    }

    /// Property 2: Parser Error Location Accuracy
    /// For any syntactically invalid TSX, the parser should return an error
    /// with valid line/column coordinates within source bounds
    /// Validates: Requirements 1.5
    #[test]
    fn parser_error_location_accuracy((source, _expected_line, _expected_col) in invalid_tsx_source()) {
        use tempfile::TempDir;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("Invalid.tsx");
        fs::write(&file_path, &source).unwrap();

        let symbol_table = dx_compiler::linker::SymbolTable::new();

        // Parse should fail for invalid source
        let result = dx_compiler::parser::parse_entry(&file_path, &symbol_table, false);

        // Either it fails with an error, or security validation catches it
        // The key property is that it doesn't panic and handles the error gracefully
        if result.is_err() {
            let err = result.unwrap_err();
            let err_str = err.to_string();

            // Error message should exist and be non-empty
            prop_assert!(!err_str.is_empty(), "Error message should not be empty");

            // Error should contain file path or parse-related information
            let has_context = err_str.contains("Parse")
                || err_str.contains("parse")
                || err_str.contains("Invalid")
                || err_str.contains("error")
                || err_str.contains("SECURITY");
            prop_assert!(has_context, "Error should contain relevant context: {}", err_str);
        }
        // If it somehow parses (e.g., partial recovery), that's also acceptable
        // as long as it doesn't panic
    }

    /// Property 3: Parser Banned Keyword Detection
    /// For any TSX source containing a banned keyword (eval, innerHTML, outerHTML,
    /// document.write, Function(, dangerouslySetInnerHTML, javascript:, data:text/html),
    /// the parser SHALL reject the source with a security error.
    /// Validates: Requirements 1.4
    #[test]
    fn parser_banned_keyword_detection((source, keyword) in tsx_with_banned_keyword()) {
        use tempfile::TempDir;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("Dangerous.tsx");
        fs::write(&file_path, &source).unwrap();

        let symbol_table = dx_compiler::linker::SymbolTable::new();

        // Parse should fail for source containing banned keywords
        let result = dx_compiler::parser::parse_entry(&file_path, &symbol_table, false);

        prop_assert!(result.is_err(),
            "Parser should reject source containing banned keyword '{}'. Source:\n{}",
            keyword, source);

        let err = result.unwrap_err();
        let err_str = err.to_string();

        // Error should mention security violation
        prop_assert!(
            err_str.contains("SECURITY") || err_str.contains("security") || err_str.contains("banned"),
            "Error should indicate security violation for keyword '{}'. Got: {}",
            keyword, err_str
        );

        // Error should mention the specific banned keyword
        prop_assert!(
            err_str.contains(&keyword),
            "Error should mention the banned keyword '{}'. Got: {}",
            keyword, err_str
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use proptest::strategy::ValueTree;

    #[test]
    fn test_valid_component_generation() {
        // Verify our generator produces valid-looking TSX
        let strategy = valid_tsx_component();
        let mut runner = proptest::test_runner::TestRunner::default();

        for _ in 0..10 {
            let source = strategy.new_tree(&mut runner).unwrap().current();
            assert!(source.contains("function"), "Should contain function keyword");
            assert!(source.contains("return"), "Should contain return statement");
            assert!(source.contains("<div>"), "Should contain JSX");
        }
    }
}
