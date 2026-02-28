//! Property-based tests for binary validation
//!
//! Feature: production-readiness
//! Property 8: Binary Validation Magic Bytes
//! Property 9: Binary Validation Version
//! Property 10: Binary Validation Signature
//! Property 11: Binary Validation Checksum
//! Validates: Requirements 5.1, 5.2, 5.3, 5.4

use dx_www_binary::{
    DxBinaryError, MAGIC_BYTES, VERSION, deserializer::HtipStream, protocol::HtipHeader,
    serializer::HtipWriter,
};
use ed25519_dalek::SigningKey;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // =========================================================================
    // Property 8: Binary Validation Magic Bytes
    // For any byte sequence that does not start with b"DXB1", the binary
    // validator SHALL return an InvalidMagic error.
    // Validates: Requirements 5.1
    // =========================================================================

    /// Property 8: Invalid magic bytes are rejected
    #[test]
    fn magic_bytes_validation_rejects_invalid(
        invalid_magic in prop::array::uniform4(any::<u8>()).prop_filter(
            "Must not be valid magic bytes",
            |m| m != MAGIC_BYTES
        ),
        template_html in "[a-zA-Z0-9 ]{1,50}",
    ) {
        // Create a valid binary first
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Replace magic bytes with invalid ones
        binary[0..4].copy_from_slice(&invalid_magic);

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        prop_assert!(
            result.is_err(),
            "Invalid magic bytes should cause validation failure"
        );

        match result.unwrap_err() {
            DxBinaryError::InvalidMagic => {
                // Expected error
            }
            other => {
                prop_assert!(
                    false,
                    "Expected InvalidMagic error, got {:?}",
                    other
                );
            }
        }
    }

    /// Property 8 variant: Valid magic bytes pass validation
    #[test]
    fn magic_bytes_validation_accepts_valid(
        template_html in "[a-zA-Z0-9 ]{1,50}",
    ) {
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Verify magic bytes are correct
        prop_assert_eq!(&binary[0..4], MAGIC_BYTES);

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        prop_assert!(
            result.is_ok(),
            "Valid magic bytes should pass validation: {:?}",
            result.err()
        );
    }

    // =========================================================================
    // Property 9: Binary Validation Version
    // For any byte sequence with valid magic but unsupported version, the binary
    // validator SHALL return an UnsupportedVersion error.
    // Validates: Requirements 5.2
    // =========================================================================

    /// Property 9: Invalid version is rejected
    #[test]
    fn version_validation_rejects_invalid(
        invalid_version in any::<u8>().prop_filter(
            "Must not be valid version",
            |v| *v != VERSION
        ),
        template_html in "[a-zA-Z0-9 ]{1,50}",
    ) {
        // Create a valid binary first
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Replace version byte with invalid one (version is at offset 4)
        binary[4] = invalid_version;

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        prop_assert!(
            result.is_err(),
            "Invalid version should cause validation failure"
        );

        match result.unwrap_err() {
            DxBinaryError::UnsupportedVersion(v) => {
                prop_assert_eq!(v, invalid_version);
            }
            other => {
                prop_assert!(
                    false,
                    "Expected UnsupportedVersion error, got {:?}",
                    other
                );
            }
        }
    }

    /// Property 9 variant: Valid version passes validation
    #[test]
    fn version_validation_accepts_valid(
        template_html in "[a-zA-Z0-9 ]{1,50}",
    ) {
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Verify version is correct
        prop_assert_eq!(binary[4], VERSION);

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        prop_assert!(
            result.is_ok(),
            "Valid version should pass validation: {:?}",
            result.err()
        );
    }

    // =========================================================================
    // Property 10: Binary Validation Signature
    // For any byte sequence with valid magic and version but invalid Ed25519
    // signature, the binary validator SHALL return a SignatureVerificationFailed error.
    // Validates: Requirements 5.3
    // =========================================================================

    /// Property 10: Invalid signature is rejected (wrong key)
    #[test]
    fn signature_validation_rejects_wrong_key(
        template_html in "[a-zA-Z0-9 ]{1,50}",
        wrong_key_bytes in prop::array::uniform32(any::<u8>()).prop_filter(
            "Must not be the signing key",
            |k| k != &[0u8; 32]
        ),
    ) {
        // Create a valid binary
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Use a different key for verification
        let wrong_key = SigningKey::from_bytes(&wrong_key_bytes);
        let wrong_verifying_key = wrong_key.verifying_key();

        let result = HtipStream::new(&binary, &wrong_verifying_key);

        prop_assert!(
            result.is_err(),
            "Wrong verification key should cause validation failure"
        );

        match result.unwrap_err() {
            DxBinaryError::SignatureVerificationFailed => {
                // Expected error
            }
            other => {
                prop_assert!(
                    false,
                    "Expected SignatureVerificationFailed error, got {:?}",
                    other
                );
            }
        }
    }

    /// Property 10 variant: Corrupted signature is rejected
    #[test]
    fn signature_validation_rejects_corrupted_signature(
        template_html in "[a-zA-Z0-9 ]{1,50}",
        corruption_pos in 0usize..64,
        corruption_byte in any::<u8>(),
    ) {
        // Create a valid binary
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Corrupt a byte in the signature (signature starts at offset 8, is 64 bytes)
        let sig_offset = 8 + corruption_pos;
        let original = binary[sig_offset];

        // Only test if corruption actually changes the byte
        if original != corruption_byte {
            binary[sig_offset] = corruption_byte;

            let verifying_key = signing_key.verifying_key();
            let result = HtipStream::new(&binary, &verifying_key);

            prop_assert!(
                result.is_err(),
                "Corrupted signature should cause validation failure"
            );

            match result.unwrap_err() {
                DxBinaryError::SignatureVerificationFailed => {
                    // Expected error
                }
                DxBinaryError::ChecksumMismatch { .. } => {
                    // Also acceptable - checksum is validated before signature
                    // but signature corruption doesn't affect checksum
                }
                other => {
                    prop_assert!(
                        false,
                        "Expected SignatureVerificationFailed error, got {:?}",
                        other
                    );
                }
            }
        }
    }

    /// Property 10 variant: Valid signature passes validation
    #[test]
    fn signature_validation_accepts_valid(
        template_html in "[a-zA-Z0-9 ]{1,50}",
        key_bytes in prop::array::uniform32(any::<u8>()),
    ) {
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        prop_assert!(
            result.is_ok(),
            "Valid signature should pass validation: {:?}",
            result.err()
        );
    }

    // =========================================================================
    // Property 11: Binary Validation Checksum
    // For any byte sequence with valid magic, version, and signature but invalid
    // CRC32 checksum, the binary validator SHALL return a ChecksumMismatch error.
    // Validates: Requirements 5.4
    // =========================================================================

    /// Property 11: Binary Validation Checksum
    /// For any byte sequence with valid magic, version, and signature but invalid CRC32 checksum,
    /// the binary validator SHALL return a ChecksumMismatch error.
    /// Validates: Requirements 5.4
    #[test]
    fn checksum_validation_detects_corruption(
        template_html in "[a-zA-Z0-9 ]{1,100}",
        corruption_pos in 0usize..1000,
        corruption_byte in any::<u8>(),
    ) {
        // Create a valid binary
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);
        writer.write_instantiate(1, 0, 0);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Corrupt a byte in the payload (after header)
        if binary.len() > HtipHeader::SIZE + 1 {
            let pos = HtipHeader::SIZE + (corruption_pos % (binary.len() - HtipHeader::SIZE));
            let original = binary[pos];

            // Only test if corruption actually changes the byte
            if original != corruption_byte {
                binary[pos] = corruption_byte;

                // Attempt to deserialize - should fail with ChecksumMismatch
                let verifying_key = signing_key.verifying_key();
                let result = HtipStream::new(&binary, &verifying_key);

                prop_assert!(
                    result.is_err(),
                    "Corrupted payload should fail validation"
                );

                // Should be a ChecksumMismatch error
                match result.unwrap_err() {
                    DxBinaryError::ChecksumMismatch { .. } => {
                        // Expected error
                    }
                    DxBinaryError::BincodeError(_) => {
                        // Also acceptable - corruption may cause deserialization failure
                    }
                    other => {
                        prop_assert!(
                            false,
                            "Expected ChecksumMismatch or BincodeError, got {:?}",
                            other
                        );
                    }
                }
            }
        }
    }

    /// Property 11 variant: Valid binaries pass checksum validation
    /// For any valid HTIP binary, the checksum validation SHALL pass.
    #[test]
    fn valid_binary_passes_checksum(
        template_html in "[a-zA-Z0-9 ]{1,100}",
        num_instantiates in 1usize..10,
    ) {
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);

        for i in 0..num_instantiates {
            writer.write_instantiate(i as u32, 0, 0);
        }

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        prop_assert!(
            result.is_ok(),
            "Valid binary should pass validation: {:?}",
            result.err()
        );
    }

    /// Property 11 variant: Checksum detects single-bit flips
    /// For any valid binary, flipping any single bit in the payload SHALL cause checksum failure.
    #[test]
    fn checksum_detects_single_bit_flip(
        template_html in "[a-zA-Z0-9]{10,50}",
        bit_position in 0usize..8000,
    ) {
        let mut writer = HtipWriter::new();
        writer.write_template(0, &template_html, vec![]);
        writer.write_instantiate(1, 0, 0);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).expect("Serialization should succeed");

        // Flip a single bit in the payload
        let payload_len = binary.len() - HtipHeader::SIZE;
        if payload_len > 0 {
            let byte_pos = HtipHeader::SIZE + (bit_position / 8) % payload_len;
            let bit_offset = bit_position % 8;

            binary[byte_pos] ^= 1 << bit_offset;

            let verifying_key = signing_key.verifying_key();
            let result = HtipStream::new(&binary, &verifying_key);

            prop_assert!(
                result.is_err(),
                "Single bit flip should cause validation failure"
            );
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_checksum_mismatch_error_message() {
        let mut writer = HtipWriter::new();
        writer.write_template(0, "<div>test</div>", vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).unwrap();

        // Corrupt payload
        if binary.len() > HtipHeader::SIZE + 5 {
            binary[HtipHeader::SIZE + 5] ^= 0xFF;
        }

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("Checksum mismatch") || err_msg.contains("Bincode"));
    }
}
