//! Unit tests for anti-pattern detection

use dx_check::anti_pattern::{AntiPatternConfig, AntiPatternDetector};
use std::path::{Path, PathBuf};

#[test]
fn test_detect_god_object_with_many_methods() {
    let detector = AntiPatternDetector::with_default_config();

    // Create a class with 25 methods (exceeds default threshold of 20)
    let methods: Vec<String> =
        (0..25).map(|i| format!("  method{}() {{ return {}; }}", i, i)).collect();

    let source = format!("class GodClass {{\n{}\n}}", methods.join("\n"));

    eprintln!("Source:\n{}", source);

    let path = Path::new("god_class.js");
    let diagnostics = detector.detect(path, &source);

    eprintln!("Diagnostics: {:?}", diagnostics);

    // Should detect god object
    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(!god_objects.is_empty(), "Should detect god object with many methods");
    assert!(god_objects[0].message.contains("GodClass"));
}

#[test]
fn test_detect_god_object_with_many_fields() {
    let detector = AntiPatternDetector::with_default_config();

    // Create a class with 20 fields (exceeds default threshold of 15)
    let fields: Vec<String> = (0..20).map(|i| format!("  field{}: number;", i)).collect();

    let source = format!("class DataBlob {{\n{}\n}}", fields.join("\n"));

    let path = Path::new("data_blob.ts");
    let diagnostics = detector.detect(path, &source);

    // Should detect god object
    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(!god_objects.is_empty(), "Should detect god object with many fields");
}

#[test]
fn test_detect_god_object_combined_violations() {
    let detector = AntiPatternDetector::with_default_config();

    // Create a class with both many methods and many fields
    let fields: Vec<String> = (0..16).map(|i| format!("  private field{}: string;", i)).collect();

    let methods: Vec<String> = (0..22).map(|i| format!("  method{}() {{}}", i)).collect();

    let source = format!("class MegaClass {{\n{}\n\n{}\n}}", fields.join("\n"), methods.join("\n"));

    let path = Path::new("mega_class.java");
    let diagnostics = detector.detect(path, &source);

    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(!god_objects.is_empty(), "Should detect god object with multiple violations");
    assert!(god_objects[0].suggestion.is_some());
}

#[test]
fn test_no_god_object_for_small_class() {
    let detector = AntiPatternDetector::with_default_config();

    let source = r#"
class SmallClass {
  field1: string;
  field2: number;
  
  constructor(f1: string, f2: number) {
    this.field1 = f1;
    this.field2 = f2;
  }
  
  method1() {
    return this.field1;
  }
  
  method2() {
    return this.field2;
  }
}
"#;

    let path = Path::new("small_class.ts");
    let diagnostics = detector.detect(path, source);

    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(god_objects.is_empty(), "Should not detect god object for small class");
}

#[test]
fn test_detect_circular_dependency_simple() {
    let detector = AntiPatternDetector::with_default_config();

    let files = vec![
        (
            PathBuf::from("module_a.js"),
            "import { B } from './module_b.js';\nexport class A {}".to_string(),
        ),
        (
            PathBuf::from("module_b.js"),
            "import { A } from './module_a.js';\nexport class B {}".to_string(),
        ),
    ];

    let diagnostics = detector.detect_project(&files);

    let circular_deps: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "circular-dependency").collect();

    assert!(!circular_deps.is_empty(), "Should detect circular dependency");
    assert!(circular_deps[0].message.contains("Circular dependency"));
}

#[test]
fn test_detect_circular_dependency_chain() {
    let detector = AntiPatternDetector::with_default_config();

    let files = vec![
        (
            PathBuf::from("a.js"),
            "import { B } from './b.js';\nexport const A = 1;".to_string(),
        ),
        (
            PathBuf::from("b.js"),
            "import { C } from './c.js';\nexport const B = 2;".to_string(),
        ),
        (
            PathBuf::from("c.js"),
            "import { A } from './a.js';\nexport const C = 3;".to_string(),
        ),
    ];

    let diagnostics = detector.detect_project(&files);

    let circular_deps: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "circular-dependency").collect();

    assert!(!circular_deps.is_empty(), "Should detect circular dependency chain");
}

#[test]
fn test_no_circular_dependency_for_acyclic_graph() {
    let detector = AntiPatternDetector::with_default_config();

    let files = vec![
        (
            PathBuf::from("a.js"),
            "import { B } from './b.js';\nexport const A = 1;".to_string(),
        ),
        (
            PathBuf::from("b.js"),
            "import { C } from './c.js';\nexport const B = 2;".to_string(),
        ),
        (PathBuf::from("c.js"), "export const C = 3;".to_string()),
    ];

    let diagnostics = detector.detect_project(&files);

    let circular_deps: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "circular-dependency").collect();

    assert!(
        circular_deps.is_empty(),
        "Should not detect circular dependency in acyclic graph"
    );
}

#[test]
fn test_detect_tight_coupling_many_dependencies() {
    let detector = AntiPatternDetector::with_default_config();

    // Create a file with 15 imports (exceeds default threshold of 10)
    let imports: Vec<String> = (0..15)
        .map(|i| format!("import {{ Thing{} }} from './module{}.js';", i, i))
        .collect();

    let source = imports.join("\n");
    let files = vec![(PathBuf::from("main.js"), source)];

    let diagnostics = detector.detect_project(&files);

    let coupling: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "tight-coupling").collect();

    assert!(!coupling.is_empty(), "Should detect tight coupling with many dependencies");
    assert!(coupling[0].message.contains("dependencies"));
}

#[test]
fn test_detect_tight_coupling_excessive_imports_from_single_module() {
    let detector = AntiPatternDetector::with_default_config();

    // Create a file with 8 imports from the same module (exceeds default threshold of 5)
    let imports: Vec<String> =
        (0..8).map(|i| format!("import {{ Thing{} }} from './utils.js';", i)).collect();

    let source = imports.join("\n");
    let files = vec![(PathBuf::from("main.js"), source)];

    let diagnostics = detector.detect_project(&files);

    let coupling: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "tight-coupling").collect();

    assert!(
        !coupling.is_empty(),
        "Should detect tight coupling with excessive imports from single module"
    );
}

#[test]
fn test_no_tight_coupling_for_reasonable_dependencies() {
    let detector = AntiPatternDetector::with_default_config();

    let source = r#"
import { useState } from 'react';
import { useRouter } from 'next/router';
import { api } from './api';
import { utils } from './utils';

export function Component() {
  return <div>Hello</div>;
}
"#;

    let files = vec![(PathBuf::from("component.jsx"), source.to_string())];

    let diagnostics = detector.detect_project(&files);

    let coupling: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "tight-coupling").collect();

    assert!(
        coupling.is_empty(),
        "Should not detect tight coupling for reasonable dependencies"
    );
}

#[test]
fn test_custom_config_thresholds() {
    let config = AntiPatternConfig {
        max_methods: 5,
        max_fields: 3,
        max_class_lines: 100,
        max_dependencies: 3,
        max_imports_per_module: 2,
    };

    let detector = AntiPatternDetector::new(config);

    // Class with 6 methods should trigger with custom threshold
    let methods: Vec<String> = (0..6).map(|i| format!("  method{}() {{}}", i)).collect();

    let source = format!("class TestClass {{\n{}\n}}", methods.join("\n"));
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, &source);

    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(!god_objects.is_empty(), "Should detect god object with custom threshold");
}

#[test]
fn test_rust_struct_detection() {
    let detector = AntiPatternDetector::with_default_config();

    let fields: Vec<String> = (0..20).map(|i| format!("    field{}: i32,", i)).collect();

    let source = format!("pub struct BigStruct {{\n{}\n}}", fields.join("\n"));

    let path = Path::new("big_struct.rs");
    let diagnostics = detector.detect(path, &source);

    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(!god_objects.is_empty(), "Should detect god object in Rust struct");
}

#[test]
fn test_python_class_detection() {
    let detector = AntiPatternDetector::with_default_config();

    let methods: Vec<String> =
        (0..25).map(|i| format!("    def method{}(self):\n        pass", i)).collect();

    let source = format!("class BigClass:\n{}", methods.join("\n\n"));

    let path = Path::new("big_class.py");
    let diagnostics = detector.detect(path, &source);

    let god_objects: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "god-object").collect();

    assert!(!god_objects.is_empty(), "Should detect god object in Python class");
}

#[test]
fn test_refactoring_suggestions_provided() {
    let detector = AntiPatternDetector::with_default_config();

    let methods: Vec<String> = (0..25).map(|i| format!("  method{}() {{}}", i)).collect();

    let source = format!("class BigClass {{\n{}\n}}", methods.join("\n"));
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, &source);

    for diagnostic in diagnostics {
        assert!(
            diagnostic.suggestion.is_some(),
            "All anti-pattern diagnostics should have refactoring suggestions"
        );
    }
}
