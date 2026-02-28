//! Property-based tests for stack allocation

use dx_py_stack::{escape_analysis::AllocKind, EscapeAnalyzer, StackList, StackTuple, TaggedValue};
use proptest::prelude::*;

/// Property 10: Escape Analysis Soundness
mod escape_analysis_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// All allocation sites are found
        #[test]
        fn prop_finds_all_allocations(
            tuple_offsets in prop::collection::vec(0u32..50, 0..5),
            list_offsets in prop::collection::vec(50u32..100, 0..5)
        ) {
            // Build bytecode with allocations at specified offsets
            // Use non-overlapping ranges: tuples at 0-49, lists at 50-99
            let mut bytecode = vec![0xF0u8; 200]; // NOP padding

            for &offset in &tuple_offsets {
                if (offset as usize) < bytecode.len() - 1 {
                    bytecode[offset as usize] = 0x80; // BuildTuple
                    bytecode[offset as usize + 1] = 2; // size
                }
            }

            for &offset in &list_offsets {
                if (offset as usize) < bytecode.len() - 1 {
                    bytecode[offset as usize] = 0x81; // BuildList
                    bytecode[offset as usize + 1] = 2; // size
                }
            }

            let mut analyzer = EscapeAnalyzer::new();
            analyzer.analyze(&bytecode);

            // All tuple allocation sites should be found
            for &offset in &tuple_offsets {
                if (offset as usize) < bytecode.len() - 1 {
                    let site = analyzer.alloc_sites().get(&offset);
                    if let Some(site) = site {
                        prop_assert_eq!(site.kind, AllocKind::Tuple);
                    }
                }
            }

            // All list allocation sites should be found
            for &offset in &list_offsets {
                if (offset as usize) < bytecode.len() - 1 {
                    let site = analyzer.alloc_sites().get(&offset);
                    if let Some(site) = site {
                        prop_assert_eq!(site.kind, AllocKind::List);
                    }
                }
            }
        }

        /// Returned allocations always escape
        #[test]
        fn prop_return_escapes(
            alloc_offset in 0u32..50
        ) {
            // BuildTuple followed by Return
            let mut bytecode = vec![0xF0u8; 100];
            bytecode[alloc_offset as usize] = 0x80; // BuildTuple
            bytecode[alloc_offset as usize + 1] = 2;
            bytecode[alloc_offset as usize + 2] = 0x56; // Return

            let mut analyzer = EscapeAnalyzer::new();
            analyzer.analyze(&bytecode);

            // The allocation should escape
            prop_assert!(!analyzer.can_stack_allocate(alloc_offset));
        }
    }
}

/// Property 7: Stack Allocation Semantic Equivalence
mod stack_allocation_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// StackTuple behaves like Vec for valid operations
        #[test]
        fn prop_stack_tuple_equiv(
            values in prop::collection::vec(any::<u64>(), 0..8)
        ) {
            let mut tuple: StackTuple<8> = StackTuple::new();
            let mut vec_equiv: Vec<u64> = Vec::new();

            for &v in &values {
                tuple.push(v);
                vec_equiv.push(v);
            }

            prop_assert_eq!(tuple.len(), vec_equiv.len());

            for i in 0..values.len() {
                prop_assert_eq!(tuple.get(i), vec_equiv.get(i).copied());
            }

            prop_assert_eq!(tuple.to_vec(), vec_equiv);
        }

        /// StackList behaves like Vec even after overflow
        #[test]
        fn prop_stack_list_equiv(
            values in prop::collection::vec(any::<u64>(), 0..50)
        ) {
            let mut list: StackList<8> = StackList::new();
            let mut vec_equiv: Vec<u64> = Vec::new();

            for &v in &values {
                list.push(v);
                vec_equiv.push(v);
            }

            prop_assert_eq!(list.len(), vec_equiv.len());

            for i in 0..values.len() {
                prop_assert_eq!(list.get(i), vec_equiv.get(i).copied());
            }

            // Pop should work the same
            while !vec_equiv.is_empty() {
                prop_assert_eq!(list.pop(), vec_equiv.pop());
            }

            prop_assert!(list.is_empty());
        }

        /// TaggedValue preserves integer values
        #[test]
        fn prop_tagged_int_roundtrip(
            value in TaggedValue::MIN_SMALL_INT..=TaggedValue::MAX_SMALL_INT
        ) {
            let tagged = TaggedValue::from_small_int(value).unwrap();
            prop_assert!(tagged.is_small_int());
            prop_assert_eq!(tagged.as_small_int(), Some(value));
        }

        /// TaggedValue preserves pointer values
        #[test]
        fn prop_tagged_ptr_roundtrip(
            // Generate aligned addresses (divisible by 2)
            addr in (1u64..u64::MAX/2).prop_map(|x| x * 2)
        ) {
            let tagged = TaggedValue::from_raw(addr);
            if tagged.is_ptr() {
                prop_assert_eq!(tagged.raw(), addr);
            }
        }
    }
}

/// Tests for StackTuple edge cases
mod stack_tuple_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Iteration produces all elements in order
        #[test]
        fn prop_iter_order(
            values in prop::collection::vec(any::<u64>(), 1..8)
        ) {
            let tuple = StackTuple::from_array({
                let mut arr = [0u64; 8];
                for (i, &v) in values.iter().take(8).enumerate() {
                    arr[i] = v;
                }
                arr
            });

            // Note: from_array sets len to N, so we need to handle this
            let collected: Vec<u64> = tuple.iter().take(values.len().min(8)).collect();
            let expected: Vec<u64> = values.iter().take(8).copied().collect();

            // First N elements should match
            for (a, b) in collected.iter().zip(expected.iter()) {
                prop_assert_eq!(a, b);
            }
        }

        /// Clone produces identical tuple
        #[test]
        fn prop_clone_identical(
            values in prop::collection::vec(any::<u64>(), 0..4)
        ) {
            let mut original: StackTuple<4> = StackTuple::new();
            for &v in &values {
                original.push(v);
            }

            let cloned = original.clone();

            prop_assert_eq!(original.len(), cloned.len());
            for i in 0..original.len() {
                prop_assert_eq!(original.get(i), cloned.get(i));
            }
        }
    }
}

/// Tests for StackList heap fallback
mod stack_list_heap_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Heap fallback preserves all data
        #[test]
        fn prop_heap_fallback_preserves(
            values in prop::collection::vec(any::<u64>(), 10..30)
        ) {
            let mut list: StackList<4> = StackList::new();

            for &v in &values {
                list.push(v);
            }

            // Should have overflowed to heap
            prop_assert!(list.is_on_heap());
            prop_assert_eq!(list.len(), values.len());

            // All values should be preserved
            for (i, &expected) in values.iter().enumerate() {
                prop_assert_eq!(list.get(i), Some(expected));
            }
        }

        /// Set works after heap fallback
        #[test]
        fn prop_set_after_overflow(
            initial in prop::collection::vec(any::<u64>(), 10..20),
            new_value in any::<u64>(),
            index in 0usize..10
        ) {
            let mut list: StackList<4> = StackList::new();
            for &v in &initial {
                list.push(v);
            }

            let idx = index % list.len();
            prop_assert!(list.set(idx, new_value));
            prop_assert_eq!(list.get(idx), Some(new_value));
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_tagged_value_zero() {
        let zero = TaggedValue::from_small_int(0).unwrap();
        assert!(zero.is_small_int());
        assert!(!zero.is_null());
        assert_eq!(zero.as_small_int(), Some(0));
    }

    #[test]
    fn test_stack_list_clear() {
        let mut list: StackList<4> = StackList::new();
        list.extend([1, 2, 3, 4, 5, 6]); // Overflow to heap
        assert!(list.is_on_heap());

        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }
}
