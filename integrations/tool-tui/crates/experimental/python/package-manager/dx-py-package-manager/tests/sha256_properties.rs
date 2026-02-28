//! Property-based tests for SHA256 verification
//!
//! **Feature: dx-py-hardening, Property 7: SHA256 Verification**
//! **Validates: Requirements 1.3, 8.2**

use dx_py_package_manager::{compute_sha256, verify_sha256};
use proptest::prelude::*;

proptest! {
    /// Property 7: SHA256 Verification
    /// *For any* downloaded content, if the computed SHA256 hash does not match
    /// the expected hash, the download SHALL be rejected.
    #[test]
    fn prop_sha256_correct_hash_accepted(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let hash = compute_sha256(&data);
        // Correct hash should always be accepted
        prop_assert!(verify_sha256(&data, &hash).is_ok());
    }

    /// Property 7: SHA256 Verification - Wrong hash rejected
    /// *For any* data and any different hash, verification SHALL fail.
    #[test]
    fn prop_sha256_wrong_hash_rejected(
        data in prop::collection::vec(any::<u8>(), 1..1024),
        wrong_hash in "[0-9a-f]{64}"
    ) {
        let correct_hash = compute_sha256(&data);
        // Only test if the wrong hash is actually different
        if wrong_hash.to_lowercase() != correct_hash {
            prop_assert!(verify_sha256(&data, &wrong_hash).is_err());
        }
    }

    /// Property 7: SHA256 Verification - Case insensitivity
    /// *For any* data, the hash verification SHALL be case-insensitive.
    #[test]
    fn prop_sha256_case_insensitive(data in prop::collection::vec(any::<u8>(), 0..512)) {
        let hash = compute_sha256(&data);
        let upper_hash = hash.to_uppercase();
        let lower_hash = hash.to_lowercase();

        prop_assert!(verify_sha256(&data, &upper_hash).is_ok());
        prop_assert!(verify_sha256(&data, &lower_hash).is_ok());
    }

    /// Property 7: SHA256 Verification - Deterministic
    /// *For any* data, computing the hash multiple times SHALL produce the same result.
    #[test]
    fn prop_sha256_deterministic(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let hash1 = compute_sha256(&data);
        let hash2 = compute_sha256(&data);
        prop_assert_eq!(hash1, hash2);
    }

    /// Property 7: SHA256 Verification - Different data produces different hashes
    /// *For any* two different data inputs, the hashes SHALL be different (with high probability).
    #[test]
    fn prop_sha256_collision_resistant(
        data1 in prop::collection::vec(any::<u8>(), 1..512),
        data2 in prop::collection::vec(any::<u8>(), 1..512)
    ) {
        if data1 != data2 {
            let hash1 = compute_sha256(&data1);
            let hash2 = compute_sha256(&data2);
            prop_assert_ne!(hash1, hash2);
        }
    }
}

#[test]
fn test_sha256_empty_data() {
    let data: &[u8] = b"";
    let hash = compute_sha256(data);
    // SHA256 of empty string is well-known
    assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert!(verify_sha256(data, &hash).is_ok());
}

#[test]
fn test_sha256_known_value() {
    let data = b"hello world";
    let hash = compute_sha256(data);
    assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    assert!(verify_sha256(data, &hash).is_ok());
}
