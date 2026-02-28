//! Tests for dx-py-discovery

use super::*;
use dx_py_core::{Marker, TestCase};
use proptest::prelude::*;
use tempfile::TempDir;

// Include parametrize tests
mod parametrize_tests {
    include!("parametrize_tests.rs");
}

// Include plugin tests
mod plugin_tests {
    include!("plugin_tests.rs");
}

// Include coverage tests
mod coverage_tests {
    include!("coverage_tests.rs");
}

// Property 23: Parametrize Expansion
// For any test with @pytest.mark.parametrize(name, values), the test SHALL run exactly len(values) times

proptest! {
    /// Feature: dx-py-production-ready, Property 23: Parametrize Expansion
    /// Validates: Requirements 12.1, 12.4, 12.5
    #[test]
    fn prop_parametrize_expansion_count(
        num_sets in 1usize..5,
        num_params in 1usize..3,
    ) {
        // Generate parameter names
        let param_names: Vec<String> = (0..num_params).map(|i| format!("p{}", i)).collect();
        let param_names_str = param_names.join(",");

        // Generate values
        let values: Vec<Vec<String>> = (0..num_sets)
            .map(|i| (0..num_params).map(|j| format!("v{}_{}", i, j)).collect())
            .collect();

        // Format as Python list
        let values_str = format!("[{}]",
            values.iter()
                .map(|v| if v.len() == 1 { v[0].clone() } else { format!("({})", v.join(", ")) })
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Create test with parametrize marker
        let test = TestCase::new("test_param", std::path::PathBuf::from("test.py"), 1)
            .with_marker(Marker::with_args(
                "pytest.mark.parametrize",
                vec![format!("\"{}\"", param_names_str), values_str],
            ));

        let expander = ParametrizeExpander::new();
        let expanded = expander.expand(&test);

        // Property: number of expanded tests equals number of parameter sets
        prop_assert_eq!(expanded.len(), num_sets,
            "Expected {} expanded tests, got {}", num_sets, expanded.len());

        // Property: each expanded test has all parameter values
        for (idx, exp) in expanded.iter().enumerate() {
            for param_name in param_names.iter() {
                prop_assert!(exp.param_values.contains_key(param_name),
                    "Expanded test {} missing parameter {}", idx, param_name);
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 24: Parametrize Cartesian Product
    /// Validates: Requirements 12.2
    #[test]
    fn prop_parametrize_cartesian_product(
        sets1 in 1usize..4,
        sets2 in 1usize..4,
    ) {
        // Create test with two parametrize decorators
        let values1_str = format!("[{}]", (0..sets1).map(|i| i.to_string()).collect::<Vec<_>>().join(", "));
        let values2_str = format!("[{}]", (0..sets2).map(|i| format!("\"{}\"", (b'a' + i as u8) as char)).collect::<Vec<_>>().join(", "));

        let test = TestCase::new("test_param", std::path::PathBuf::from("test.py"), 1)
            .with_marker(Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), values1_str],
            ))
            .with_marker(Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"y\"".to_string(), values2_str],
            ));

        let expander = ParametrizeExpander::new();
        let expanded = expander.expand(&test);

        // Property: cartesian product produces sets1 * sets2 tests
        let expected_count = sets1 * sets2;
        prop_assert_eq!(expanded.len(), expected_count,
            "Expected {} expanded tests ({}x{}), got {}", expected_count, sets1, sets2, expanded.len());

        // Property: all combinations are present
        let mut seen_combinations = std::collections::HashSet::new();
        for exp in &expanded {
            let x = exp.param_values.get("x").cloned().unwrap_or_default();
            let y = exp.param_values.get("y").cloned().unwrap_or_default();
            seen_combinations.insert((x, y));
        }
        prop_assert_eq!(seen_combinations.len(), expected_count,
            "Expected {} unique combinations, got {}", expected_count, seen_combinations.len());
    }

    /// Feature: dx-py-production-ready, Property 23: Parametrize Expansion
    /// Validates: Requirements 12.3 (custom IDs)
    #[test]
    fn prop_parametrize_unique_ids(
        num_sets in 1usize..10,
    ) {
        // Generate values
        let values_str = format!("[{}]",
            (0..num_sets).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
        );

        let test = TestCase::new("test_param", std::path::PathBuf::from("test.py"), 1)
            .with_marker(Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), values_str],
            ));

        let expander = ParametrizeExpander::new();
        let expanded = expander.expand(&test);

        // Property: all expanded tests have unique IDs
        let ids: std::collections::HashSet<_> = expanded.iter()
            .map(|e| e.full_id())
            .collect();

        prop_assert_eq!(ids.len(), expanded.len(),
            "Expected {} unique IDs, got {}", expanded.len(), ids.len());
    }

    /// Feature: dx-py-production-ready, Property 23: Parametrize Expansion
    /// Validates: Requirements 12.1, 12.4 (correct parameter values)
    #[test]
    fn prop_parametrize_correct_values(
        values in prop::collection::vec(0i32..100, 1..10),
    ) {
        let values_str = format!("[{}]",
            values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
        );

        let test = TestCase::new("test_param", std::path::PathBuf::from("test.py"), 1)
            .with_marker(Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), values_str],
            ));

        let expander = ParametrizeExpander::new();
        let expanded = expander.expand(&test);

        // Property: expanded tests have correct parameter values
        prop_assert_eq!(expanded.len(), values.len());

        for (idx, exp) in expanded.iter().enumerate() {
            let expected_value = values[idx].to_string();
            let actual_value = exp.param_values.get("x").cloned().unwrap_or_default();
            prop_assert_eq!(&actual_value, &expected_value,
                "Test {} has wrong value: expected {}, got {}", idx, &expected_value, &actual_value);
        }
    }
}

// Property 1: Test Function Detection
// For any Python source with functions, only test_* or *_test or decorated functions are detected

fn arb_function_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // Test functions (should be detected)
        "test_[a-z_]{1,15}".prop_map(|s| s),
        "[a-z_]{1,15}_test".prop_map(|s| s),
        // Non-test functions (should not be detected)
        "[a-z_]{1,15}"
            .prop_filter("not a test", |s| { !s.starts_with("test_") && !s.ends_with("_test") }),
    ]
}

fn arb_class_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // Test classes (should be scanned)
        "Test[A-Z][a-zA-Z]{0,15}".prop_map(|s| s),
        // Non-test classes (should not be scanned)
        "[A-Z][a-zA-Z]{0,15}".prop_filter("not a test class", |s| !s.starts_with("Test")),
    ]
}

fn generate_python_function(name: &str, has_pytest_mark: bool) -> String {
    if has_pytest_mark {
        format!("@pytest.mark.unit\ndef {}():\n    pass\n", name)
    } else {
        format!("def {}():\n    pass\n", name)
    }
}

fn generate_python_class(class_name: &str, methods: &[String]) -> String {
    let mut code = format!("class {}:\n", class_name);
    for method in methods {
        code.push_str(&format!("    def {}(self):\n        pass\n", method));
    }
    code
}

proptest! {
    /// Feature: dx-py-test-runner, Property 1: Test Function Detection
    /// Validates: Requirements 1.2, 1.3, 1.4
    #[test]
    fn prop_test_function_detection(
        func_name in arb_function_name(),
        has_pytest_mark in any::<bool>()
    ) {
        let source = generate_python_function(&func_name, has_pytest_mark);
        let mut scanner = TestScanner::new().unwrap();
        let tests = scanner.scan_source(&source).unwrap();

        let is_test_name = func_name.starts_with("test_") || func_name.ends_with("_test");
        let should_be_detected = is_test_name || has_pytest_mark;

        if should_be_detected {
            prop_assert!(!tests.is_empty(), "Expected test to be detected: {}", func_name);
            prop_assert_eq!(&tests[0].name, &func_name);
        } else {
            prop_assert!(tests.is_empty(), "Expected no test to be detected: {}", func_name);
        }
    }

    /// Feature: dx-py-test-runner, Property 1: Test Function Detection
    /// Validates: Requirements 1.2, 1.3
    #[test]
    fn prop_test_class_detection(
        class_name in arb_class_name(),
        method_names in prop::collection::vec(arb_function_name(), 1..5)
    ) {
        let source = generate_python_class(&class_name, &method_names);
        let mut scanner = TestScanner::new().unwrap();
        let tests = scanner.scan_source(&source).unwrap();

        let is_test_class = class_name.starts_with("Test");

        if is_test_class {
            // Should find test methods in Test* classes
            let expected_test_count = method_names.iter()
                .filter(|m| m.starts_with("test_") || m.ends_with("_test"))
                .count();
            prop_assert_eq!(tests.len(), expected_test_count,
                "Expected {} tests in class {}, found {}",
                expected_test_count, class_name, tests.len());

            // All detected tests should have the class name
            for test in &tests {
                prop_assert_eq!(test.class_name.as_deref(), Some(class_name.as_str()));
            }
        } else {
            // Non-Test classes should not have their methods scanned
            prop_assert!(tests.is_empty(),
                "Expected no tests from non-Test class {}", class_name);
        }
    }
}

// Property 2: Test Index Round-Trip
// For any set of test cases, writing to index and reading back produces equivalent data

proptest! {
    /// Feature: dx-py-test-runner, Property 2: Test Index Round-Trip
    /// Validates: Requirements 1.5, 1.6
    #[test]
    fn prop_test_index_roundtrip(
        test_names in prop::collection::vec("test_[a-z_]{1,10}", 1..10)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_example.py");
        let index_file = temp_dir.path().join("test.dxti");

        // Create a Python file with tests
        let mut source = String::new();
        for name in &test_names {
            source.push_str(&format!("def {}():\n    pass\n\n", name));
        }
        std::fs::write(&test_file, &source).unwrap();

        // Scan and build index
        let mut scanner = TestScanner::new().unwrap();
        let tests = scanner.scan_file(&test_file).unwrap();

        let mut builder = TestIndexBuilder::new();
        builder.add_file(&test_file, tests.clone()).unwrap();
        let index = builder.build();

        // Save and reload
        index.save(&index_file).unwrap();
        let loaded = TestIndex::load(&index_file).unwrap();

        // Verify round-trip
        let original_tests = index.all_tests();
        let loaded_tests = loaded.all_tests();

        prop_assert_eq!(original_tests.len(), loaded_tests.len());
        for (orig, loaded) in original_tests.iter().zip(loaded_tests.iter()) {
            prop_assert_eq!(&orig.name, &loaded.name);
            prop_assert_eq!(orig.line_number, loaded.line_number);
            prop_assert_eq!(&orig.class_name, &loaded.class_name);
        }
    }
}

// Unit tests

#[test]
fn test_scan_simple_test_function() {
    let source = r#"
def test_example():
    assert True
"#;
    let mut scanner = TestScanner::new().unwrap();
    let tests = scanner.scan_source(source).unwrap();

    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].name, "test_example");
    assert!(tests[0].class_name.is_none());
}

#[test]
fn test_scan_test_class() {
    let source = r#"
class TestExample:
    def test_one(self):
        pass

    def test_two(self):
        pass

    def helper(self):
        pass
"#;
    let mut scanner = TestScanner::new().unwrap();
    let tests = scanner.scan_source(source).unwrap();

    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "test_one");
    assert_eq!(tests[0].class_name.as_deref(), Some("TestExample"));
    assert_eq!(tests[1].name, "test_two");
    assert_eq!(tests[1].class_name.as_deref(), Some("TestExample"));
}

#[test]
fn test_scan_pytest_mark_decorator() {
    let source = r#"
import pytest

@pytest.mark.slow
def my_slow_function():
    pass
"#;
    let mut scanner = TestScanner::new().unwrap();
    let tests = scanner.scan_source(source).unwrap();

    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].name, "my_slow_function");
}

#[test]
fn test_scan_fixture() {
    let source = r#"
import pytest

@pytest.fixture
def my_fixture():
    return 42
"#;
    let mut scanner = TestScanner::new().unwrap();
    let tests = scanner.scan_source(source).unwrap();

    // Fixtures should be detected and marked as fixtures
    assert_eq!(tests.len(), 1);
    assert!(tests[0].is_fixture);
    assert_eq!(tests[0].name, "my_fixture");
}

#[test]
fn test_scan_non_test_class() {
    let source = r#"
class Helper:
    def test_like_method(self):
        pass
"#;
    let mut scanner = TestScanner::new().unwrap();
    let tests = scanner.scan_source(source).unwrap();

    // Non-Test classes should not have their methods scanned
    assert!(tests.is_empty());
}

#[test]
fn test_scan_test_with_parameters() {
    let source = r#"
def test_with_fixtures(sample_data, db_connection):
    assert sample_data is not None
    assert db_connection is not None
"#;
    let mut scanner = TestScanner::new().unwrap();
    let discovered = scanner.scan_source(source).unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "test_with_fixtures");
    assert_eq!(discovered[0].parameters, vec!["sample_data", "db_connection"]);
}

#[test]
fn test_scan_test_with_self_parameter() {
    let source = r#"
class TestExample:
    def test_method(self, fixture_a, fixture_b):
        pass
"#;
    let mut scanner = TestScanner::new().unwrap();
    let discovered = scanner.scan_source(source).unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "test_method");
    // 'self' should be filtered out
    assert_eq!(discovered[0].parameters, vec!["fixture_a", "fixture_b"]);
}

#[test]
fn test_scan_test_no_parameters() {
    let source = r#"
def test_simple():
    assert True
"#;
    let mut scanner = TestScanner::new().unwrap();
    let discovered = scanner.scan_source(source).unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "test_simple");
    assert!(discovered[0].parameters.is_empty());
}

#[test]
fn test_index_needs_rescan() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_example.py");
    let index_file = temp_dir.path().join("test.dxti");

    // Create initial file
    std::fs::write(&test_file, "def test_one():\n    pass\n").unwrap();

    // Build and save index
    let mut scanner = TestScanner::new().unwrap();
    let tests = scanner.scan_file(&test_file).unwrap();
    let mut builder = TestIndexBuilder::new();
    builder.add_file(&test_file, tests).unwrap();
    let index = builder.build();
    index.save(&index_file).unwrap();

    // Load index - should not need rescan
    let loaded = TestIndex::load(&index_file).unwrap();
    assert!(!loaded.needs_rescan(&test_file));

    // Modify file
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(&test_file, "def test_one():\n    pass\ndef test_two():\n    pass\n").unwrap();

    // Should need rescan now
    assert!(loaded.needs_rescan(&test_file));
}

#[test]
fn test_index_file_count_and_test_count() {
    let temp_dir = TempDir::new().unwrap();

    let mut builder = TestIndexBuilder::new();

    // Create two test files
    for i in 0..2 {
        let test_file = temp_dir.path().join(format!("test_{}.py", i));
        let source = format!("def test_a{}():\n    pass\ndef test_b{}():\n    pass\n", i, i);
        std::fs::write(&test_file, &source).unwrap();

        let mut scanner = TestScanner::new().unwrap();
        let tests = scanner.scan_file(&test_file).unwrap();
        builder.add_file(&test_file, tests).unwrap();
    }

    let index = builder.build();
    assert_eq!(index.file_count(), 2);
    assert_eq!(index.test_count(), 4);
}
