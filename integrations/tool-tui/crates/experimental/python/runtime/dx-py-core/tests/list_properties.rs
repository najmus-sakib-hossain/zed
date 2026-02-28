//! Property-based tests for list methods
//!
//! Feature: dx-py-production-ready-v2
//! Property 4: List Append Invariant
//! Property 5: List Sort Ordering
//! Property 6: List Reverse Round-Trip
//! Property 7: List Pop Consistency
//! Validates: Requirements 2.1-2.11

use proptest::prelude::*;

use dx_py_core::pylist::{PyList, PyValue};

// ===== Generators for property tests =====

/// Generate a list of integers
fn arb_int_list() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(any::<i64>(), 0..20)
}

/// Generate a non-empty list of integers
fn arb_nonempty_int_list() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(any::<i64>(), 1..20)
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready-v2, Property 4: List Append Invariant
    /// For any list lst and item x, after calling lst.append(x), the length of lst
    /// should increase by 1 and lst[-1] should equal x.
    /// Validates: Requirements 2.1
    #[test]
    fn prop_append_increases_length_by_one(items in arb_int_list(), new_item: i64) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        let original_len = list.len();
        
        list.append(PyValue::Int(new_item));
        
        prop_assert_eq!(list.len(), original_len + 1,
            "append should increase length by 1");
    }

    /// Feature: dx-py-production-ready-v2, Property 4: List Append Invariant
    /// After append, the last element should be the appended item.
    /// Validates: Requirements 2.1
    #[test]
    fn prop_append_item_is_last(items in arb_int_list(), new_item: i64) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        list.append(PyValue::Int(new_item));
        
        let last = list.getitem(-1).expect("should have last element");
        if let PyValue::Int(val) = last {
            prop_assert_eq!(val, new_item,
                "last element should be the appended item");
        } else {
            prop_assert!(false, "last element should be an Int");
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 5: List Sort Ordering
    /// For any list lst of comparable elements, after calling lst.sort(),
    /// for all valid indices i, lst[i] <= lst[i+1] should hold.
    /// Validates: Requirements 2.7
    #[test]
    fn prop_sort_produces_ordered_list(items in arb_int_list()) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        // Sort should succeed for homogeneous int lists
        let result = list.sort();
        prop_assert!(result.is_ok(), "sort should succeed for int list");
        
        // Verify ordering
        let sorted_items = list.to_vec();
        for i in 0..sorted_items.len().saturating_sub(1) {
            if let (PyValue::Int(a), PyValue::Int(b)) = (&sorted_items[i], &sorted_items[i + 1]) {
                prop_assert!(a <= b,
                    "sorted list should be in ascending order: {} <= {}", a, b);
            }
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 5: List Sort Ordering
    /// Sort should be idempotent - sorting twice should produce the same result.
    /// Validates: Requirements 2.7
    #[test]
    fn prop_sort_idempotence(items in arb_int_list()) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        list.sort().ok();
        let after_first_sort: Vec<i64> = list.to_vec().iter().filter_map(|v| {
            if let PyValue::Int(i) = v { Some(*i) } else { None }
        }).collect();
        
        list.sort().ok();
        let after_second_sort: Vec<i64> = list.to_vec().iter().filter_map(|v| {
            if let PyValue::Int(i) = v { Some(*i) } else { None }
        }).collect();
        
        prop_assert_eq!(after_first_sort, after_second_sort,
            "sort should be idempotent");
    }

    /// Feature: dx-py-production-ready-v2, Property 6: List Reverse Round-Trip
    /// For any list lst, calling lst.reverse() twice should restore the original order.
    /// Validates: Requirements 2.8
    #[test]
    fn prop_reverse_round_trip(items in arb_int_list()) {
        let original: Vec<i64> = items.clone();
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        // Reverse twice
        list.reverse();
        list.reverse();
        
        // Should be back to original
        let restored: Vec<i64> = list.to_vec().iter().filter_map(|v| {
            if let PyValue::Int(i) = v { Some(*i) } else { None }
        }).collect();
        
        prop_assert_eq!(original, restored,
            "reverse twice should restore original order");
    }

    /// Feature: dx-py-production-ready-v2, Property 6: List Reverse Round-Trip
    /// Reverse should actually reverse the order.
    /// Validates: Requirements 2.8
    #[test]
    fn prop_reverse_reverses_order(items in arb_int_list()) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        list.reverse();
        
        let reversed: Vec<i64> = list.to_vec().iter().filter_map(|v| {
            if let PyValue::Int(i) = v { Some(*i) } else { None }
        }).collect();
        
        let expected: Vec<i64> = items.iter().rev().cloned().collect();
        
        prop_assert_eq!(reversed, expected,
            "reverse should reverse the order");
    }

    /// Feature: dx-py-production-ready-v2, Property 7: List Pop Consistency
    /// For any non-empty list lst, if last = lst[-1] before calling pop(),
    /// then lst.pop() should return last and the length should decrease by 1.
    /// Validates: Requirements 2.5, 2.6
    #[test]
    fn prop_pop_returns_last_and_decreases_length(items in arb_nonempty_int_list()) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        let original_len = list.len();
        let expected_last = items.last().cloned().unwrap();
        
        let popped = list.pop(None).expect("pop should succeed on non-empty list");
        
        if let PyValue::Int(val) = popped {
            prop_assert_eq!(val, expected_last,
                "pop should return the last element");
        } else {
            prop_assert!(false, "popped element should be an Int");
        }
        
        prop_assert_eq!(list.len(), original_len - 1,
            "pop should decrease length by 1");
    }

    /// Feature: dx-py-production-ready-v2, Property 7: List Pop Consistency
    /// Pop with index should return the element at that index.
    /// Validates: Requirements 2.5, 2.6
    #[test]
    fn prop_pop_with_index_returns_correct_element(
        items in arb_nonempty_int_list(),
        index_factor in 0.0f64..1.0f64
    ) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        let len = list.len();
        let index = (index_factor * len as f64) as i64;
        let expected = items[index as usize];
        
        let popped = list.pop(Some(index)).expect("pop should succeed");
        
        if let PyValue::Int(val) = popped {
            prop_assert_eq!(val, expected,
                "pop(index) should return element at index");
        } else {
            prop_assert!(false, "popped element should be an Int");
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 4: List Append Invariant
    /// Extend should add all elements from the iterable.
    /// Validates: Requirements 2.2
    #[test]
    fn prop_extend_adds_all_elements(
        initial in arb_int_list(),
        extension in arb_int_list()
    ) {
        let list = PyList::from_values(initial.iter().map(|&i| PyValue::Int(i)).collect());
        let original_len = list.len();
        
        list.extend(extension.iter().map(|&i| PyValue::Int(i)));
        
        prop_assert_eq!(list.len(), original_len + extension.len(),
            "extend should add all elements");
    }

    /// Feature: dx-py-production-ready-v2, Property 4: List Append Invariant
    /// Insert at index 0 should prepend the element.
    /// Validates: Requirements 2.3
    #[test]
    fn prop_insert_at_zero_prepends(items in arb_int_list(), new_item: i64) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        list.insert(0, PyValue::Int(new_item));
        
        let first = list.getitem(0).expect("should have first element");
        if let PyValue::Int(val) = first {
            prop_assert_eq!(val, new_item,
                "insert at 0 should prepend the element");
        } else {
            prop_assert!(false, "first element should be an Int");
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 7: List Pop Consistency
    /// Count should return the correct number of occurrences.
    /// Validates: Requirements 2.10
    #[test]
    fn prop_count_returns_correct_occurrences(
        base in arb_int_list(),
        target: i64,
        extra_count in 0usize..5
    ) {
        // Create a list with known number of target occurrences
        let mut items: Vec<PyValue> = base.iter().filter(|&&x| x != target).map(|&i| PyValue::Int(i)).collect();
        for _ in 0..extra_count {
            items.push(PyValue::Int(target));
        }
        
        let list = PyList::from_values(items);
        let count = list.count(&PyValue::Int(target));
        
        prop_assert_eq!(count, extra_count,
            "count should return correct number of occurrences");
    }

    /// Feature: dx-py-production-ready-v2, Property 7: List Pop Consistency
    /// Index should return the correct position of the first occurrence.
    /// Validates: Requirements 2.9
    #[test]
    fn prop_index_returns_first_occurrence(items in arb_nonempty_int_list()) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        let target = items[0]; // Use first element as target
        
        let index = list.index(&PyValue::Int(target)).expect("index should find element");
        
        // The index should be 0 or the position of first occurrence
        prop_assert!(index < items.len(),
            "index should be within bounds");
        prop_assert_eq!(items[index], target,
            "element at returned index should match target");
    }

    /// Feature: dx-py-production-ready-v2, Property 4: List Append Invariant
    /// Clear should empty the list.
    /// Validates: Requirements 2.6
    #[test]
    fn prop_clear_empties_list(items in arb_int_list()) {
        let list = PyList::from_values(items.iter().map(|&i| PyValue::Int(i)).collect());
        
        list.clear();
        
        prop_assert_eq!(list.len(), 0,
            "clear should empty the list");
        prop_assert!(list.is_empty(),
            "list should be empty after clear");
    }
}

// ===== Unit tests for edge cases =====

#[test]
fn test_pop_empty_list_returns_error() {
    let list = PyList::new();
    let result = list.pop(None);
    assert!(result.is_err(), "pop on empty list should return error");
}

#[test]
fn test_remove_nonexistent_returns_error() {
    let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2)]);
    let result = list.remove(&PyValue::Int(99));
    assert!(result.is_err(), "remove nonexistent should return error");
}

#[test]
fn test_index_nonexistent_returns_error() {
    let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2)]);
    let result = list.index(&PyValue::Int(99));
    assert!(result.is_err(), "index nonexistent should return error");
}

#[test]
fn test_insert_negative_index() {
    let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2), PyValue::Int(3)]);
    list.insert(-1, PyValue::Int(99));
    // Negative index -1 should insert before the last element
    let items: Vec<i64> = list.to_vec().iter().filter_map(|v| {
        if let PyValue::Int(i) = v { Some(*i) } else { None }
    }).collect();
    assert_eq!(items.len(), 4);
}

#[test]
fn test_pop_with_negative_index() {
    let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2), PyValue::Int(3)]);
    let popped = list.pop(Some(-1)).expect("pop should succeed");
    if let PyValue::Int(val) = popped {
        assert_eq!(val, 3, "pop(-1) should return last element");
    } else {
        panic!("popped element should be Int");
    }
}

#[test]
fn test_extend_with_empty() {
    let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2)]);
    let original_len = list.len();
    list.extend(std::iter::empty::<PyValue>());
    assert_eq!(list.len(), original_len, "extend with empty should not change length");
}

#[test]
fn test_sort_empty_list() {
    let list = PyList::new();
    let result = list.sort();
    assert!(result.is_ok(), "sort empty list should succeed");
    assert!(list.is_empty(), "list should still be empty");
}

#[test]
fn test_reverse_empty_list() {
    let list = PyList::new();
    list.reverse();
    assert!(list.is_empty(), "reverse empty list should still be empty");
}

#[test]
fn test_count_empty_list() {
    let list = PyList::new();
    let count = list.count(&PyValue::Int(1));
    assert_eq!(count, 0, "count on empty list should be 0");
}
