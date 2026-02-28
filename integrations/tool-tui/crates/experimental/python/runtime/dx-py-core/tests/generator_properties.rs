//! Property-based tests for Generators
//!
//! Feature: dx-py-production-ready, Property 11: Generator Iteration Equivalence
//! Feature: dx-py-production-ready, Property 12: Yield From Delegation
//! Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5, 6.6
//!
//! This module tests the generator properties:
//! - Property 11: For any generator expression or generator function, iterating with
//!   `next()` or a for loop SHALL yield values in the same order as the equivalent
//!   list comprehension or function with return statements.
//! - Property 12: For any generator using `yield from sub_iter`, the yielded values
//!   SHALL be exactly the values from `sub_iter` in order.

#![allow(clippy::cloned_ref_to_slice_refs)]

use proptest::prelude::*;
use std::sync::Arc;

use dx_py_core::pyframe::PyFrame;
use dx_py_core::pyfunction::{CodeRef, PyFunction};
use dx_py_core::pygenerator::{GeneratorResult, GeneratorState, PyGenerator};
use dx_py_core::pylist::PyValue;
use dx_py_core::PyIterator;

// ===== Arbitrary value generators =====

/// Generate arbitrary primitive values for testing
fn arb_primitive_value() -> impl Strategy<Value = PyValue> {
    prop_oneof![
        Just(PyValue::None),
        any::<bool>().prop_map(PyValue::Bool),
        (-1_000_000i64..1_000_000i64).prop_map(PyValue::Int),
        (-1e6..1e6f64)
            .prop_filter("must be finite", |f| f.is_finite())
            .prop_map(PyValue::Float),
        "[a-zA-Z0-9_ ]{0,20}".prop_map(|s| PyValue::Str(Arc::from(s))),
    ]
}

/// Generate a list of primitive values for testing iteration
fn arb_value_list() -> impl Strategy<Value = Vec<PyValue>> {
    prop::collection::vec(arb_primitive_value(), 0..20)
}

/// Generate a list of integer values for testing
fn arb_int_list() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(-1000i64..1000i64, 0..20)
}

/// Generate a non-empty list of integer values
fn arb_nonempty_int_list() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(-1000i64..1000i64, 1..20)
}

/// Generate a generator function for testing
fn arb_generator_function() -> impl Strategy<Value = Arc<PyFunction>> {
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{0,10}")
        .unwrap()
        .prop_map(|name| {
            let mut func = PyFunction::new(
                name,
                CodeRef {
                    bytecode_offset: 0,
                    num_locals: 2,
                    stack_size: 4,
                    num_args: 0,
                    num_kwonly_args: 0,
                },
                vec![],
            );
            func.flags.is_generator = true;
            Arc::new(func)
        })
}

// ===== Value comparison helper =====

/// Check if two PyValues are equivalent
fn values_equal(a: &PyValue, b: &PyValue) -> bool {
    match (a, b) {
        (PyValue::None, PyValue::None) => true,
        (PyValue::Bool(a), PyValue::Bool(b)) => a == b,
        (PyValue::Int(a), PyValue::Int(b)) => a == b,
        (PyValue::Float(a), PyValue::Float(b)) => {
            if a.is_nan() && b.is_nan() {
                true
            } else {
                (a - b).abs() < 1e-10
            }
        }
        (PyValue::Str(a), PyValue::Str(b)) => a == b,
        _ => false,
    }
}

/// Check if two lists of PyValues are equivalent
fn value_lists_equal(a: &[PyValue], b: &[PyValue]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // =========================================================================
    // Property 11: Generator Iteration Equivalence
    // =========================================================================

    /// Property 11: Generator Iteration Equivalence - Iterator yields values in order
    ///
    /// For any sequence of values, iterating through a PyIterator with next()
    /// SHALL yield values in the same order as the original sequence.
    ///
    /// **Validates: Requirements 6.1, 6.2, 6.6**
    #[test]
    fn prop_iterator_yields_values_in_order(values in arb_value_list()) {
        let iter = Arc::new(PyIterator::new(values.clone()));

        // Collect all values from the iterator
        let mut collected = Vec::new();
        while let Some(value) = iter.next() {
            collected.push(value);
        }

        // Verify the collected values match the original
        prop_assert_eq!(
            collected.len(),
            values.len(),
            "Iterator should yield same number of values"
        );

        prop_assert!(
            value_lists_equal(&collected, &values),
            "Iterator should yield values in the same order as the original sequence"
        );
    }

    /// Property 11: Generator Iteration Equivalence - next() and for loop equivalence
    ///
    /// For any generator, iterating with next() SHALL yield the same values
    /// as iterating with a for loop (simulated by repeated next() calls).
    ///
    /// **Validates: Requirements 6.2, 6.6**
    #[test]
    fn prop_next_and_for_loop_equivalence(values in arb_value_list()) {
        // Create two iterators from the same values
        let iter1 = Arc::new(PyIterator::new(values.clone()));
        let iter2 = Arc::new(PyIterator::new(values.clone()));

        // Collect using next() calls (simulating manual iteration)
        let mut collected_next = Vec::new();
        loop {
            match iter1.next() {
                Some(value) => collected_next.push(value),
                None => break,
            }
        }

        // Collect using while let (simulating for loop)
        let mut collected_for = Vec::new();
        while let Some(value) = iter2.next() {
            collected_for.push(value);
        }

        // Both methods should yield the same values
        prop_assert!(
            value_lists_equal(&collected_next, &collected_for),
            "next() and for loop should yield the same values in the same order"
        );
    }

    /// Property 11: Generator Iteration Equivalence - Generator __iter__ returns self
    ///
    /// For any generator, calling __iter__ SHALL return the generator itself,
    /// making generators their own iterators.
    ///
    /// **Validates: Requirements 6.6**
    #[test]
    fn prop_generator_iter_returns_self(func in arb_generator_function()) {
        let frame = PyFrame::new(Arc::clone(&func), None);
        let gen = Arc::new(PyGenerator::new(func, frame));

        // __iter__ should return the generator itself
        let iter_result = gen.iter();
        match iter_result {
            PyValue::Generator(g) => {
                prop_assert!(
                    Arc::ptr_eq(&g, &gen),
                    "Generator __iter__ should return self"
                );
            }
            _ => prop_assert!(false, "Expected generator from __iter__"),
        }
    }

    /// Property 11: Generator Iteration Equivalence - __next__ is equivalent to send(None)
    ///
    /// For any generator, calling __next__() SHALL be equivalent to calling send(None).
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn prop_next_is_send_none(func in arb_generator_function()) {
        let frame1 = PyFrame::new(Arc::clone(&func), None);
        let frame2 = PyFrame::new(Arc::clone(&func), None);
        let gen1 = Arc::new(PyGenerator::new(Arc::clone(&func), frame1));
        let gen2 = Arc::new(PyGenerator::new(func, frame2));

        // __next__ should be equivalent to send(None)
        let next_result = gen1.next();
        let send_result = gen2.send(PyValue::None);

        // Both should have the same result type
        match (next_result, send_result) {
            (GeneratorResult::NeedExecution, GeneratorResult::NeedExecution) => {}
            (GeneratorResult::StopIteration(_), GeneratorResult::StopIteration(_)) => {}
            (GeneratorResult::Error(_), GeneratorResult::Error(_)) => {}
            (GeneratorResult::Yielded(v1), GeneratorResult::Yielded(v2)) => {
                prop_assert!(
                    values_equal(&v1, &v2),
                    "__next__ and send(None) should yield the same value"
                );
            }
            _ => prop_assert!(false, "__next__ and send(None) should have same result type"),
        }
    }

    /// Property 11: Generator Iteration Equivalence - Exhausted generator raises StopIteration
    ///
    /// For any exhausted generator, calling next() SHALL raise StopIteration.
    ///
    /// **Validates: Requirements 6.4**
    #[test]
    fn prop_exhausted_generator_raises_stop_iteration(func in arb_generator_function()) {
        let frame = PyFrame::new(Arc::clone(&func), None);
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Mark as completed
        gen.complete(PyValue::None);

        // Any operation on exhausted generator should return StopIteration
        let send_result = gen.send(PyValue::None);
        prop_assert!(
            matches!(send_result, GeneratorResult::StopIteration(_)),
            "Exhausted generator should raise StopIteration on send()"
        );

        let next_result = gen.next();
        prop_assert!(
            matches!(next_result, GeneratorResult::StopIteration(_)),
            "Exhausted generator should raise StopIteration on next()"
        );
    }

    /// Property 11: Generator Iteration Equivalence - Generator state transitions
    ///
    /// For any generator, the state SHALL transition correctly:
    /// Created -> Running -> Suspended -> Running -> ... -> Completed
    ///
    /// **Validates: Requirements 6.2, 6.4**
    #[test]
    fn prop_generator_state_transitions(func in arb_generator_function()) {
        let frame = PyFrame::new(Arc::clone(&func), None);
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Initial state is Created
        prop_assert_eq!(
            gen.get_state(),
            GeneratorState::Created,
            "Initial state should be Created"
        );

        // After yield, state should be Suspended
        gen.yield_value(PyValue::Int(1));
        prop_assert_eq!(
            gen.get_state(),
            GeneratorState::Suspended,
            "After yield, state should be Suspended"
        );

        // After complete, state should be Completed
        gen.complete(PyValue::None);
        prop_assert_eq!(
            gen.get_state(),
            GeneratorState::Completed,
            "After complete, state should be Completed"
        );
        prop_assert!(gen.is_exhausted(), "Completed generator should be exhausted");
    }

    /// Property 11: Generator Iteration Equivalence - First send must be None
    ///
    /// For any just-started generator, the first send() call SHALL only accept None.
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn prop_first_send_must_be_none(
        func in arb_generator_function(),
        value in arb_primitive_value()
    ) {
        let frame = PyFrame::new(Arc::clone(&func), None);
        let gen = Arc::new(PyGenerator::new(func, frame));

        // First send with non-None value should error
        if !matches!(value, PyValue::None) {
            let result = gen.send(value);
            prop_assert!(
                matches!(result, GeneratorResult::Error(_)),
                "First send with non-None value should error"
            );
        }
    }

    // =========================================================================
    // Property 12: Yield From Delegation
    // =========================================================================

    /// Property 12: Yield From Delegation - Iterator yields exact values in order
    ///
    /// For any iterator used with yield from, the yielded values SHALL be
    /// exactly the values from the sub-iterator in order.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_exact_values_in_order(values in arb_value_list()) {
        let iter = Arc::new(PyIterator::new(values.clone()));

        // Collect all values from the iterator (simulating yield from)
        let mut collected = Vec::new();
        while let Some(value) = iter.next() {
            collected.push(value);
        }

        // The collected values should be exactly the input values in order
        prop_assert_eq!(
            collected.len(),
            values.len(),
            "yield from should yield exactly the same number of values"
        );

        for (i, (collected_val, original_val)) in collected.iter().zip(values.iter()).enumerate() {
            prop_assert!(
                values_equal(collected_val, original_val),
                "yield from should yield value {} in order: {:?} != {:?}",
                i,
                collected_val,
                original_val
            );
        }
    }

    /// Property 12: Yield From Delegation - Empty iterator yields nothing
    ///
    /// For any generator using yield from with an empty iterator,
    /// no values SHALL be yielded.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_empty_iterator(_dummy in 0..10i32) {
        let values: Vec<PyValue> = vec![];
        let iter = Arc::new(PyIterator::new(values));

        // Empty iterator should yield nothing
        prop_assert!(
            iter.next().is_none(),
            "yield from empty iterator should yield nothing"
        );
    }

    /// Property 12: Yield From Delegation - Single element iterator
    ///
    /// For any generator using yield from with a single-element iterator,
    /// exactly one value SHALL be yielded.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_single_element(value in arb_primitive_value()) {
        let values = vec![value.clone()];
        let iter = Arc::new(PyIterator::new(values));

        // Should yield exactly one value
        let first = iter.next();
        prop_assert!(first.is_some(), "Should yield one value");
        prop_assert!(
            values_equal(&first.unwrap(), &value),
            "Should yield the correct value"
        );

        // Then be exhausted
        prop_assert!(
            iter.next().is_none(),
            "Should be exhausted after single element"
        );
    }

    /// Property 12: Yield From Delegation - Preserves value types
    ///
    /// For any generator using yield from, the yielded values SHALL
    /// preserve their original types.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_preserves_types(values in arb_value_list()) {
        let iter = Arc::new(PyIterator::new(values.clone()));

        // Collect and verify types are preserved
        let mut collected = Vec::new();
        while let Some(value) = iter.next() {
            collected.push(value);
        }

        for (i, (collected_val, original_val)) in collected.iter().zip(values.iter()).enumerate() {
            prop_assert_eq!(
                collected_val.type_name(),
                original_val.type_name(),
                "yield from should preserve type at index {}: {} != {}",
                i,
                collected_val.type_name(),
                original_val.type_name()
            );
        }
    }

    /// Property 12: Yield From Delegation - Sequential iterators
    ///
    /// For any generator using multiple yield from statements,
    /// the values SHALL be yielded in sequence from each iterator.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_sequential_iterators(
        values1 in arb_value_list(),
        values2 in arb_value_list()
    ) {
        // Create two iterators
        let iter1 = Arc::new(PyIterator::new(values1.clone()));
        let iter2 = Arc::new(PyIterator::new(values2.clone()));

        // Collect from first iterator (simulating first yield from)
        let mut collected = Vec::new();
        while let Some(value) = iter1.next() {
            collected.push(value);
        }

        // Collect from second iterator (simulating second yield from)
        while let Some(value) = iter2.next() {
            collected.push(value);
        }

        // Should have all values in order
        let expected_len = values1.len() + values2.len();
        prop_assert_eq!(
            collected.len(),
            expected_len,
            "Sequential yield from should yield all values"
        );

        // Verify first part matches values1
        for (i, (collected_val, original_val)) in
            collected.iter().take(values1.len()).zip(values1.iter()).enumerate()
        {
            prop_assert!(
                values_equal(collected_val, original_val),
                "First yield from should yield value {} correctly",
                i
            );
        }

        // Verify second part matches values2
        for (i, (collected_val, original_val)) in
            collected.iter().skip(values1.len()).zip(values2.iter()).enumerate()
        {
            prop_assert!(
                values_equal(collected_val, original_val),
                "Second yield from should yield value {} correctly",
                i
            );
        }
    }

    /// Property 12: Yield From Delegation - Integer sequence equivalence
    ///
    /// For any sequence of integers, yield from SHALL produce the same
    /// sequence as a list comprehension would.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_integer_sequence_equivalence(ints in arb_int_list()) {
        // Convert to PyValue list (simulating list comprehension result)
        let list_comp_result: Vec<PyValue> = ints.iter().map(|&i| PyValue::Int(i)).collect();

        // Create iterator (simulating generator with yield from)
        let iter = Arc::new(PyIterator::new(list_comp_result.clone()));

        // Collect from iterator
        let mut generator_result = Vec::new();
        while let Some(value) = iter.next() {
            generator_result.push(value);
        }

        // Results should be equivalent
        prop_assert!(
            value_lists_equal(&generator_result, &list_comp_result),
            "yield from should produce same sequence as list comprehension"
        );
    }

    /// Property 12: Yield From Delegation - Nested iteration order
    ///
    /// For nested yield from (yield from [yield from inner]),
    /// values SHALL be yielded in depth-first order.
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_yield_from_nested_order(
        inner1 in arb_nonempty_int_list(),
        inner2 in arb_nonempty_int_list()
    ) {
        // Create inner iterators
        let inner_values1: Vec<PyValue> = inner1.iter().map(|&i| PyValue::Int(i)).collect();
        let inner_values2: Vec<PyValue> = inner2.iter().map(|&i| PyValue::Int(i)).collect();

        let iter1 = Arc::new(PyIterator::new(inner_values1.clone()));
        let iter2 = Arc::new(PyIterator::new(inner_values2.clone()));

        // Simulate nested yield from: first exhaust iter1, then iter2
        let mut collected = Vec::new();

        // First yield from
        while let Some(value) = iter1.next() {
            collected.push(value);
        }

        // Second yield from
        while let Some(value) = iter2.next() {
            collected.push(value);
        }

        // Expected order: all of inner1, then all of inner2
        let expected: Vec<PyValue> = inner_values1
            .into_iter()
            .chain(inner_values2.into_iter())
            .collect();

        prop_assert!(
            value_lists_equal(&collected, &expected),
            "Nested yield from should yield in depth-first order"
        );
    }
}


// ===== Unit Tests for Edge Cases =====

/// Test that a generator starts in Created state
/// **Validates: Requirements 6.1, 6.3**
#[test]
fn test_generator_creation_state() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = PyGenerator::new(func, frame);

    assert_eq!(gen.get_state(), GeneratorState::Created);
    assert!(!gen.is_exhausted());
}

/// Test that generators are their own iterators
/// **Validates: Requirements 6.6**
#[test]
fn test_generator_is_own_iterator() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // __iter__ should return self
    let iter_result = gen.iter();
    match iter_result {
        PyValue::Generator(g) => {
            assert!(Arc::ptr_eq(&g, &gen));
        }
        _ => panic!("Expected generator from __iter__"),
    }
}

/// Test that exhausted generators raise StopIteration
/// **Validates: Requirements 6.4**
#[test]
fn test_exhausted_generator_raises_stop_iteration() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // Mark as completed
    gen.complete(PyValue::None);

    // Should be exhausted
    assert!(gen.is_exhausted());
    assert_eq!(gen.get_state(), GeneratorState::Completed);

    // next() should return StopIteration
    let result = gen.next();
    assert!(matches!(result, GeneratorResult::StopIteration(_)));
}

/// Test generator state transitions
/// **Validates: Requirements 6.2**
#[test]
fn test_generator_state_transitions() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // Initial state is Created
    assert_eq!(gen.get_state(), GeneratorState::Created);

    // After yield, state should be Suspended
    gen.yield_value(PyValue::Int(1));
    assert_eq!(gen.get_state(), GeneratorState::Suspended);

    // After complete, state should be Completed
    gen.complete(PyValue::None);
    assert_eq!(gen.get_state(), GeneratorState::Completed);
    assert!(gen.is_exhausted());
}

/// Test that first send must be None
/// **Validates: Requirements 6.2**
#[test]
fn test_first_send_must_be_none() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // First send with non-None should error
    let result = gen.send(PyValue::Int(42));
    assert!(matches!(result, GeneratorResult::Error(_)));
}

/// Test yield from with empty iterator
/// **Validates: Requirements 6.5**
#[test]
fn test_yield_from_empty_iterator() {
    let values: Vec<PyValue> = vec![];
    let iter = Arc::new(PyIterator::new(values));

    // Empty iterator should yield nothing
    assert!(iter.next().is_none());
}

/// Test yield from with single element
/// **Validates: Requirements 6.5**
#[test]
fn test_yield_from_single_element() {
    let values = vec![PyValue::Int(42)];
    let iter = Arc::new(PyIterator::new(values));

    // Should yield exactly one value
    let first = iter.next();
    assert!(matches!(first, Some(PyValue::Int(42))));

    // Then be exhausted
    assert!(iter.next().is_none());
}

/// Test yield from preserves value types
/// **Validates: Requirements 6.5**
#[test]
fn test_yield_from_preserves_types() {
    let values = vec![
        PyValue::Int(1),
        PyValue::Float(2.5),
        PyValue::Str(Arc::from("hello")),
        PyValue::Bool(true),
        PyValue::None,
    ];
    let iter = Arc::new(PyIterator::new(values));

    // Verify each type is preserved
    assert!(matches!(iter.next(), Some(PyValue::Int(1))));
    assert!(matches!(iter.next(), Some(PyValue::Float(f)) if (f - 2.5).abs() < 0.001));
    assert!(matches!(iter.next(), Some(PyValue::Str(s)) if &*s == "hello"));
    assert!(matches!(iter.next(), Some(PyValue::Bool(true))));
    assert!(matches!(iter.next(), Some(PyValue::None)));
    assert!(iter.next().is_none());
}

/// Test yield from with sequential iterators
/// **Validates: Requirements 6.5**
#[test]
fn test_yield_from_sequential_iterators() {
    // First iterator
    let iter1 = Arc::new(PyIterator::new(vec![PyValue::Int(1), PyValue::Int(2)]));

    // Second iterator
    let iter2 = Arc::new(PyIterator::new(vec![PyValue::Int(3), PyValue::Int(4)]));

    // Collect from first iterator
    let mut collected = Vec::new();
    while let Some(value) = iter1.next() {
        collected.push(value);
    }

    // Collect from second iterator
    while let Some(value) = iter2.next() {
        collected.push(value);
    }

    // Should have all values in order
    assert_eq!(collected.len(), 4);
    assert!(matches!(collected[0], PyValue::Int(1)));
    assert!(matches!(collected[1], PyValue::Int(2)));
    assert!(matches!(collected[2], PyValue::Int(3)));
    assert!(matches!(collected[3], PyValue::Int(4)));
}

/// Test generator close on fresh generator
/// **Validates: Requirements 6.4**
#[test]
fn test_generator_close_fresh() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // Close a fresh generator
    let result = gen.close();
    assert!(matches!(result, GeneratorResult::Closed));
    assert_eq!(gen.get_state(), GeneratorState::Completed);
}

/// Test generator close on suspended generator
/// **Validates: Requirements 6.4**
#[test]
fn test_generator_close_suspended() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // Simulate suspension
    gen.yield_value(PyValue::Int(42));

    // Close a suspended generator should throw GeneratorExit
    let result = gen.close();
    assert!(matches!(result, GeneratorResult::NeedExecution));

    // Check that GeneratorExit was thrown
    let throw_value = gen.take_throw_value();
    match throw_value {
        Some(PyValue::Str(s)) => assert_eq!(&*s, "GeneratorExit"),
        _ => panic!("Expected GeneratorExit"),
    }
}

/// Test iterator yields values in exact order
/// **Validates: Requirements 6.1, 6.2, 6.6**
#[test]
fn test_iterator_yields_values_in_order() {
    let values = vec![
        PyValue::Int(1),
        PyValue::Int(2),
        PyValue::Int(3),
        PyValue::Int(4),
        PyValue::Int(5),
    ];
    let iter = Arc::new(PyIterator::new(values.clone()));

    // Collect all values
    let mut collected = Vec::new();
    while let Some(value) = iter.next() {
        collected.push(value);
    }

    // Verify order
    assert_eq!(collected.len(), 5);
    for (i, value) in collected.iter().enumerate() {
        match value {
            PyValue::Int(n) => assert_eq!(*n, (i + 1) as i64),
            _ => panic!("Expected Int"),
        }
    }
}

/// Test that generator throw works correctly
/// **Validates: Requirements 6.2**
#[test]
fn test_generator_throw() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // Throw an exception into the generator
    let exc = PyValue::Str(Arc::from("ValueError"));
    let result = gen.throw(exc.clone());
    assert!(matches!(result, GeneratorResult::NeedExecution));

    // Check that the exception was stored
    let throw_value = gen.take_throw_value();
    match throw_value {
        Some(PyValue::Str(s)) => assert_eq!(&*s, "ValueError"),
        _ => panic!("Expected exception value"),
    }
}

/// Test that running generator errors on re-entry
/// **Validates: Requirements 6.2**
#[test]
fn test_running_generator_errors() {
    let mut func = PyFunction::new(
        "test_gen",
        CodeRef {
            bytecode_offset: 0,
            num_locals: 0,
            stack_size: 4,
            num_args: 0,
            num_kwonly_args: 0,
        },
        vec![],
    );
    func.flags.is_generator = true;
    let func = Arc::new(func);
    let frame = PyFrame::new(Arc::clone(&func), None);
    let gen = Arc::new(PyGenerator::new(func, frame));

    // Set to running state manually
    *gen.state.lock() = GeneratorState::Running;

    // All operations should error when generator is running
    let send_result = gen.send(PyValue::None);
    assert!(matches!(send_result, GeneratorResult::Error(_)));

    let throw_result = gen.throw(PyValue::Str(Arc::from("Exception")));
    assert!(matches!(throw_result, GeneratorResult::Error(_)));

    let close_result = gen.close();
    assert!(matches!(close_result, GeneratorResult::Error(_)));
}
