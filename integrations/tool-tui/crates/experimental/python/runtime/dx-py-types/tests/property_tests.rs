//! Property-based tests for Type Speculation
//!
//! Property 6: JIT Deoptimization Correctness
//! Property 11: Inline Cache Hit Rate
//! Validates: Requirements 5.4, 5.11

use dx_py_jit::compiler::FunctionId;
use dx_py_jit::profile::PyType;
use dx_py_types::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 11: Inline Cache Hit Rate
    /// Validates: Requirements 5.4
    ///
    /// Monomorphic sites should achieve 99% hit rate.
    #[test]
    fn prop_inline_cache_hit_rate_monomorphic(
        lookups in 100usize..1000
    ) {
        let cache = InlineCache::new();
        let code = 0x1234 as *const u8;

        // Initialize with Int type
        cache.update(PyType::Int, code);

        // All lookups with same type should hit
        for _ in 0..lookups {
            let result = cache.lookup(PyType::Int);
            prop_assert!(result.is_some());
        }

        let hit_rate = cache.hit_rate();
        prop_assert!(
            hit_rate >= 0.99,
            "Monomorphic hit rate {} should be >= 99%",
            hit_rate
        );
    }

    /// Property: PIC correctly handles up to 4 types
    #[test]
    fn prop_pic_handles_four_types(
        type1 in 1u8..15,
        type2 in 1u8..15,
        type3 in 1u8..15,
        type4 in 1u8..15
    ) {
        // Ensure all types are different
        prop_assume!(type1 != type2 && type1 != type3 && type1 != type4);
        prop_assume!(type2 != type3 && type2 != type4);
        prop_assume!(type3 != type4);

        let pic = PolymorphicInlineCache::new();

        let types = [
            PyType::from_u8(type1),
            PyType::from_u8(type2),
            PyType::from_u8(type3),
            PyType::from_u8(type4),
        ];

        // Add all 4 types (use non-null pointers starting from 0x1000)
        for (i, &t) in types.iter().enumerate() {
            let code = ((i + 1) * 0x1000) as *const u8;
            prop_assert!(pic.add_entry(t, code));
        }

        prop_assert!(pic.is_full());

        // All types should be found
        for (i, &t) in types.iter().enumerate() {
            let expected = ((i + 1) * 0x1000) as *const u8;
            prop_assert_eq!(pic.lookup(t), Some(expected));
        }
    }

    /// Property: PIC rejects 5th type
    #[test]
    fn prop_pic_rejects_fifth_type(_seed in any::<u64>()) {
        let pic = PolymorphicInlineCache::new();

        // Add 4 types
        prop_assert!(pic.add_entry(PyType::Int, std::ptr::null()));
        prop_assert!(pic.add_entry(PyType::Float, std::ptr::null()));
        prop_assert!(pic.add_entry(PyType::Str, std::ptr::null()));
        prop_assert!(pic.add_entry(PyType::List, std::ptr::null()));

        // 5th type should be rejected
        prop_assert!(!pic.add_entry(PyType::Dict, std::ptr::null()));
    }

    /// Property 6: Deoptimization preserves correctness
    /// Validates: Requirements 5.11
    #[test]
    fn prop_deopt_preserves_info(
        func_id in 0u64..1000,
        bc_offset in 0usize..10000,
        reason_idx in 0u8..6
    ) {
        use dx_py_types::deopt::{DeoptHandler, DeoptInfo, DeoptReason};
        use dx_py_jit::compiler::ValueLocation;

        let handler = DeoptHandler::new();

        let reason = match reason_idx {
            0 => DeoptReason::TypeGuardFailed,
            1 => DeoptReason::Overflow,
            2 => DeoptReason::DivisionByZero,
            3 => DeoptReason::NullPointer,
            4 => DeoptReason::BoundsCheck,
            _ => DeoptReason::Unknown,
        };

        let info = DeoptInfo {
            func_id: FunctionId(func_id),
            bytecode_offset: bc_offset,
            value_locations: vec![
                ValueLocation::Register(0),
                ValueLocation::Stack(8),
            ],
            reason,
        };

        let code_addr = 0x1000 + func_id as usize;
        handler.register(code_addr, info.clone());

        // Deoptimization should return correct info
        let result = handler.deoptimize(code_addr);
        prop_assert!(result.is_some());

        let deopt_info = result.unwrap();
        prop_assert_eq!(deopt_info.func_id, FunctionId(func_id));
        prop_assert_eq!(deopt_info.bytecode_offset, bc_offset);
        prop_assert_eq!(deopt_info.reason, reason);
        prop_assert_eq!(deopt_info.value_locations.len(), 2);
    }

    /// Property: Type predictor confidence is in [0, 1]
    #[test]
    fn prop_predictor_confidence_range(
        int_count in 0u64..1000,
        float_count in 0u64..1000
    ) {
        let predictor = TypePredictor::with_settings(1, 0.5);
        let func_id = FunctionId(1);

        for _ in 0..int_count {
            predictor.record(func_id, 0, PyType::Int);
        }
        for _ in 0..float_count {
            predictor.record(func_id, 0, PyType::Float);
        }

        let int_conf = predictor.get_confidence(func_id, 0, PyType::Int);
        let float_conf = predictor.get_confidence(func_id, 0, PyType::Float);

        prop_assert!((0.0..=1.0).contains(&int_conf));
        prop_assert!((0.0..=1.0).contains(&float_conf));

        // Confidences should sum to <= 1.0 (may be less due to other types)
        prop_assert!(int_conf + float_conf <= 1.001); // Small epsilon for float errors
    }

    /// Property: Cache state transitions are monotonic
    #[test]
    fn prop_cache_state_monotonic(types in prop::collection::vec(1u8..15, 1..10)) {
        let cache = InlineCache::new();

        let mut prev_state = CacheState::Uninitialized;

        for type_byte in types {
            let py_type = PyType::from_u8(type_byte);
            cache.update(py_type, std::ptr::null());

            let state = cache.state();

            // State should only increase (Uninitialized -> Monomorphic -> Polymorphic)
            prop_assert!(
                state as u8 >= prev_state as u8,
                "State went from {:?} to {:?}",
                prev_state, state
            );

            prev_state = state;
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_inline_cache_reset() {
        let cache = InlineCache::new();

        cache.update(PyType::Int, 0x1000 as *const u8);
        cache.lookup(PyType::Int);
        cache.lookup(PyType::Float);

        assert!(cache.hit_count() > 0 || cache.miss_count() > 0);

        cache.reset();

        assert_eq!(cache.state(), CacheState::Uninitialized);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn test_pic_reset() {
        let pic = PolymorphicInlineCache::new();

        pic.add_entry(PyType::Int, std::ptr::null());
        pic.add_entry(PyType::Float, std::ptr::null());

        assert_eq!(pic.len(), 2);

        pic.reset();

        assert!(pic.is_empty());
    }

    #[test]
    fn test_predictor_clear_function() {
        let predictor = TypePredictor::new();

        predictor.record(FunctionId(1), 0, PyType::Int);
        predictor.record(FunctionId(2), 0, PyType::Float);

        predictor.clear_function(FunctionId(1));

        assert!(predictor.get_stats(FunctionId(1), 0).is_none());
        assert!(predictor.get_stats(FunctionId(2), 0).is_some());
    }
}
