//! Property-based tests for security layer.
//!
//! Feature: dcp-protocol, Property 9: Ed25519 Signature Verification
//! Feature: dcp-protocol, Property 10: Replay and Expiration Protection

use dcp::security::{NonceStore, Signer, Verifier};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 9: Ed25519 Signature Verification
    /// For any SignedToolDef, a valid signature SHALL verify successfully.
    /// **Validates: Requirements 7.2**
    #[test]
    fn prop_tool_def_signature_valid(
        seed in any::<[u8; 32]>(),
        tool_id in any::<u32>(),
        schema_hash in any::<[u8; 32]>(),
        capabilities in any::<u64>(),
    ) {
        let signer = Signer::from_seed(&seed);
        let def = signer.sign_tool_def(tool_id, schema_hash, capabilities);

        // Valid signature should verify
        let result = Verifier::verify_tool_def(&def);
        prop_assert!(result.is_ok(), "Valid signature should verify: {:?}", result);
    }

    /// Feature: dcp-protocol, Property 9: Ed25519 Signature Verification
    /// For any SignedToolDef, any modification to the signed data SHALL cause
    /// verification to fail.
    /// **Validates: Requirements 7.2**
    #[test]
    fn prop_tool_def_tamper_detection(
        seed in any::<[u8; 32]>(),
        tool_id in any::<u32>(),
        schema_hash in any::<[u8; 32]>(),
        capabilities in any::<u64>(),
        tamper_field in 0u8..3,
        tamper_value in any::<u8>(),
    ) {
        let signer = Signer::from_seed(&seed);
        let mut def = signer.sign_tool_def(tool_id, schema_hash, capabilities);

        // Tamper with a field
        match tamper_field {
            0 => {
                // Tamper with tool_id
                let new_id = def.tool_id.wrapping_add(1);
                if new_id != def.tool_id {
                    def.tool_id = new_id;
                } else {
                    return Ok(()); // Skip if no change
                }
            }
            1 => {
                // Tamper with schema_hash
                def.schema_hash[0] = def.schema_hash[0].wrapping_add(1);
            }
            _ => {
                // Tamper with capabilities
                def.capabilities = def.capabilities.wrapping_add(1);
            }
        }

        // Tampered signature should fail verification
        let result = Verifier::verify_tool_def(&def);
        prop_assert!(result.is_err(), "Tampered signature should fail verification");
    }

    /// Feature: dcp-protocol, Property 9: Ed25519 Signature Verification
    /// For any SignedInvocation, a valid signature SHALL verify successfully.
    /// **Validates: Requirements 7.2**
    #[test]
    fn prop_invocation_signature_valid(
        seed in any::<[u8; 32]>(),
        tool_id in any::<u32>(),
        nonce in any::<u64>(),
        timestamp in any::<u64>(),
        args in prop::collection::vec(any::<u8>(), 0..100),
    ) {
        let signer = Signer::from_seed(&seed);
        let inv = signer.sign_invocation(tool_id, nonce, timestamp, &args);
        let public_key = signer.public_key_bytes();

        // Valid signature should verify
        let result = Verifier::verify_invocation(&inv, &public_key);
        prop_assert!(result.is_ok(), "Valid signature should verify: {:?}", result);

        // Args hash should match
        prop_assert!(Verifier::verify_args_hash(&inv, &args));
    }

    /// Feature: dcp-protocol, Property 9: Ed25519 Signature Verification
    /// For any SignedInvocation, any modification SHALL cause verification to fail.
    /// **Validates: Requirements 7.2**
    #[test]
    fn prop_invocation_tamper_detection(
        seed in any::<[u8; 32]>(),
        tool_id in any::<u32>(),
        nonce in any::<u64>(),
        timestamp in any::<u64>(),
        args in prop::collection::vec(any::<u8>(), 0..100),
        tamper_field in 0u8..4,
    ) {
        let signer = Signer::from_seed(&seed);
        let mut inv = signer.sign_invocation(tool_id, nonce, timestamp, &args);
        let public_key = signer.public_key_bytes();

        // Tamper with a field
        match tamper_field {
            0 => inv.tool_id = inv.tool_id.wrapping_add(1),
            1 => inv.nonce = inv.nonce.wrapping_add(1),
            2 => inv.timestamp = inv.timestamp.wrapping_add(1),
            _ => inv.args_hash[0] = inv.args_hash[0].wrapping_add(1),
        }

        // Tampered signature should fail verification
        let result = Verifier::verify_invocation(&inv, &public_key);
        prop_assert!(result.is_err(), "Tampered signature should fail verification");
    }

    /// Feature: dcp-protocol, Property 9: Ed25519 Signature Verification
    /// Wrong public key SHALL cause verification to fail.
    /// **Validates: Requirements 7.2**
    #[test]
    fn prop_wrong_public_key_fails(
        seed1 in any::<[u8; 32]>(),
        seed2 in any::<[u8; 32]>(),
        tool_id in any::<u32>(),
        nonce in any::<u64>(),
        timestamp in any::<u64>(),
        args in prop::collection::vec(any::<u8>(), 0..50),
    ) {
        prop_assume!(seed1 != seed2);

        let signer1 = Signer::from_seed(&seed1);
        let signer2 = Signer::from_seed(&seed2);

        let inv = signer1.sign_invocation(tool_id, nonce, timestamp, &args);
        let wrong_key = signer2.public_key_bytes();

        // Verification with wrong key should fail
        let result = Verifier::verify_invocation(&inv, &wrong_key);
        prop_assert!(result.is_err(), "Wrong public key should fail verification");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 10: Replay and Expiration Protection
    /// For any SignedInvocation, reusing the same nonce SHALL be rejected.
    /// **Validates: Requirements 7.4**
    #[test]
    fn prop_nonce_reuse_rejected(
        nonce in any::<u64>(),
        timestamp_offset in 0u64..60, // Within valid window
    ) {
        let mut store = NonceStore::with_config(1000, 300);
        let now = NonceStore::current_timestamp();
        let timestamp = now.saturating_sub(timestamp_offset);

        // First use should succeed
        let result1 = store.check_nonce(nonce, timestamp);
        prop_assert!(result1.is_ok(), "First nonce use should succeed");

        // Second use should fail as replay
        let result2 = store.check_nonce(nonce, timestamp);
        prop_assert!(result2.is_err(), "Nonce reuse should be rejected");
        prop_assert!(matches!(result2, Err(dcp::SecurityError::ReplayAttack)));
    }

    /// Feature: dcp-protocol, Property 10: Replay and Expiration Protection
    /// Timestamps older than the expiration window SHALL be rejected.
    /// **Validates: Requirements 7.5**
    #[test]
    fn prop_expired_timestamp_rejected(
        nonce in any::<u64>(),
        expiration_secs in 60u64..300,
        extra_age in 1u64..100,
    ) {
        let mut store = NonceStore::with_config(1000, expiration_secs);
        let now = NonceStore::current_timestamp();

        // Timestamp older than expiration window
        let old_timestamp = now.saturating_sub(expiration_secs + extra_age);

        let result = store.check_nonce(nonce, old_timestamp);
        prop_assert!(result.is_err(), "Expired timestamp should be rejected");
        prop_assert!(matches!(result, Err(dcp::SecurityError::ExpiredTimestamp)));
    }

    /// Feature: dcp-protocol, Property 10: Replay and Expiration Protection
    /// Valid timestamps within the window SHALL be accepted.
    /// **Validates: Requirements 7.5**
    #[test]
    fn prop_valid_timestamp_accepted(
        nonce in any::<u64>(),
        expiration_secs in 60u64..300,
        age in 0u64..60, // Within valid window
    ) {
        let mut store = NonceStore::with_config(1000, expiration_secs);
        let now = NonceStore::current_timestamp();
        let timestamp = now.saturating_sub(age.min(expiration_secs - 1));

        let result = store.check_nonce(nonce, timestamp);
        prop_assert!(result.is_ok(), "Valid timestamp should be accepted: {:?}", result);
    }

    /// Feature: dcp-protocol, Property 10: Replay and Expiration Protection
    /// Different nonces SHALL be accepted independently.
    /// **Validates: Requirements 7.4**
    #[test]
    fn prop_different_nonces_independent(
        nonces in prop::collection::vec(any::<u64>(), 1..50),
    ) {
        let mut store = NonceStore::with_config(1000, 300);
        let now = NonceStore::current_timestamp();

        // Deduplicate nonces for this test
        let unique_nonces: std::collections::HashSet<_> = nonces.into_iter().collect();

        for nonce in unique_nonces {
            let result = store.check_nonce(nonce, now);
            prop_assert!(result.is_ok(), "Unique nonce {} should be accepted", nonce);
        }
    }

    /// Feature: dcp-protocol, Property 10: Replay and Expiration Protection
    /// Cleanup SHALL remove only expired nonces.
    /// **Validates: Requirements 7.4, 7.5**
    #[test]
    fn prop_cleanup_preserves_valid_nonces(
        valid_nonces in prop::collection::vec(any::<u64>(), 1..20),
    ) {
        let expiration_secs = 300u64;
        let mut store = NonceStore::with_config(1000, expiration_secs);
        let now = NonceStore::current_timestamp();

        // Add valid nonces
        let unique_valid: std::collections::HashSet<_> = valid_nonces.into_iter().collect();
        for &nonce in &unique_valid {
            store.check_nonce(nonce, now).ok();
        }

        let count_before = store.len();

        // Cleanup should not remove valid nonces
        store.cleanup_expired();

        prop_assert_eq!(store.len(), count_before, "Valid nonces should be preserved after cleanup");

        // Valid nonces should still be rejected as replays
        for &nonce in &unique_valid {
            let result = store.check_nonce(nonce, now);
            prop_assert!(result.is_err(), "Valid nonce should still be tracked after cleanup");
        }
    }
}
