//! # Cryptographic Module
//!
//! Ed25519 signature verification for cache integrity

use ed25519_dalek::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, VerifyingKey};

/// Verify signature of cached data
pub fn verify_signature(data: &[u8], signature_bytes: &[u8], public_key_bytes: &[u8]) -> bool {
    if signature_bytes.len() != SIGNATURE_LENGTH {
        return false;
    }

    if public_key_bytes.len() != PUBLIC_KEY_LENGTH {
        return false;
    }

    // Parse public key
    let public_key = match VerifyingKey::from_bytes(public_key_bytes.try_into().unwrap()) {
        Ok(key) => key,
        Err(_) => return false,
    };

    // Parse signature
    let mut sig_array = [0u8; SIGNATURE_LENGTH];
    sig_array.copy_from_slice(signature_bytes);
    let signature = Signature::from_bytes(&sig_array);

    // Verify
    public_key.verify_strict(data, &signature).is_ok()
}

/// Generate cache key from origin and public key
pub fn generate_cache_key(origin: &str, public_key: &[u8]) -> String {
    let hash = blake3::hash(public_key);
    format!("{}:{}", origin, hash.to_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    #[test]
    fn test_signature_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let data = b"test data";
        let signature = signing_key.sign(data);

        assert!(verify_signature(data, signature.to_bytes().as_ref(), verifying_key.as_bytes(),));
    }

    #[test]
    fn test_invalid_signature() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let data = b"test data";
        let signature = signing_key.sign(data);

        let wrong_data = b"wrong data";

        assert!(!verify_signature(
            wrong_data,
            signature.to_bytes().as_ref(),
            verifying_key.as_bytes(),
        ));
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = generate_cache_key("https://example.com", &[1, 2, 3]);
        let key2 = generate_cache_key("https://example.com", &[1, 2, 3]);
        let key3 = generate_cache_key("https://example.com", &[4, 5, 6]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
