//! Property-based tests for PyCell (closure variable cells)
//!
//! Feature: dx-py-vm-integration
//! Property 2: Closure Cell Preservation
//! Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5

use proptest::prelude::*;
use std::sync::Arc;

use dx_py_core::pylist::{PyCell, PyValue};

// ===== Generators for property tests =====

/// Generate arbitrary PyValue for testing
fn arb_pyvalue() -> impl Strategy<Value = PyValue> {
    prop_oneof![
        Just(PyValue::None),
        any::<bool>().prop_map(PyValue::Bool),
        any::<i64>().prop_map(PyValue::Int),
        any::<f64>()
            .prop_filter("finite float", |f| f.is_finite())
            .prop_map(PyValue::Float),
        "[a-zA-Z0-9_]{0,50}".prop_map(|s| PyValue::Str(Arc::from(s))),
    ]
}

/// Generate a sequence of PyValue updates
fn arb_value_sequence() -> impl Strategy<Value = Vec<PyValue>> {
    prop::collection::vec(arb_pyvalue(), 1..20)
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any value stored in a cell, getting the value SHALL return the same value.
    /// Validates: Requirements 2.1, 2.2
    #[test]
    fn prop_cell_get_returns_stored_value(value in arb_pyvalue()) {
        let cell = PyCell::new(value.clone());
        let retrieved = cell.get();

        prop_assert!(values_equal(&value, &retrieved),
            "Cell should return the stored value: expected {:?}, got {:?}", value, retrieved);
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any value set in a cell, subsequent gets SHALL return the new value.
    /// Validates: Requirements 2.2, 2.3
    #[test]
    fn prop_cell_set_updates_value(
        initial in arb_pyvalue(),
        updated in arb_pyvalue()
    ) {
        let cell = PyCell::new(initial);
        cell.set(updated.clone());
        let retrieved = cell.get();

        prop_assert!(values_equal(&updated, &retrieved),
            "Cell should return the updated value after set: expected {:?}, got {:?}", updated, retrieved);
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any sequence of updates to a cell, the final get SHALL return the last set value.
    /// Validates: Requirements 2.2, 2.3, 2.4
    #[test]
    fn prop_cell_preserves_last_value(values in arb_value_sequence()) {
        let cell = PyCell::new(PyValue::None);

        // Apply all updates
        for value in &values {
            cell.set(value.clone());
        }

        // The cell should contain the last value
        let last_value = values.last().unwrap();
        let retrieved = cell.get();

        prop_assert!(values_equal(last_value, &retrieved),
            "Cell should preserve the last set value: expected {:?}, got {:?}", last_value, retrieved);
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any cell shared between multiple references (simulating closure capture),
    /// updates through one reference SHALL be visible through all references.
    /// Validates: Requirements 2.4, 2.5
    #[test]
    fn prop_cell_shared_updates_visible(
        initial in arb_pyvalue(),
        updated in arb_pyvalue()
    ) {
        // Create a cell and share it (simulating closure capture)
        let cell = Arc::new(PyCell::new(initial));

        // Create multiple "closures" that reference the same cell
        let closure1_cell = Arc::clone(&cell);
        let closure2_cell = Arc::clone(&cell);

        // Update through one reference
        closure1_cell.set(updated.clone());

        // The update should be visible through the other reference
        let retrieved = closure2_cell.get();

        prop_assert!(values_equal(&updated, &retrieved),
            "Shared cell updates should be visible across all references: expected {:?}, got {:?}",
            updated, retrieved);
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any cell, the is_empty() method SHALL correctly reflect whether the cell contains None.
    /// Validates: Requirements 2.1
    #[test]
    fn prop_cell_is_empty_correct(value in arb_pyvalue()) {
        let cell = PyCell::new(value.clone());
        let is_empty = cell.is_empty();
        let should_be_empty = matches!(value, PyValue::None);

        prop_assert_eq!(is_empty, should_be_empty,
            "is_empty() should return {} for value {:?}", should_be_empty, value);
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any cell created with empty(), it SHALL initially be empty.
    /// Validates: Requirements 2.1
    #[test]
    fn prop_empty_cell_is_empty(_dummy in any::<u8>()) {
        let cell = PyCell::empty();
        prop_assert!(cell.is_empty(), "Empty cell should be empty");

        // Verify it contains None
        let value = cell.get();
        prop_assert!(matches!(value, PyValue::None), "Empty cell should contain None");
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// For any cell, setting a non-None value SHALL make is_empty() return false.
    /// Validates: Requirements 2.1, 2.3
    #[test]
    fn prop_cell_not_empty_after_set(value in arb_pyvalue().prop_filter("non-None", |v| !matches!(v, PyValue::None))) {
        let cell = PyCell::empty();
        prop_assert!(cell.is_empty(), "Cell should start empty");

        cell.set(value);
        prop_assert!(!cell.is_empty(), "Cell should not be empty after setting non-None value");
    }

    /// Feature: dx-py-vm-integration, Property 2: Closure Cell Preservation
    /// Simulates closure behavior: a cell captured by a closure should preserve
    /// updates made after the enclosing function returns.
    /// Validates: Requirements 2.4, 2.5
    #[test]
    fn prop_closure_captures_current_value(
        initial in any::<i64>(),
        updates in prop::collection::vec(any::<i64>(), 1..10)
    ) {
        // Simulate an enclosing function that creates a cell
        let captured_cell = Arc::new(PyCell::new(PyValue::Int(initial)));
        // The "enclosing function" has now "returned", but the cell lives on

        // Simulate multiple closures accessing and updating the cell
        for update_value in &updates {
            // One closure updates the value
            captured_cell.set(PyValue::Int(*update_value));

            // Another closure reads the value
            let read_value = captured_cell.get();

            // The read should see the update
            if let PyValue::Int(v) = read_value {
                prop_assert_eq!(v, *update_value,
                    "Closure should see updated value: expected {}, got {}", update_value, v);
            } else {
                prop_assert!(false, "Expected Int value");
            }
        }
    }
}

// ===== Helper functions =====

/// Compare two PyValues for equality (simplified comparison)
fn values_equal(a: &PyValue, b: &PyValue) -> bool {
    match (a, b) {
        (PyValue::None, PyValue::None) => true,
        (PyValue::Bool(x), PyValue::Bool(y)) => x == y,
        (PyValue::Int(x), PyValue::Int(y)) => x == y,
        (PyValue::Float(x), PyValue::Float(y)) => {
            // Handle NaN and floating point comparison
            (x.is_nan() && y.is_nan()) || (x == y)
        }
        (PyValue::Str(x), PyValue::Str(y)) => x == y,
        _ => false,
    }
}

// ===== Unit tests for specific scenarios =====

#[test]
fn test_cell_basic_operations() {
    let cell = PyCell::new(PyValue::Int(42));

    // Get initial value
    if let PyValue::Int(v) = cell.get() {
        assert_eq!(v, 42);
    } else {
        panic!("Expected Int");
    }

    // Set new value
    cell.set(PyValue::Int(100));
    if let PyValue::Int(v) = cell.get() {
        assert_eq!(v, 100);
    } else {
        panic!("Expected Int");
    }
}

#[test]
fn test_cell_shared_between_closures() {
    // Simulate two closures sharing a cell
    let cell = Arc::new(PyCell::new(PyValue::Int(0)));

    let closure1 = Arc::clone(&cell);
    let closure2 = Arc::clone(&cell);

    // Closure 1 increments
    if let PyValue::Int(v) = closure1.get() {
        closure1.set(PyValue::Int(v + 1));
    }

    // Closure 2 should see the update
    if let PyValue::Int(v) = closure2.get() {
        assert_eq!(v, 1, "Closure 2 should see the update from Closure 1");
    }

    // Closure 2 increments
    if let PyValue::Int(v) = closure2.get() {
        closure2.set(PyValue::Int(v + 1));
    }

    // Closure 1 should see the update
    if let PyValue::Int(v) = closure1.get() {
        assert_eq!(v, 2, "Closure 1 should see the update from Closure 2");
    }
}

#[test]
fn test_cell_survives_scope() {
    // Simulate a cell that outlives its creating scope
    let cell = Arc::new(PyCell::new(PyValue::Str(Arc::from("captured"))));

    // Cell should still be accessible
    if let PyValue::Str(s) = cell.get() {
        assert_eq!(&*s, "captured");
    } else {
        panic!("Expected Str");
    }
}

#[test]
fn test_cell_type_changes() {
    // Cell can hold different types over time
    let cell = PyCell::new(PyValue::Int(42));

    cell.set(PyValue::Str(Arc::from("hello")));
    if let PyValue::Str(s) = cell.get() {
        assert_eq!(&*s, "hello");
    } else {
        panic!("Expected Str");
    }

    cell.set(PyValue::Bool(true));
    if let PyValue::Bool(b) = cell.get() {
        assert!(b);
    } else {
        panic!("Expected Bool");
    }
}
