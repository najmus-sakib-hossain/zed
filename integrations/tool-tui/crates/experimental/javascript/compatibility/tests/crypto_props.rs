//! Property-based tests for crypto implementations.
//!
//! These tests validate the correctness properties defined in the design document:
//! - Property 20: Key derivation correctness
//! - Property 21: Sign/verify round-trip
//! - Property 22: Encrypt/decrypt round-trip
//! - Property 23: Key pair validity

use dx_compat_node::crypto::{
    decrypt, encrypt, generate_ec_p256_key_pair, generate_ec_p384_key_pair,
    generate_rsa_key_pair, pbkdf2_sync, scrypt_sync, sign_ec_p256, sign_ec_p384,
    sign_rsa_key_pair, verify_ec_p256, verify_ec_p384, verify_rsa_key_pair,
    CipherAlgorithm, Pbkdf2Digest, RsaKeyGenOptions, ScryptOptions, SignDigest,
};
use proptest::prelude::*;

/// Generate arbitrary byte data for testing.
fn arb_bytes(max_len: usize) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..max_len)
}

/// Generate a valid scrypt cost (power of 2).
fn arb_scrypt_cost() -> impl Strategy<Value = u32> {
    prop::sample::select(vec![1024u32, 2048, 4096, 8192])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: production-readiness, Property 20: Key derivation correctness (PBKDF2)**
    /// *For any* password and salt, pbkdf2() SHALL produce deterministic, correct key material.
    /// **Validates: Requirements 6.1**
    #[test]
    fn pbkdf2_deterministic(
        password in arb_bytes(64),
        salt in arb_bytes(32),
        iterations in 100u32..1000,
        key_length in 16usize..64
    ) {
        let key1 = pbkdf2_sync(&password, &salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
        let key2 = pbkdf2_sync(&password, &salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
        
        // Same inputs should produce same output
        prop_assert_eq!(key1.len(), key_length);
        prop_assert_eq!(key1, key2);
    }

    /// **Feature: production-readiness, Property 20: Key derivation correctness (scrypt)**
    /// *For any* password and salt, scrypt() SHALL produce deterministic, correct key material.
    /// **Validates: Requirements 6.2**
    #[test]
    fn scrypt_deterministic(
        password in arb_bytes(64),
        salt in arb_bytes(32),
        cost in arb_scrypt_cost(),
        key_length in 16usize..64
    ) {
        let options = ScryptOptions::new().cost(cost);
        
        let key1 = scrypt_sync(&password, &salt, key_length, &options).unwrap();
        let key2 = scrypt_sync(&password, &salt, key_length, &options).unwrap();
        
        // Same inputs should produce same output
        prop_assert_eq!(key1.len(), key_length);
        prop_assert_eq!(key1, key2);
    }

    /// **Feature: production-readiness, Property 21: Sign/verify round-trip (EC P-256)**
    /// *For any* data and key pair, signing then verifying SHALL return true.
    /// **Validates: Requirements 6.4, 6.5**
    #[test]
    fn ec_p256_sign_verify_round_trip(data in arb_bytes(1024)) {
        let key_pair = generate_ec_p256_key_pair().unwrap();
        
        let signature = sign_ec_p256(&key_pair, &data).unwrap();
        let valid = verify_ec_p256(&key_pair, &data, &signature).unwrap();
        
        prop_assert!(valid, "Signature verification should succeed for original data");
    }

    /// **Feature: production-readiness, Property 21: Sign/verify round-trip (EC P-384)**
    /// *For any* data and key pair, signing then verifying SHALL return true.
    /// **Validates: Requirements 6.4, 6.5**
    #[test]
    fn ec_p384_sign_verify_round_trip(data in arb_bytes(1024)) {
        let key_pair = generate_ec_p384_key_pair().unwrap();
        
        let signature = sign_ec_p384(&key_pair, &data).unwrap();
        let valid = verify_ec_p384(&key_pair, &data, &signature).unwrap();
        
        prop_assert!(valid, "Signature verification should succeed for original data");
    }

    /// **Feature: production-readiness, Property 21: Sign/verify detects tampering**
    /// *For any* data, verifying with wrong data SHALL return false.
    /// **Validates: Requirements 6.4, 6.5**
    #[test]
    fn ec_p256_sign_verify_detects_tampering(
        data in arb_bytes(100).prop_filter("non-empty", |d| !d.is_empty()),
        tamper_index in any::<usize>()
    ) {
        let key_pair = generate_ec_p256_key_pair().unwrap();
        
        let signature = sign_ec_p256(&key_pair, &data).unwrap();
        
        // Tamper with the data
        let mut tampered = data.clone();
        let idx = tamper_index % tampered.len();
        tampered[idx] ^= 0xFF;
        
        let valid = verify_ec_p256(&key_pair, &tampered, &signature).unwrap();
        
        prop_assert!(!valid, "Signature verification should fail for tampered data");
    }

    /// **Feature: production-readiness, Property 22: Encrypt/decrypt round-trip (AES-256-CBC)**
    /// *For any* plaintext and key, encrypting then decrypting SHALL produce the original.
    /// **Validates: Requirements 6.6, 6.7**
    #[test]
    fn aes256_cbc_round_trip(plaintext in arb_bytes(1024)) {
        let key = [0x42u8; 32]; // Fixed key for testing
        let iv = [0x24u8; 16];  // Fixed IV for testing
        
        let ciphertext = encrypt(CipherAlgorithm::Aes256Cbc, &key, &iv, &plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::Aes256Cbc, &key, &iv, &ciphertext).unwrap();
        
        prop_assert_eq!(plaintext, decrypted);
    }

    /// **Feature: production-readiness, Property 22: Encrypt/decrypt round-trip (AES-256-CTR)**
    /// *For any* plaintext and key, encrypting then decrypting SHALL produce the original.
    /// **Validates: Requirements 6.6, 6.7**
    #[test]
    fn aes256_ctr_round_trip(plaintext in arb_bytes(1024)) {
        let key = [0x42u8; 32];
        let iv = [0x24u8; 16];
        
        let ciphertext = encrypt(CipherAlgorithm::Aes256Ctr, &key, &iv, &plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::Aes256Ctr, &key, &iv, &ciphertext).unwrap();
        
        prop_assert_eq!(plaintext, decrypted);
    }

    /// **Feature: production-readiness, Property 22: Encrypt/decrypt round-trip (ChaCha20-Poly1305)**
    /// *For any* plaintext and key, encrypting then decrypting SHALL produce the original.
    /// **Validates: Requirements 6.6, 6.7**
    #[test]
    fn chacha20_poly1305_round_trip(plaintext in arb_bytes(1024)) {
        let key = [0x42u8; 32];
        let nonce = [0x24u8; 12];
        
        let ciphertext = encrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, &plaintext).unwrap();
        let decrypted = decrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, &ciphertext).unwrap();
        
        prop_assert_eq!(plaintext, decrypted);
    }

    /// **Feature: production-readiness, Property 23: Key pair validity (EC P-256)**
    /// *For any* generated key pair, the keys SHALL be valid for signing.
    /// **Validates: Requirements 6.3**
    #[test]
    fn ec_p256_key_pair_valid(_seed in any::<u64>()) {
        let key_pair = generate_ec_p256_key_pair().unwrap();
        
        // Private key should be 32 bytes
        prop_assert_eq!(key_pair.private_key_bytes().len(), 32);
        
        // Public key (uncompressed) should be 65 bytes
        prop_assert_eq!(key_pair.public_key_bytes().len(), 65);
        
        // Should be able to sign and verify
        let data = b"test data";
        let signature = sign_ec_p256(&key_pair, data).unwrap();
        let valid = verify_ec_p256(&key_pair, data, &signature).unwrap();
        prop_assert!(valid);
    }

    /// **Feature: production-readiness, Property 23: Key pair validity (EC P-384)**
    /// *For any* generated key pair, the keys SHALL be valid for signing.
    /// **Validates: Requirements 6.3**
    #[test]
    fn ec_p384_key_pair_valid(_seed in any::<u64>()) {
        let key_pair = generate_ec_p384_key_pair().unwrap();
        
        // Private key should be 48 bytes
        prop_assert_eq!(key_pair.private_key_bytes().len(), 48);
        
        // Public key (uncompressed) should be 97 bytes
        prop_assert_eq!(key_pair.public_key_bytes().len(), 97);
        
        // Should be able to sign and verify
        let data = b"test data";
        let signature = sign_ec_p384(&key_pair, data).unwrap();
        let valid = verify_ec_p384(&key_pair, data, &signature).unwrap();
        prop_assert!(valid);
    }
}

/// **Feature: production-readiness, Property 21: Sign/verify round-trip (RSA)**
/// RSA signing and verification should work correctly.
/// **Validates: Requirements 6.4, 6.5**
#[test]
fn rsa_sign_verify_round_trip() {
    let options = RsaKeyGenOptions::new().modulus_length(2048);
    let key_pair = generate_rsa_key_pair(&options).unwrap();
    let data = b"Hello, World!";

    let signature = sign_rsa_key_pair(&key_pair, data, SignDigest::Sha256).unwrap();
    let valid = verify_rsa_key_pair(&key_pair, data, &signature, SignDigest::Sha256).unwrap();

    assert!(valid);
}

/// **Feature: production-readiness, Property 21: RSA sign/verify detects tampering**
/// RSA verification should fail for tampered data.
/// **Validates: Requirements 6.4, 6.5**
#[test]
fn rsa_sign_verify_detects_tampering() {
    let options = RsaKeyGenOptions::new().modulus_length(2048);
    let key_pair = generate_rsa_key_pair(&options).unwrap();
    let data = b"Hello, World!";

    let signature = sign_rsa_key_pair(&key_pair, data, SignDigest::Sha256).unwrap();
    let valid = verify_rsa_key_pair(&key_pair, b"Tampered!", &signature, SignDigest::Sha256).unwrap();

    assert!(!valid);
}

/// **Feature: production-readiness, Property 23: RSA key pair validity**
/// Generated RSA key pairs should be valid.
/// **Validates: Requirements 6.3**
#[test]
fn rsa_key_pair_valid() {
    let options = RsaKeyGenOptions::new().modulus_length(2048);
    let key_pair = generate_rsa_key_pair(&options).unwrap();

    // Should be able to export keys
    assert!(!key_pair.private_key_der().is_empty());
    assert!(!key_pair.public_key_der().is_empty());
    assert!(key_pair.private_key_pem().contains("BEGIN PRIVATE KEY"));
    assert!(key_pair.public_key_pem().contains("BEGIN PUBLIC KEY"));
}

/// **Feature: production-readiness, Property 22: AEAD tamper detection**
/// ChaCha20-Poly1305 should detect tampering.
/// **Validates: Requirements 6.6, 6.7**
#[test]
fn chacha20_poly1305_detects_tampering() {
    let key = [0x42u8; 32];
    let nonce = [0x24u8; 12];
    let plaintext = b"Hello, World!";

    let mut ciphertext = encrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, plaintext).unwrap();
    
    // Tamper with the ciphertext
    if !ciphertext.is_empty() {
        ciphertext[0] ^= 0xFF;
    }

    // Decryption should fail
    let result = decrypt(CipherAlgorithm::ChaCha20Poly1305, &key, &nonce, &ciphertext);
    assert!(result.is_err());
}

/// **Feature: production-readiness, Property 20: Different passwords produce different keys**
/// PBKDF2 with different passwords should produce different keys.
/// **Validates: Requirements 6.1**
#[test]
fn pbkdf2_different_passwords_different_keys() {
    let salt = b"salt";
    let iterations = 1000;
    let key_length = 32;

    let key1 = pbkdf2_sync(b"password1", salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
    let key2 = pbkdf2_sync(b"password2", salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();

    assert_ne!(key1, key2);
}

/// **Feature: production-readiness, Property 20: Different salts produce different keys**
/// PBKDF2 with different salts should produce different keys.
/// **Validates: Requirements 6.1**
#[test]
fn pbkdf2_different_salts_different_keys() {
    let password = b"password";
    let iterations = 1000;
    let key_length = 32;

    let key1 = pbkdf2_sync(password, b"salt1", iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
    let key2 = pbkdf2_sync(password, b"salt2", iterations, key_length, Pbkdf2Digest::Sha256).unwrap();

    assert_ne!(key1, key2);
}
