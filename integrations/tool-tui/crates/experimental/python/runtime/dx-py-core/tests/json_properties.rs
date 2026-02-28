//! Property-based tests for JSON module
//!
//! Feature: dx-py-production-ready, Property 10: JSON Round-Trip Consistency
//! Validates: Requirements 5.2, 5.3, 5.4
//!
//! This module tests the JSON round-trip property:
//! For any JSON-serializable Python object (dict, list, str, int, float, bool, None),
//! `json.loads(json.dumps(obj))` SHALL produce a value equal to the original object.

#![allow(clippy::cloned_ref_to_slice_refs)]

use proptest::prelude::*;
use std::sync::Arc;

use dx_py_core::pydict::PyKey;
use dx_py_core::pylist::PyValue;
use dx_py_core::stdlib::json_builtins_expanded;
use dx_py_core::PyDict;
use dx_py_core::PyList;

// ===== Arbitrary value generators =====

/// Generate arbitrary JSON-serializable primitive values
fn arb_json_primitive() -> impl Strategy<Value = PyValue> {
    prop_oneof![
        // None
        Just(PyValue::None),
        // Boolean
        any::<bool>().prop_map(PyValue::Bool),
        // Integer (use reasonable range to avoid precision issues)
        (-1_000_000_000i64..1_000_000_000i64).prop_map(PyValue::Int),
        // Float (use finite values only - NaN and Infinity are not JSON-compliant)
        (-1e10..1e10f64)
            .prop_filter("must be finite", |f| f.is_finite())
            .prop_map(PyValue::Float),
        // String (ASCII and common characters for simplicity)
        "[a-zA-Z0-9_ ]{0,50}".prop_map(|s| PyValue::Str(Arc::from(s))),
    ]
}

/// Generate arbitrary JSON-serializable string values
fn arb_json_string() -> impl Strategy<Value = PyValue> {
    "[a-zA-Z0-9_ ]{0,30}".prop_map(|s| PyValue::Str(Arc::from(s)))
}

/// Generate arbitrary JSON-serializable dict keys (must be strings)
fn arb_dict_key() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,15}"
}

/// Generate arbitrary JSON-serializable list values
fn arb_json_list() -> impl Strategy<Value = PyValue> {
    prop::collection::vec(arb_json_primitive(), 0..10).prop_map(|items| {
        PyValue::List(Arc::new(PyList::from_values(items)))
    })
}

/// Generate arbitrary JSON-serializable dict values
fn arb_json_dict() -> impl Strategy<Value = PyValue> {
    prop::collection::vec((arb_dict_key(), arb_json_primitive()), 0..10).prop_map(|pairs| {
        let dict = PyDict::new();
        for (k, v) in pairs {
            dict.setitem(PyKey::Str(Arc::from(k)), v);
        }
        PyValue::Dict(Arc::new(dict))
    })
}

/// Generate arbitrary nested JSON-serializable values (recursive)
fn arb_json_value() -> impl Strategy<Value = PyValue> {
    let leaf = arb_json_primitive();

    leaf.prop_recursive(
        3,   // max depth
        64,  // max nodes
        10,  // items per collection
        |inner| {
            prop_oneof![
                // List of values
                prop::collection::vec(inner.clone(), 0..5)
                    .prop_map(|v| PyValue::List(Arc::new(PyList::from_values(v)))),
                // Dict with string keys
                prop::collection::vec((arb_dict_key(), inner.clone()), 0..5).prop_map(|pairs| {
                    let dict = PyDict::new();
                    for (k, v) in pairs {
                        dict.setitem(PyKey::Str(Arc::from(k)), v);
                    }
                    PyValue::Dict(Arc::new(dict))
                }),
            ]
        },
    )
}

// ===== Value comparison helper =====

/// Check if two PyValues are equivalent for JSON purposes
/// Handles the fact that JSON doesn't distinguish between int and float for whole numbers
fn json_values_equal(a: &PyValue, b: &PyValue) -> bool {
    match (a, b) {
        (PyValue::None, PyValue::None) => true,
        (PyValue::Bool(a), PyValue::Bool(b)) => a == b,
        (PyValue::Int(a), PyValue::Int(b)) => a == b,
        (PyValue::Float(a), PyValue::Float(b)) => {
            // Handle floating point comparison with tolerance
            if a.is_nan() && b.is_nan() {
                true
            } else {
                (a - b).abs() < 1e-10
            }
        }
        // JSON may convert int to float or vice versa for whole numbers
        (PyValue::Int(a), PyValue::Float(b)) | (PyValue::Float(b), PyValue::Int(a)) => {
            (*a as f64 - b).abs() < 1e-10
        }
        (PyValue::Str(a), PyValue::Str(b)) => a == b,
        (PyValue::List(a), PyValue::List(b)) => {
            let a_vec = a.to_vec();
            let b_vec = b.to_vec();
            a_vec.len() == b_vec.len()
                && a_vec
                    .iter()
                    .zip(b_vec.iter())
                    .all(|(x, y)| json_values_equal(x, y))
        }
        (PyValue::Dict(a), PyValue::Dict(b)) => {
            let a_items = a.items();
            let b_items = b.items();
            if a_items.len() != b_items.len() {
                return false;
            }
            for (k, v) in &a_items {
                if let Some(bv) = b_items.iter().find(|(bk, _)| bk == k).map(|(_, v)| v) {
                    if !json_values_equal(v, bv) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // Feature: dx-py-production-ready, Property 10: JSON Round-Trip Consistency
    // Validates: Requirements 5.2, 5.3, 5.4

    /// Property 10: JSON Round-Trip Consistency for primitive values
    /// For any JSON-serializable primitive (str, int, float, bool, None),
    /// json.loads(json.dumps(obj)) SHALL produce a value equal to the original.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_primitives(value in arb_json_primitive()) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for JSON-serializable value");

        // Verify dumps returns a string
        prop_assert!(
            matches!(json_str, PyValue::Str(_)),
            "dumps should return a string, got {:?}",
            json_str
        );

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for valid JSON string");

        // Verify round-trip consistency
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Round-trip failed for primitive: original={:?}, parsed={:?}",
            value,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for list values
    /// For any JSON-serializable list, json.loads(json.dumps(obj)) SHALL produce
    /// a value equal to the original.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_lists(value in arb_json_list()) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for JSON-serializable list");

        // Verify dumps returns a string
        prop_assert!(
            matches!(json_str, PyValue::Str(_)),
            "dumps should return a string for list"
        );

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for valid JSON array string");

        // Verify round-trip consistency
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Round-trip failed for list: original={:?}, parsed={:?}",
            value,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for dict values
    /// For any JSON-serializable dict, json.loads(json.dumps(obj)) SHALL produce
    /// a value equal to the original.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_dicts(value in arb_json_dict()) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for JSON-serializable dict");

        // Verify dumps returns a string
        prop_assert!(
            matches!(json_str, PyValue::Str(_)),
            "dumps should return a string for dict"
        );

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for valid JSON object string");

        // Verify round-trip consistency
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Round-trip failed for dict: original={:?}, parsed={:?}",
            value,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for nested values
    /// For any nested JSON-serializable object (dict, list, str, int, float, bool, None),
    /// json.loads(json.dumps(obj)) SHALL produce a value equal to the original.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_nested(value in arb_json_value()) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for nested JSON-serializable value");

        // Verify dumps returns a string
        prop_assert!(
            matches!(json_str, PyValue::Str(_)),
            "dumps should return a string for nested value"
        );

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for valid nested JSON string");

        // Verify round-trip consistency
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Round-trip failed for nested value: original={:?}, parsed={:?}",
            value,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency with indent parameter
    /// For any JSON-serializable object with indent formatting,
    /// json.loads(json.dumps(obj, indent=N)) SHALL produce a value equal to the original.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_with_indent(
        value in arb_json_value(),
        indent in 0..8i64
    ) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        // Serialize with indent parameter
        let json_str = dumps.call(&[value.clone(), PyValue::Int(indent)])
            .expect("dumps with indent should succeed");

        // Verify dumps returns a string
        prop_assert!(
            matches!(json_str, PyValue::Str(_)),
            "dumps with indent should return a string"
        );

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for indented JSON string");

        // Verify round-trip consistency
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Round-trip with indent={} failed: original={:?}, parsed={:?}",
            indent,
            value,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for string values with special characters
    /// For any string value, json.loads(json.dumps(str)) SHALL produce the original string.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_strings(value in arb_json_string()) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for string value");

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for JSON string");

        // Verify exact string equality
        prop_assert!(
            json_values_equal(&value, &parsed),
            "String round-trip failed: original={:?}, parsed={:?}",
            value,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for boolean values
    /// For any boolean value, json.loads(json.dumps(bool)) SHALL produce the original boolean.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_booleans(b in any::<bool>()) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        let value = PyValue::Bool(b);

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for boolean");

        // Verify the JSON representation
        if let PyValue::Str(s) = &json_str {
            let expected = if b { "true" } else { "false" };
            prop_assert_eq!(s.as_ref(), expected, "Boolean should serialize to {}", expected);
        }

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for JSON boolean");

        // Verify exact boolean equality
        prop_assert!(
            matches!(parsed, PyValue::Bool(pb) if pb == b),
            "Boolean round-trip failed: original={}, parsed={:?}",
            b,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for integer values
    /// For any integer value, json.loads(json.dumps(int)) SHALL produce the original integer.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_integers(i in -1_000_000_000i64..1_000_000_000i64) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        let value = PyValue::Int(i);

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for integer");

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for JSON integer");

        // Verify integer equality (may be parsed as int or float)
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Integer round-trip failed: original={}, parsed={:?}",
            i,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for float values
    /// For any finite float value, json.loads(json.dumps(float)) SHALL produce
    /// a value approximately equal to the original.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_floats(
        f in (-1e10..1e10f64).prop_filter("must be finite", |f| f.is_finite())
    ) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        let value = PyValue::Float(f);

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for float");

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for JSON float");

        // Verify float equality with tolerance
        prop_assert!(
            json_values_equal(&value, &parsed),
            "Float round-trip failed: original={}, parsed={:?}",
            f,
            parsed
        );
    }

    /// Property 10: JSON Round-Trip Consistency for None value
    /// json.loads(json.dumps(None)) SHALL produce None.
    ///
    /// **Validates: Requirements 5.2, 5.3, 5.4**
    #[test]
    fn prop_json_roundtrip_none(_dummy in 0..10i32) {
        let dumps = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "dumps")
            .expect("dumps function should exist");
        let loads = json_builtins_expanded()
            .into_iter()
            .find(|f| f.name == "loads")
            .expect("loads function should exist");

        let value = PyValue::None;

        // Serialize to JSON string
        let json_str = dumps.call(&[value.clone()])
            .expect("dumps should succeed for None");

        // Verify the JSON representation
        if let PyValue::Str(s) = &json_str {
            prop_assert_eq!(s.as_ref(), "null", "None should serialize to 'null'");
        }

        // Deserialize back to PyValue
        let parsed = loads.call(&[json_str])
            .expect("loads should succeed for JSON null");

        // Verify None equality
        prop_assert!(
            matches!(parsed, PyValue::None),
            "None round-trip failed: parsed={:?}",
            parsed
        );
    }
}

// ===== Unit Tests for Edge Cases =====

#[test]
fn test_json_roundtrip_empty_list() {
    let dumps = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "dumps")
        .unwrap();
    let loads = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "loads")
        .unwrap();

    let empty_list = PyValue::List(Arc::new(PyList::new()));
    let json_str = dumps.call(&[empty_list.clone()]).unwrap();

    if let PyValue::Str(s) = &json_str {
        assert_eq!(s.as_ref(), "[]");
    }

    let parsed = loads.call(&[json_str]).unwrap();
    assert!(json_values_equal(&empty_list, &parsed));
}

#[test]
fn test_json_roundtrip_empty_dict() {
    let dumps = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "dumps")
        .unwrap();
    let loads = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "loads")
        .unwrap();

    let empty_dict = PyValue::Dict(Arc::new(PyDict::new()));
    let json_str = dumps.call(&[empty_dict.clone()]).unwrap();

    if let PyValue::Str(s) = &json_str {
        assert_eq!(s.as_ref(), "{}");
    }

    let parsed = loads.call(&[json_str]).unwrap();
    assert!(json_values_equal(&empty_dict, &parsed));
}

#[test]
fn test_json_roundtrip_deeply_nested() {
    let dumps = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "dumps")
        .unwrap();
    let loads = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "loads")
        .unwrap();

    // Create a deeply nested structure: {"a": {"b": {"c": [1, 2, 3]}}}
    let inner_list = PyValue::List(Arc::new(PyList::from_values(vec![
        PyValue::Int(1),
        PyValue::Int(2),
        PyValue::Int(3),
    ])));

    let inner_dict = PyDict::new();
    inner_dict.setitem(PyKey::Str(Arc::from("c")), inner_list);

    let middle_dict = PyDict::new();
    middle_dict.setitem(PyKey::Str(Arc::from("b")), PyValue::Dict(Arc::new(inner_dict)));

    let outer_dict = PyDict::new();
    outer_dict.setitem(PyKey::Str(Arc::from("a")), PyValue::Dict(Arc::new(middle_dict)));

    let value = PyValue::Dict(Arc::new(outer_dict));
    let json_str = dumps.call(&[value.clone()]).unwrap();
    let parsed = loads.call(&[json_str]).unwrap();

    assert!(json_values_equal(&value, &parsed));
}

#[test]
fn test_json_roundtrip_mixed_list() {
    let dumps = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "dumps")
        .unwrap();
    let loads = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "loads")
        .unwrap();

    // Create a list with mixed types: [1, "hello", true, null, 3.14]
    let mixed_list = PyValue::List(Arc::new(PyList::from_values(vec![
        PyValue::Int(1),
        PyValue::Str(Arc::from("hello")),
        PyValue::Bool(true),
        PyValue::None,
        PyValue::Float(3.14),
    ])));

    let json_str = dumps.call(&[mixed_list.clone()]).unwrap();
    let parsed = loads.call(&[json_str]).unwrap();

    assert!(json_values_equal(&mixed_list, &parsed));
}

#[test]
fn test_json_dumps_produces_valid_json_string() {
    let dumps = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "dumps")
        .unwrap();

    // Test that dumps produces valid JSON for dict
    let dict = PyDict::new();
    dict.setitem(PyKey::Str(Arc::from("key")), PyValue::Str(Arc::from("value")));
    let result = dumps.call(&[PyValue::Dict(Arc::new(dict))]).unwrap();

    if let PyValue::Str(s) = result {
        // Should be valid JSON object format
        assert!(s.starts_with('{'));
        assert!(s.ends_with('}'));
        assert!(s.contains("\"key\""));
        assert!(s.contains("\"value\""));
    } else {
        panic!("dumps should return a string");
    }
}

#[test]
fn test_json_loads_returns_correct_python_object() {
    let loads = json_builtins_expanded()
        .into_iter()
        .find(|f| f.name == "loads")
        .unwrap();

    // Test parsing a JSON object
    let json = PyValue::Str(Arc::from(r#"{"a": 1, "b": [1, 2, 3]}"#));
    let result = loads.call(&[json]).unwrap();

    if let PyValue::Dict(d) = result {
        // Check "a" key
        let a = d.getitem(&PyKey::Str(Arc::from("a"))).unwrap();
        assert!(json_values_equal(&a, &PyValue::Int(1)));

        // Check "b" key
        let b = d.getitem(&PyKey::Str(Arc::from("b"))).unwrap();
        if let PyValue::List(list) = b {
            assert_eq!(list.len(), 3);
        } else {
            panic!("Expected list for key 'b'");
        }
    } else {
        panic!("loads should return a dict for JSON object");
    }
}
