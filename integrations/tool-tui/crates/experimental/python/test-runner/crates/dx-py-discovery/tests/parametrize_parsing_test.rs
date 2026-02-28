//! Integration test for parametrize parsing (Task 17.1)
//!
//! This test verifies that @pytest.mark.parametrize decorators are correctly parsed
//! to extract parameter names and values.

use dx_py_core::Marker;
use dx_py_discovery::ParametrizeExpander;

#[test]
fn test_parse_single_parameter() {
    let expander = ParametrizeExpander::new();
    
    // Create a marker as it would be extracted from AST
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec!["\"x\"".to_string(), "[1, 2, 3]".to_string()],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Should expand to 3 test cases (one per value)
    assert_eq!(expanded.len(), 3);
    
    // Verify parameter values are correctly extracted
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
    assert_eq!(expanded[1].param_values.get("x"), Some(&"2".to_string()));
    assert_eq!(expanded[2].param_values.get("x"), Some(&"3".to_string()));
}

#[test]
fn test_parse_multiple_parameters() {
    let expander = ParametrizeExpander::new();
    
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec!["\"x,y\"".to_string(), "[(1, 2), (3, 4)]".to_string()],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Should expand to 2 test cases
    assert_eq!(expanded.len(), 2);
    
    // Verify both parameters are extracted correctly
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
    assert_eq!(expanded[0].param_values.get("y"), Some(&"2".to_string()));
    assert_eq!(expanded[1].param_values.get("x"), Some(&"3".to_string()));
    assert_eq!(expanded[1].param_values.get("y"), Some(&"4".to_string()));
}

#[test]
fn test_parse_with_custom_ids() {
    let expander = ParametrizeExpander::new();
    
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec![
            "\"value\"".to_string(),
            "[pytest.param(1, id=\"one\"), pytest.param(2, id=\"two\")]".to_string(),
        ],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Should expand to 2 test cases
    assert_eq!(expanded.len(), 2);
    
    // Verify custom IDs are used
    assert_eq!(expanded[0].id_suffix, "one");
    assert_eq!(expanded[1].id_suffix, "two");
    
    // Verify values are still extracted
    assert_eq!(expanded[0].param_values.get("value"), Some(&"1".to_string()));
    assert_eq!(expanded[1].param_values.get("value"), Some(&"2".to_string()));
}

#[test]
fn test_parse_with_marks() {
    let expander = ParametrizeExpander::new();
    
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec![
            "\"x\"".to_string(),
            "[pytest.param(1, marks=pytest.mark.xfail), 2]".to_string(),
        ],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Should expand to 2 test cases
    assert_eq!(expanded.len(), 2);
    
    // First should be marked as expected failure
    assert!(expanded[0].expected_failure);
    assert!(!expanded[1].expected_failure);
}

#[test]
fn test_parse_string_values() {
    let expander = ParametrizeExpander::new();
    
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec!["\"name\"".to_string(), "[\"alice\", \"bob\"]".to_string()],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Should expand to 2 test cases
    assert_eq!(expanded.len(), 2);
    
    // Verify string values are preserved with quotes
    assert_eq!(expanded[0].param_values.get("name"), Some(&"\"alice\"".to_string()));
    assert_eq!(expanded[1].param_values.get("name"), Some(&"\"bob\"".to_string()));
}

#[test]
fn test_parse_mixed_types() {
    let expander = ParametrizeExpander::new();
    
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec!["\"x,y\"".to_string(), "[(1, \"a\"), (2, \"b\")]".to_string()],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Should expand to 2 test cases
    assert_eq!(expanded.len(), 2);
    
    // Verify mixed types are handled correctly
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
    assert_eq!(expanded[0].param_values.get("y"), Some(&"\"a\"".to_string()));
    assert_eq!(expanded[1].param_values.get("x"), Some(&"2".to_string()));
    assert_eq!(expanded[1].param_values.get("y"), Some(&"\"b\"".to_string()));
}

#[test]
fn test_parse_empty_list() {
    let expander = ParametrizeExpander::new();
    
    let marker = Marker::with_args(
        "pytest.mark.parametrize",
        vec!["\"x\"".to_string(), "[]".to_string()],
    );
    
    let test = create_test_with_markers(vec![marker]);
    let expanded = expander.expand(&test);
    
    // Empty parameter list should result in no expanded tests
    assert_eq!(expanded.len(), 0);
}

// Helper function to create a test case with markers
fn create_test_with_markers(markers: Vec<Marker>) -> dx_py_core::TestCase {
    use std::path::PathBuf;
    
    let mut tc = dx_py_core::TestCase::new("test_example", PathBuf::from("test.py"), 1);
    for marker in markers {
        tc = tc.with_marker(marker);
    }
    tc
}
