// Unit tests for parametrization

use super::parametrize::*;
use dx_py_core::{Marker, TestCase};
use std::path::PathBuf;

fn make_test(name: &str, markers: Vec<Marker>) -> TestCase {
    let mut tc = TestCase::new(name, PathBuf::from("test_file.py"), 1);
    for marker in markers {
        tc = tc.with_marker(marker);
    }
    tc
}

#[test]
fn test_no_parametrize() {
    let expander = ParametrizeExpander::new();
    let test = make_test("test_simple", vec![]);
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 1);
    assert!(expanded[0].param_values.is_empty());
    assert!(expanded[0].id_suffix.is_empty());
}

#[test]
fn test_single_param_single_value() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec!["\"x\"".to_string(), "[1]".to_string()],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
}

#[test]
fn test_single_param_multiple_values() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec!["\"x\"".to_string(), "[1, 2, 3]".to_string()],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 3);
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
    assert_eq!(expanded[1].param_values.get("x"), Some(&"2".to_string()));
    assert_eq!(expanded[2].param_values.get("x"), Some(&"3".to_string()));
}

#[test]
fn test_multiple_params() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec!["\"x,y\"".to_string(), "[(1, 2), (3, 4)]".to_string()],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
    assert_eq!(expanded[0].param_values.get("y"), Some(&"2".to_string()));
    assert_eq!(expanded[1].param_values.get("x"), Some(&"3".to_string()));
    assert_eq!(expanded[1].param_values.get("y"), Some(&"4".to_string()));
}

#[test]
fn test_cartesian_product() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), "[1, 2]".to_string()],
            ),
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"y\"".to_string(), "[\"a\", \"b\"]".to_string()],
            ),
        ],
    );
    
    let expanded = expander.expand(&test);
    
    // 2 x 2 = 4 combinations
    assert_eq!(expanded.len(), 4);
    
    // Check all combinations exist
    let combinations: Vec<_> = expanded.iter()
        .map(|e| (e.param_values.get("x").unwrap().clone(), e.param_values.get("y").unwrap().clone()))
        .collect();
    
    assert!(combinations.contains(&("1".to_string(), "\"a\"".to_string())));
    assert!(combinations.contains(&("1".to_string(), "\"b\"".to_string())));
    assert!(combinations.contains(&("2".to_string(), "\"a\"".to_string())));
    assert!(combinations.contains(&("2".to_string(), "\"b\"".to_string())));
}

#[test]
fn test_custom_ids() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec![
                "\"x\"".to_string(),
                "[pytest.param(1, id=\"one\"), pytest.param(2, id=\"two\")]".to_string(),
            ],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0].id_suffix, "one");
    assert_eq!(expanded[1].id_suffix, "two");
}

#[test]
fn test_xfail_marker() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec![
                "\"x\"".to_string(),
                "[pytest.param(1, marks=pytest.mark.xfail), 2]".to_string(),
            ],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 2);
    assert!(expanded[0].expected_failure);
    assert!(!expanded[1].expected_failure);
}

#[test]
fn test_expand_all() {
    let expander = ParametrizeExpander::new();
    let tests = vec![
        make_test("test_simple", vec![]),
        make_test(
            "test_param",
            vec![Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), "[1, 2]".to_string()],
            )],
        ),
    ];
    
    let expanded = expander.expand_all(&tests);
    
    // 1 (non-parametrized) + 2 (parametrized) = 3
    assert_eq!(expanded.len(), 3);
}

#[test]
fn test_to_test_case() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec!["\"x\"".to_string(), "[1, 2]".to_string()],
        )],
    );
    
    let expanded = expander.expand(&test);
    let test_case = expanded[0].to_test_case();
    
    assert!(test_case.name.contains("["));
    assert!(test_case.name.contains("]"));
}

#[test]
fn test_string_values() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec!["\"name\"".to_string(), "[\"alice\", \"bob\"]".to_string()],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0].param_values.get("name"), Some(&"\"alice\"".to_string()));
    assert_eq!(expanded[1].param_values.get("name"), Some(&"\"bob\"".to_string()));
}

#[test]
fn test_mixed_types() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![Marker::with_args(
            "pytest.mark.parametrize",
            vec!["\"x,y\"".to_string(), "[(1, \"a\"), (2, \"b\")]".to_string()],
        )],
    );
    
    let expanded = expander.expand(&test);
    
    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0].param_values.get("x"), Some(&"1".to_string()));
    assert_eq!(expanded[0].param_values.get("y"), Some(&"\"a\"".to_string()));
}

#[test]
fn test_three_way_cartesian() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), "[1, 2]".to_string()],
            ),
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"y\"".to_string(), "[\"a\", \"b\"]".to_string()],
            ),
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"z\"".to_string(), "[True, False]".to_string()],
            ),
        ],
    );
    
    let expanded = expander.expand(&test);
    
    // 2 x 2 x 2 = 8 combinations
    assert_eq!(expanded.len(), 8);
}

#[test]
fn test_id_suffix_format() {
    let expander = ParametrizeExpander::new();
    let test = make_test(
        "test_param",
        vec![
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"x\"".to_string(), "[1, 2]".to_string()],
            ),
            Marker::with_args(
                "pytest.mark.parametrize",
                vec!["\"y\"".to_string(), "[\"a\", \"b\"]".to_string()],
            ),
        ],
    );
    
    let expanded = expander.expand(&test);
    
    // IDs should be in format "0-0", "0-1", "1-0", "1-1"
    let ids: Vec<_> = expanded.iter().map(|e| e.id_suffix.clone()).collect();
    assert!(ids.contains(&"0-0".to_string()));
    assert!(ids.contains(&"0-1".to_string()));
    assert!(ids.contains(&"1-0".to_string()));
    assert!(ids.contains(&"1-1".to_string()));
}
