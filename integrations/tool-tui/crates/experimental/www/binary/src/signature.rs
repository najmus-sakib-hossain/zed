//! # Ed25519 Signature Support
//!
//! Every HTIP stream is signed to prevent injection attacks.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Sign payload with Ed25519
pub fn sign_payload(payload: &[u8], signing_key: &SigningKey) -> Signature {
    signing_key.sign(payload)
}

/// Verify payload signature
pub fn verify_payload(payload: &[u8], signature: &Signature, verifying_key: &VerifyingKey) -> bool {
    verifying_key.verify(payload, signature).is_ok()
}

/// Generate a new keypair (for testing/setup)
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_verify() {
        let payload = b"test payload";
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let signature = sign_payload(payload, &signing_key);
        assert!(verify_payload(payload, &signature, &verifying_key));
    }

    #[test]
    fn test_verify_wrong_payload() {
        let payload = b"test payload";
        let wrong_payload = b"wrong payload";

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let signature = sign_payload(payload, &signing_key);
        assert!(!verify_payload(wrong_payload, &signature, &verifying_key));
    }

    #[test]
    fn test_verify_wrong_key() {
        let payload = b"test payload";

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let signature = sign_payload(payload, &signing_key);

        let wrong_key = SigningKey::from_bytes(&[1u8; 32]);
        let wrong_verifying_key = wrong_key.verifying_key();

        assert!(!verify_payload(payload, &signature, &wrong_verifying_key));
    }
}
