//! Property-based tests for dict methods
//!
//! Feature: dx-py-production-ready-v2
//! Property 8: Dict Keys-Values-Items Consistency
//! Property 9: Dict Get Consistency
//! Property 10: Dict Update Merge
//! Validates: Requirements 3.1-3.9

use proptest::prelude::*;
use std::sync::Arc;

use dx_py_core::pydict::{PyDict, PyKey};
use dx_py_core::pylist::PyValue;

// ===== Generators for property tests =====

/// Generate a key-value pair with string key
fn arb_str_key_value() -> impl Strategy<Value = (String, i64)> {
    ("[a-z]{1,10}", any::<i64>())
}

/// Generate a dict with string keys and int values
fn arb_str_int_dict() -> impl Strategy<Value = Vec<(String, i64)>> {
    prop::collection::vec(arb_str_key_value(), 0..20)
}

/// Generate a non-empty dict with string keys and int values
fn arb_nonempty_str_int_dict() -> impl Strategy<Value = Vec<(String, i64)>> {
    prop::collection::vec(arb_str_key_value(), 1..20)
}

/// Generate a key-value pair with int key
fn arb_int_key_value() -> impl Strategy<Value = (i64, i64)> {
    (any::<i64>(), any::<i64>())
}

/// Generate a dict with int keys and int values
fn arb_int_int_dict() -> impl Strategy<Value = Vec<(i64, i64)>> {
    prop::collection::vec(arb_int_key_value(), 0..20)
}

/// Generate a non-empty dict with int keys and int values
fn arb_nonempty_int_int_dict() -> impl Strategy<Value = Vec<(i64, i64)>> {
    prop::collection::vec(arb_int_key_value(), 1..20)
}

// ===== Helper functions =====

/// Create a PyDict from a list of (String, i64) pairs
fn create_str_dict(items: &[(String, i64)]) -> PyDict {
    let dict = PyDict::new();
    for (k, v) in items {
        dict.setitem(PyKey::Str(Arc::from(k.as_str())), PyValue::Int(*v));
    }
    dict
}

/// Create a PyDict from a list of (i64, i64) pairs
fn create_int_dict(items: &[(i64, i64)]) -> PyDict {
    let dict = PyDict::new();
    for (k, v) in items {
        dict.setitem(PyKey::Int(*k), PyValue::Int(*v));
    }
    dict
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // ===== Property 8: Dict Keys-Values-Items Consistency =====

    /// Feature: dx-py-production-ready-v2, Property 8: Dict Keys-Values-Items Consistency
    /// For any dict d, len(d.keys()) should equal len(d.values()) should equal len(d.items()) should equal len(d).
    /// Validates: Requirements 3.1, 3.2, 3.3
    #[test]
    fn prop_keys_values_items_length_consistency(items in arb_str_int_dict()) {
        let dict = create_str_dict(&items);
        let dict_len = dict.len();
        
        let keys = dict.keys_list();
        let values = dict.values_list();
        let items_list = dict.items_list();
        
        // Extract lengths from PyValue::List
        let keys_len = if let PyValue::List(l) = keys { l.len() } else { panic!("keys should return list") };
        let values_len = if let PyValue::List(l) = values { l.len() } else { panic!("values should return list") };
        let items_len = if let PyValue::List(l) = items_list { l.len() } else { panic!("items should return list") };
        
        prop_assert_eq!(keys_len, dict_len,
            "keys() length should equal dict length");
        prop_assert_eq!(values_len, dict_len,
            "values() length should equal dict length");
        prop_assert_eq!(items_len, dict_len,
            "items() length should equal dict length");
    }

    /// Feature: dx-py-production-ready-v2, Property 8: Dict Keys-Values-Items Consistency
    /// Items should contain tuples of (key, value) pairs.
    /// Validates: Requirements 3.3
    #[test]
    fn prop_items_contains_key_value_tuples(items in arb_nonempty_int_int_dict()) {
        let dict = create_int_dict(&items);
        
        let items_list = dict.items_list();
        if let PyValue::List(list) = items_list {
            for i in 0..list.len() {
                let item = list.getitem(i as i64).expect("should get item");
                if let PyValue::Tuple(tuple) = item {
                    prop_assert_eq!(tuple.len(), 2,
                        "each item should be a 2-tuple");
                } else {
                    prop_assert!(false, "items should contain tuples");
                }
            }
        } else {
            prop_assert!(false, "items should return list");
        }
    }

    // ===== Property 9: Dict Get Consistency =====

    /// Feature: dx-py-production-ready-v2, Property 9: Dict Get Consistency
    /// For any dict d and key k, if k in d then d.get(k) should equal d[k].
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_get_returns_value_for_existing_key(items in arb_nonempty_int_int_dict()) {
        let dict = create_int_dict(&items);
        
        // Pick a key that exists
        let (key, expected_value) = &items[0];
        let py_key = PyKey::Int(*key);
        
        // get() should return the value
        let result = dict.get_with_default(&py_key, PyValue::None);
        
        if let PyValue::Int(val) = result {
            prop_assert_eq!(val, *expected_value,
                "get() should return the value for existing key");
        } else {
            prop_assert!(false, "get() should return Int for existing key");
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 9: Dict Get Consistency
    /// For any dict d and key k not in d, d.get(k) should be None and d.get(k, default) should equal default.
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_get_returns_default_for_missing_key(items in arb_int_int_dict(), missing_key: i64, default_value: i64) {
        let dict = create_int_dict(&items);
        
        // Use a key that's unlikely to exist (very large number)
        let unlikely_key = missing_key.wrapping_add(i64::MAX / 2);
        let py_key = PyKey::Int(unlikely_key);
        
        // If key doesn't exist, get() with no default should return None
        if !dict.contains(&py_key) {
            let result_none = dict.get_with_default(&py_key, PyValue::None);
            prop_assert!(matches!(result_none, PyValue::None),
                "get() should return None for missing key");
            
            // get() with default should return the default
            let result_default = dict.get_with_default(&py_key, PyValue::Int(default_value));
            if let PyValue::Int(val) = result_default {
                prop_assert_eq!(val, default_value,
                    "get() should return default for missing key");
            } else {
                prop_assert!(false, "get() should return default value");
            }
        }
    }

    // ===== Property 10: Dict Update Merge =====

    /// Feature: dx-py-production-ready-v2, Property 10: Dict Update Merge
    /// For any dicts d1 and d2, after d1.update(d2), for all keys k in d2, d1[k] should equal d2[k].
    /// Validates: Requirements 3.7
    #[test]
    fn prop_update_merges_all_keys_from_other(
        items1 in arb_int_int_dict(),
        items2 in arb_nonempty_int_int_dict()
    ) {
        let dict1 = create_int_dict(&items1);
        let dict2 = create_int_dict(&items2);
        
        dict1.update_from(&dict2);
        
        // All keys from dict2 should be in dict1 with dict2's values
        for (key, expected_value) in &items2 {
            let py_key = PyKey::Int(*key);
            let result = dict1.getitem(&py_key).expect("key from dict2 should exist in dict1");
            
            if let PyValue::Int(val) = result {
                prop_assert_eq!(val, *expected_value,
                    "updated dict should have values from dict2");
            } else {
                prop_assert!(false, "value should be Int");
            }
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 10: Dict Update Merge
    /// Update should override existing keys with values from the other dict.
    /// Validates: Requirements 3.7
    #[test]
    fn prop_update_overrides_existing_keys(key: i64, value1: i64, value2: i64) {
        let dict1 = PyDict::new();
        dict1.setitem(PyKey::Int(key), PyValue::Int(value1));
        
        let dict2 = PyDict::new();
        dict2.setitem(PyKey::Int(key), PyValue::Int(value2));
        
        dict1.update_from(&dict2);
        
        let result = dict1.getitem(&PyKey::Int(key)).expect("key should exist");
        if let PyValue::Int(val) = result {
            prop_assert_eq!(val, value2,
                "update should override existing key with new value");
        } else {
            prop_assert!(false, "value should be Int");
        }
    }

    // ===== Additional Property Tests =====

    /// Feature: dx-py-production-ready-v2, Property 9: Dict Get Consistency
    /// Pop should remove and return the value for existing key.
    /// Validates: Requirements 3.6
    #[test]
    fn prop_pop_removes_and_returns_value(items in arb_nonempty_int_int_dict()) {
        let dict = create_int_dict(&items);
        let (key, expected_value) = &items[0];
        let py_key = PyKey::Int(*key);
        let original_len = dict.len();
        
        let result = dict.pop_with_default(&py_key, None).expect("pop should succeed");
        
        if let PyValue::Int(val) = result {
            prop_assert_eq!(val, *expected_value,
                "pop should return the value");
        } else {
            prop_assert!(false, "pop should return Int");
        }
        
        // Key should no longer exist
        prop_assert!(!dict.contains(&py_key),
            "key should be removed after pop");
        
        // Length should decrease by 1
        prop_assert_eq!(dict.len(), original_len - 1,
            "pop should decrease length by 1");
    }

    /// Feature: dx-py-production-ready-v2, Property 10: Dict Update Merge
    /// Clear should empty the dict.
    /// Validates: Requirements 3.8
    #[test]
    fn prop_clear_empties_dict(items in arb_int_int_dict()) {
        let dict = create_int_dict(&items);
        
        dict.clear();
        
        prop_assert_eq!(dict.len(), 0,
            "clear should empty the dict");
        prop_assert!(dict.is_empty(),
            "dict should be empty after clear");
    }

    /// Feature: dx-py-production-ready-v2, Property 8: Dict Keys-Values-Items Consistency
    /// Copy should create an independent copy with same contents.
    /// Validates: Requirements 3.1-3.3
    #[test]
    fn prop_copy_creates_independent_copy(items in arb_int_int_dict()) {
        let dict = create_int_dict(&items);
        let copy = dict.copy();
        
        // Same length
        prop_assert_eq!(copy.len(), dict.len(),
            "copy should have same length");
        
        // Same contents
        for (key, value) in &items {
            let py_key = PyKey::Int(*key);
            let copy_value = copy.getitem(&py_key).expect("key should exist in copy");
            if let PyValue::Int(val) = copy_value {
                prop_assert_eq!(val, *value,
                    "copy should have same values");
            }
        }
        
        // Modifying copy shouldn't affect original
        copy.clear();
        prop_assert_eq!(dict.len(), items.len(),
            "clearing copy should not affect original");
    }

    /// Feature: dx-py-production-ready-v2, Property 9: Dict Get Consistency
    /// Setdefault should return existing value or set and return default.
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_setdefault_returns_existing_or_sets_default(
        items in arb_int_int_dict(),
        new_key: i64,
        default_value: i64
    ) {
        let dict = create_int_dict(&items);
        let py_key = PyKey::Int(new_key);
        
        let result = dict.setdefault(py_key.clone(), PyValue::Int(default_value));
        
        if items.iter().any(|(k, _)| *k == new_key) {
            // Key existed - should return existing value
            let existing = items.iter().find(|(k, _)| *k == new_key).unwrap().1;
            if let PyValue::Int(val) = result {
                prop_assert_eq!(val, existing,
                    "setdefault should return existing value");
            }
        } else {
            // Key didn't exist - should return default and set it
            if let PyValue::Int(val) = result {
                prop_assert_eq!(val, default_value,
                    "setdefault should return default for new key");
            }
            
            // Key should now exist with default value
            let stored = dict.getitem(&py_key).expect("key should exist after setdefault");
            if let PyValue::Int(val) = stored {
                prop_assert_eq!(val, default_value,
                    "setdefault should store default value");
            }
        }
    }
}

// ===== Unit tests for edge cases =====

#[test]
fn test_pop_missing_key_without_default_returns_error() {
    let dict = PyDict::new();
    dict.setitem(PyKey::Int(1), PyValue::Int(100));
    
    let result = dict.pop_with_default(&PyKey::Int(999), None);
    assert!(result.is_err(), "pop missing key without default should error");
}

#[test]
fn test_pop_missing_key_with_default_returns_default() {
    let dict = PyDict::new();
    dict.setitem(PyKey::Int(1), PyValue::Int(100));
    
    let result = dict.pop_with_default(&PyKey::Int(999), Some(PyValue::Int(42)));
    assert!(result.is_ok(), "pop missing key with default should succeed");
    if let PyValue::Int(val) = result.unwrap() {
        assert_eq!(val, 42, "should return default value");
    } else {
        panic!("should return Int");
    }
}

#[test]
fn test_popitem_empty_dict_returns_error() {
    let dict = PyDict::new();
    let result = dict.popitem();
    assert!(result.is_err(), "popitem on empty dict should error");
}

#[test]
fn test_popitem_returns_and_removes_item() {
    let dict = PyDict::new();
    dict.setitem(PyKey::Int(1), PyValue::Int(100));
    
    let original_len = dict.len();
    let result = dict.popitem();
    assert!(result.is_ok(), "popitem should succeed");
    
    let (key, value) = result.unwrap();
    assert!(matches!(key, PyKey::Int(1)), "should return the key");
    if let PyValue::Int(val) = value {
        assert_eq!(val, 100, "should return the value");
    } else {
        panic!("should return Int value");
    }
    
    assert_eq!(dict.len(), original_len - 1, "should decrease length");
}

#[test]
fn test_keys_empty_dict() {
    let dict = PyDict::new();
    let keys = dict.keys_list();
    if let PyValue::List(list) = keys {
        assert_eq!(list.len(), 0, "keys of empty dict should be empty list");
    } else {
        panic!("keys should return list");
    }
}

#[test]
fn test_values_empty_dict() {
    let dict = PyDict::new();
    let values = dict.values_list();
    if let PyValue::List(list) = values {
        assert_eq!(list.len(), 0, "values of empty dict should be empty list");
    } else {
        panic!("values should return list");
    }
}

#[test]
fn test_items_empty_dict() {
    let dict = PyDict::new();
    let items = dict.items_list();
    if let PyValue::List(list) = items {
        assert_eq!(list.len(), 0, "items of empty dict should be empty list");
    } else {
        panic!("items should return list");
    }
}

#[test]
fn test_update_empty_dict() {
    let dict1 = PyDict::new();
    let dict2 = PyDict::new();
    dict2.setitem(PyKey::Int(1), PyValue::Int(100));
    
    dict1.update_from(&dict2);
    
    assert_eq!(dict1.len(), 1, "update should add items from other dict");
}

#[test]
fn test_update_with_empty_dict() {
    let dict1 = PyDict::new();
    dict1.setitem(PyKey::Int(1), PyValue::Int(100));
    let original_len = dict1.len();
    
    let dict2 = PyDict::new();
    dict1.update_from(&dict2);
    
    assert_eq!(dict1.len(), original_len, "update with empty dict should not change length");
}
