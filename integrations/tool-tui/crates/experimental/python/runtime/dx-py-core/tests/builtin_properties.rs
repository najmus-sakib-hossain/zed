//! Property-based tests for builtin functions
//!
//! Feature: dx-py-vm-integration
//! Property 9: Builtin Function Correctness
//! Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8

use proptest::prelude::*;
use std::sync::Arc;

use dx_py_core::builtins::*;
use dx_py_core::pylist::PyValue;
use dx_py_core::PyList;

// ===== Generators for property tests =====

/// Generate arbitrary integers for testing
fn arb_int() -> impl Strategy<Value = i64> {
    -1000i64..1000i64
}

/// Generate arbitrary non-empty list of integers
fn arb_int_list() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(arb_int(), 1..20)
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list of integers, min() SHALL return the smallest value.
    /// Validates: Requirements 9.7
    #[test]
    fn prop_min_returns_smallest(values in arb_int_list()) {
        let min_fn = builtin_min();

        // Convert to PyValue args
        let args: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();

        let result = min_fn.call(&args).unwrap();
        let expected_min = *values.iter().min().unwrap();

        if let PyValue::Int(actual_min) = result {
            prop_assert_eq!(actual_min, expected_min,
                "min() should return the smallest value: expected {}, got {}", expected_min, actual_min);
        } else {
            prop_assert!(false, "min() should return an Int");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list of integers, max() SHALL return the largest value.
    /// Validates: Requirements 9.7
    #[test]
    fn prop_max_returns_largest(values in arb_int_list()) {
        let max_fn = builtin_max();

        // Convert to PyValue args
        let args: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();

        let result = max_fn.call(&args).unwrap();
        let expected_max = *values.iter().max().unwrap();

        if let PyValue::Int(actual_max) = result {
            prop_assert_eq!(actual_max, expected_max,
                "max() should return the largest value: expected {}, got {}", expected_max, actual_max);
        } else {
            prop_assert!(false, "max() should return an Int");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list of integers, sum() SHALL return the sum of all values.
    /// Validates: Requirements 9.7
    #[test]
    fn prop_sum_returns_total(values in arb_int_list()) {
        let sum_fn = builtin_sum();

        // Create a PyList
        let py_values: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();
        let list = PyValue::List(Arc::new(PyList::from_values(py_values)));

        let result = sum_fn.call(&[list]).unwrap();
        let expected_sum: i64 = values.iter().sum();

        if let PyValue::Int(actual_sum) = result {
            prop_assert_eq!(actual_sum, expected_sum,
                "sum() should return the sum of all values: expected {}, got {}", expected_sum, actual_sum);
        } else {
            prop_assert!(false, "sum() should return an Int");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list of integers, sorted() SHALL return a list in ascending order.
    /// Validates: Requirements 9.7
    #[test]
    fn prop_sorted_returns_ascending(values in arb_int_list()) {
        let sorted_fn = builtin_sorted();

        // Create a PyList
        let py_values: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();
        let list = PyValue::List(Arc::new(PyList::from_values(py_values)));

        let result = sorted_fn.call(&[list]).unwrap();

        if let PyValue::List(sorted_list) = result {
            let sorted_values: Vec<i64> = sorted_list.to_vec().iter().filter_map(|v| {
                if let PyValue::Int(i) = v { Some(*i) } else { None }
            }).collect();

            // Check that the result is sorted
            for i in 1..sorted_values.len() {
                prop_assert!(sorted_values[i-1] <= sorted_values[i],
                    "sorted() should return values in ascending order");
            }

            // Check that the result has the same length
            prop_assert_eq!(sorted_values.len(), values.len(),
                "sorted() should preserve the number of elements");
        } else {
            prop_assert!(false, "sorted() should return a List");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list, reversed() SHALL return a list in reverse order.
    /// Validates: Requirements 9.7
    #[test]
    fn prop_reversed_returns_reverse(values in arb_int_list()) {
        let reversed_fn = builtin_reversed();

        // Create a PyList
        let py_values: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();
        let list = PyValue::List(Arc::new(PyList::from_values(py_values)));

        let result = reversed_fn.call(&[list]).unwrap();

        if let PyValue::List(reversed_list) = result {
            let reversed_values: Vec<i64> = reversed_list.to_vec().iter().filter_map(|v| {
                if let PyValue::Int(i) = v { Some(*i) } else { None }
            }).collect();

            // Check that the result is reversed
            let mut expected: Vec<i64> = values.clone();
            expected.reverse();

            prop_assert_eq!(reversed_values, expected,
                "reversed() should return values in reverse order");
        } else {
            prop_assert!(false, "reversed() should return a List");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list, enumerate() SHALL return (index, value) pairs starting from 0.
    /// Validates: Requirements 9.6
    #[test]
    fn prop_enumerate_returns_indexed_pairs(values in arb_int_list()) {
        let enumerate_fn = builtin_enumerate();

        // Create a PyList
        let py_values: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();
        let list = PyValue::List(Arc::new(PyList::from_values(py_values)));

        let result = enumerate_fn.call(&[list]).unwrap();

        if let PyValue::List(enum_list) = result {
            let items = enum_list.to_vec();

            prop_assert_eq!(items.len(), values.len(),
                "enumerate() should return same number of items");

            for (i, item) in items.iter().enumerate() {
                if let PyValue::Tuple(t) = item {
                    let tuple_items = t.to_vec();
                    prop_assert_eq!(tuple_items.len(), 2,
                        "enumerate() should return 2-tuples");

                    if let PyValue::Int(idx) = &tuple_items[0] {
                        prop_assert_eq!(*idx, i as i64,
                            "enumerate() index should match position");
                    }

                    if let PyValue::Int(val) = &tuple_items[1] {
                        prop_assert_eq!(*val, values[i],
                            "enumerate() value should match original");
                    }
                }
            }
        } else {
            prop_assert!(false, "enumerate() should return a List");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any two lists, zip() SHALL return pairs up to the shorter list's length.
    /// Validates: Requirements 9.6
    #[test]
    fn prop_zip_returns_pairs(
        values1 in arb_int_list(),
        values2 in arb_int_list()
    ) {
        let zip_fn = builtin_zip();

        // Create PyLists
        let py_values1: Vec<PyValue> = values1.iter().map(|&i| PyValue::Int(i)).collect();
        let py_values2: Vec<PyValue> = values2.iter().map(|&i| PyValue::Int(i)).collect();
        let list1 = PyValue::List(Arc::new(PyList::from_values(py_values1)));
        let list2 = PyValue::List(Arc::new(PyList::from_values(py_values2)));

        let result = zip_fn.call(&[list1, list2]).unwrap();

        if let PyValue::List(zipped_list) = result {
            let items = zipped_list.to_vec();
            let expected_len = values1.len().min(values2.len());

            prop_assert_eq!(items.len(), expected_len,
                "zip() should return min(len1, len2) items");

            for (i, item) in items.iter().enumerate() {
                if let PyValue::Tuple(t) = item {
                    let tuple_items = t.to_vec();
                    prop_assert_eq!(tuple_items.len(), 2,
                        "zip() should return 2-tuples");

                    if let (PyValue::Int(v1), PyValue::Int(v2)) = (&tuple_items[0], &tuple_items[1]) {
                        prop_assert_eq!(*v1, values1[i],
                            "zip() first value should match");
                        prop_assert_eq!(*v2, values2[i],
                            "zip() second value should match");
                    }
                }
            }
        } else {
            prop_assert!(false, "zip() should return a List");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any integer, str() SHALL return its string representation.
    /// Validates: Requirements 9.2 (getattr/setattr pattern - type conversion)
    #[test]
    fn prop_str_converts_int(value in arb_int()) {
        let str_fn = builtin_str();

        let result = str_fn.call(&[PyValue::Int(value)]).unwrap();

        if let PyValue::Str(s) = result {
            let expected = value.to_string();
            prop_assert_eq!(s.as_ref(), expected.as_str(),
                "str() should convert int to string: expected '{}', got '{}'", expected, s);
        } else {
            prop_assert!(false, "str() should return a Str");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any valid integer string, int() SHALL parse it correctly.
    /// Validates: Requirements 9.2 (type conversion)
    #[test]
    fn prop_int_parses_string(value in arb_int()) {
        let int_fn = builtin_int();

        let str_value = PyValue::Str(Arc::from(value.to_string()));
        let result = int_fn.call(&[str_value]).unwrap();

        if let PyValue::Int(parsed) = result {
            prop_assert_eq!(parsed, value,
                "int() should parse string to int: expected {}, got {}", value, parsed);
        } else {
            prop_assert!(false, "int() should return an Int");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any integer, abs() SHALL return its absolute value.
    /// Validates: Requirements 9.7
    #[test]
    fn prop_abs_returns_absolute(value in arb_int()) {
        let abs_fn = builtin_abs();

        let result = abs_fn.call(&[PyValue::Int(value)]).unwrap();

        if let PyValue::Int(abs_value) = result {
            prop_assert_eq!(abs_value, value.abs(),
                "abs() should return absolute value: expected {}, got {}", value.abs(), abs_value);
        } else {
            prop_assert!(false, "abs() should return an Int");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any list, len() SHALL return its length.
    /// Validates: Requirements 9.2
    #[test]
    fn prop_len_returns_length(values in arb_int_list()) {
        let len_fn = builtin_len();

        let py_values: Vec<PyValue> = values.iter().map(|&i| PyValue::Int(i)).collect();
        let list = PyValue::List(Arc::new(PyList::from_values(py_values)));

        let result = len_fn.call(&[list]).unwrap();

        if let PyValue::Int(length) = result {
            prop_assert_eq!(length as usize, values.len(),
                "len() should return list length: expected {}, got {}", values.len(), length);
        } else {
            prop_assert!(false, "len() should return an Int");
        }
    }

    /// Feature: dx-py-vm-integration, Property 9: Builtin Function Correctness
    /// For any value, bool() SHALL return its truthiness.
    /// Validates: Requirements 9.2
    #[test]
    fn prop_bool_returns_truthiness(value in arb_int()) {
        let bool_fn = builtin_bool();

        let result = bool_fn.call(&[PyValue::Int(value)]).unwrap();

        if let PyValue::Bool(b) = result {
            let expected = value != 0;
            prop_assert_eq!(b, expected,
                "bool() should return truthiness: expected {}, got {}", expected, b);
        } else {
            prop_assert!(false, "bool() should return a Bool");
        }
    }
}
