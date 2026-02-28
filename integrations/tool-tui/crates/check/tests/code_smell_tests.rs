use dx_check::code_smell::{CodeSmellConfig, CodeSmellDetector, CodeSmellType};
use std::path::Path;

#[test]
fn test_detect_long_method() {
    let detector = CodeSmellDetector::with_default_config();

    // Create a function with 60 lines
    let mut source = String::from("function longFunction() {\n");
    for i in 0..60 {
        source.push_str(&format!("    console.log('line {}');\n", i));
    }
    source.push_str("}\n");

    let path = Path::new("test.js");
    let diagnostics = detector.detect(path, &source);

    // Should detect long method
    let long_method_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "long-method").collect();

    assert!(!long_method_diags.is_empty(), "Should detect long method");
}

#[test]
fn test_detect_too_many_parameters() {
    let detector = CodeSmellDetector::with_default_config();
    let source = "function test(a, b, c, d, e, f, g) { return a + b; }";
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, source);

    let param_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "too-many-parameters").collect();

    assert!(!param_diags.is_empty(), "Should detect too many parameters");
}

#[test]
fn test_detect_deep_nesting() {
    let detector = CodeSmellDetector::with_default_config();
    let source = r#"
        if (a) {
            if (b) {
                if (c) {
                    if (d) {
                        if (e) {
                            console.log('too deep');
                        }
                    }
                }
            }
        }
    "#;
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, source);

    let nesting_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "deep-nesting").collect();

    assert!(!nesting_diags.is_empty(), "Should detect deep nesting");
}

#[test]
fn test_detect_magic_number() {
    let detector = CodeSmellDetector::with_default_config();
    let source = "if (age > 18) { return true; }";
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, source);

    let magic_diags: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "magic-number").collect();

    assert!(!magic_diags.is_empty(), "Should detect magic number");
}

#[test]
fn test_magic_number_exceptions() {
    let detector = CodeSmellDetector::with_default_config();
    let source = "if (count > 0) { return true; }";
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, source);

    let magic_diags: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "magic-number").collect();

    // 0 is in exceptions, should not be detected
    assert!(magic_diags.is_empty(), "Should not detect 0 as magic number");
}

#[test]
fn test_detect_large_class() {
    let detector = CodeSmellDetector::with_default_config();

    // Create a class with 550 lines
    let mut source = String::from("class LargeClass {\n");
    for i in 0..550 {
        source.push_str(&format!("    method{}() {{ return {}; }}\n", i, i));
    }
    source.push_str("}\n");

    let path = Path::new("test.js");
    let diagnostics = detector.detect(path, &source);

    let class_diags: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "large-class").collect();

    assert!(!class_diags.is_empty(), "Should detect large class");
}

#[test]
fn test_code_smell_config() {
    let mut config = CodeSmellConfig::default();
    config.max_function_lines = 30;
    config.max_parameters = 3;

    let detector = CodeSmellDetector::new(config);

    // Test with custom thresholds
    let source = "function test(a, b, c, d) { return a; }";
    let path = Path::new("test.js");

    let diagnostics = detector.detect(path, source);

    // Should detect with lower threshold
    let param_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.rule_id == "too-many-parameters").collect();

    assert!(!param_diags.is_empty(), "Should detect with custom threshold");
}

#[test]
fn test_all_code_smell_types() {
    // Verify all code smell types have proper rule IDs
    assert_eq!(CodeSmellType::LongMethod.rule_id(), "long-method");
    assert_eq!(CodeSmellType::LargeClass.rule_id(), "large-class");
    assert_eq!(CodeSmellType::TooManyParameters.rule_id(), "too-many-parameters");
    assert_eq!(CodeSmellType::DeepNesting.rule_id(), "deep-nesting");
    assert_eq!(CodeSmellType::DuplicateCode.rule_id(), "duplicate-code");
    assert_eq!(CodeSmellType::DeadCode.rule_id(), "dead-code");
    assert_eq!(CodeSmellType::MagicNumber.rule_id(), "magic-number");
    assert_eq!(CodeSmellType::MagicString.rule_id(), "magic-string");
}
