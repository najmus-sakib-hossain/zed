//! Property-based tests for state synchronization.
//!
//! Feature: dcp-protocol, Property 8: XOR Delta Round-Trip

use dcp::sync::XorDelta;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 8: XOR Delta Round-Trip
    /// For any two states (previous and new), computing an XOR delta and
    /// applying it to the previous state SHALL produce the new state.
    /// **Validates: Requirements 6.1, 6.2, 6.3, 6.4**
    #[test]
    fn prop_xor_delta_round_trip(
        prev in prop::collection::vec(any::<u8>(), 0..500),
        new in prop::collection::vec(any::<u8>(), 0..500),
    ) {
        let delta = XorDelta::compute(&prev, &new);
        let result = delta.apply(&prev);

        prop_assert!(result.is_ok(), "Delta application failed: {:?}", result);
        prop_assert_eq!(result.unwrap(), new, "Round-trip failed to produce original new state");
    }

    /// Feature: dcp-protocol, Property 8: XOR Delta Round-Trip
    /// For sparse changes, the delta size SHALL be smaller than the full state.
    /// **Validates: Requirements 6.2**
    #[test]
    fn prop_xor_delta_sparse_efficiency(
        base_size in 100usize..1000,
        change_positions in prop::collection::vec(0usize..1000, 1..10),
        change_values in prop::collection::vec(any::<u8>(), 1..10),
    ) {
        // Create a base state of zeros
        let prev = vec![0u8; base_size];
        let mut new = prev.clone();

        // Apply sparse changes
        for (pos, val) in change_positions.iter().zip(change_values.iter()) {
            let idx = pos % base_size;
            new[idx] = *val;
        }

        let delta = XorDelta::compute(&prev, &new);

        // For sparse changes (few non-zero XOR values), RLE should compress well
        // The delta should be smaller than the full state for sparse changes
        // Note: This may not always hold for very small states or many changes
        if change_positions.len() < base_size / 10 {
            prop_assert!(
                delta.is_sparse(new.len()),
                "Delta size {} should be smaller than state size {} for sparse changes",
                delta.patch_size(),
                new.len()
            );
        }

        // Verify round-trip still works
        let result = delta.apply(&prev).unwrap();
        prop_assert_eq!(result, new);
    }

    /// Feature: dcp-protocol, Property 8: XOR Delta Round-Trip
    /// Hash verification SHALL detect state mismatches.
    /// **Validates: Requirements 6.4**
    #[test]
    fn prop_xor_delta_hash_verification(
        prev in prop::collection::vec(any::<u8>(), 1..100),
        new in prop::collection::vec(any::<u8>(), 1..100),
        wrong in prop::collection::vec(any::<u8>(), 1..100),
    ) {
        prop_assume!(prev != wrong);

        let delta = XorDelta::compute(&prev, &new);

        // Should verify correctly against prev
        prop_assert!(delta.verify_prev_hash(&prev));

        // Should fail verification against wrong state
        prop_assert!(!delta.verify_prev_hash(&wrong));

        // Apply should fail with wrong state
        let result = delta.apply(&wrong);
        prop_assert!(result.is_err());
    }

    /// Feature: dcp-protocol, Property 8: XOR Delta Round-Trip
    /// Delta computation SHALL handle different length states.
    /// **Validates: Requirements 6.1**
    #[test]
    fn prop_xor_delta_different_lengths(
        prev_len in 0usize..200,
        new_len in 0usize..200,
    ) {
        let prev: Vec<u8> = (0..prev_len).map(|i| i as u8).collect();
        let new: Vec<u8> = (0..new_len).map(|i| (i * 2) as u8).collect();

        let delta = XorDelta::compute(&prev, &new);
        let result = delta.apply(&prev);

        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), new);
    }

    /// Feature: dcp-protocol, Property 8: XOR Delta Round-Trip
    /// Identical states SHALL produce minimal delta.
    /// **Validates: Requirements 6.2**
    #[test]
    fn prop_xor_delta_identical_states(
        state in prop::collection::vec(any::<u8>(), 0..500),
    ) {
        let delta = XorDelta::compute(&state, &state);

        // Delta should be very small for identical states (all zeros compress well)
        // RLE of all zeros: [count, 0] pairs
        let expected_max_size = (state.len() / 255 + 1) * 2;
        prop_assert!(
            delta.patch_size() <= expected_max_size,
            "Delta for identical states should be minimal, got {} bytes",
            delta.patch_size()
        );

        // Round-trip should work
        let result = delta.apply(&state).unwrap();
        prop_assert_eq!(result, state);
    }

    /// Feature: dcp-protocol, Property 8: XOR Delta Round-Trip
    /// Delta hashes SHALL be consistent.
    /// **Validates: Requirements 6.3, 6.4**
    #[test]
    fn prop_xor_delta_hash_consistency(
        prev in prop::collection::vec(any::<u8>(), 1..100),
        new in prop::collection::vec(any::<u8>(), 1..100),
    ) {
        let delta1 = XorDelta::compute(&prev, &new);
        let delta2 = XorDelta::compute(&prev, &new);

        // Same inputs should produce same hashes
        prop_assert_eq!(delta1.prev_hash, delta2.prev_hash);
        prop_assert_eq!(delta1.new_hash, delta2.new_hash);

        // Hashes should be different for different states (with high probability)
        if prev != new {
            prop_assert_ne!(delta1.prev_hash, delta1.new_hash);
        }
    }
}
