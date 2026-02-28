//! Digital signature operations for Node.js crypto compatibility.
//!
//! This module provides signing and verification functions compatible
//! with Node.js crypto.sign and crypto.verify APIs.

use rsa::pkcs1v15::{SigningKey as RsaSigningKey, VerifyingKey as RsaVerifyingKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Sha256, Sha384, Sha512};

use super::keygen::{EcP256KeyPair, EcP384KeyPair, RsaKeyPair};

/// Error type for signing operations.
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    /// Invalid key provided.
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    /// Signing failed.
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    /// Verification failed.
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    /// Unsupported algorithm.
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}

/// Digest algorithm for signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignDigest {
    /// SHA-256
    Sha256,
    /// SHA-384
    Sha384,
    /// SHA-512
    Sha512,
}

/// Sign data using RSA with PKCS#1 v1.5 padding.
pub fn sign_rsa(
    private_key: &RsaPrivateKey,
    data: &[u8],
    digest: SignDigest,
) -> Result<Vec<u8>, SignError> {
    match digest {
        SignDigest::Sha256 => {
            let signing_key = RsaSigningKey::<Sha256>::new_unprefixed(private_key.clone());
            let signature = signing_key.sign(data);
            Ok(signature.to_vec())
        }
        SignDigest::Sha384 => {
            let signing_key = RsaSigningKey::<Sha384>::new_unprefixed(private_key.clone());
            let signature = signing_key.sign(data);
            Ok(signature.to_vec())
        }
        SignDigest::Sha512 => {
            let signing_key = RsaSigningKey::<Sha512>::new_unprefixed(private_key.clone());
            let signature = signing_key.sign(data);
            Ok(signature.to_vec())
        }
    }
}

/// Verify an RSA signature with PKCS#1 v1.5 padding.
pub fn verify_rsa(
    public_key: &RsaPublicKey,
    data: &[u8],
    signature: &[u8],
    digest: SignDigest,
) -> Result<bool, SignError> {
    match digest {
        SignDigest::Sha256 => {
            let verifying_key = RsaVerifyingKey::<Sha256>::new_unprefixed(public_key.clone());
            let sig = rsa::pkcs1v15::Signature::try_from(signature)
                .map_err(|e| SignError::InvalidKey(e.to_string()))?;
            Ok(verifying_key.verify(data, &sig).is_ok())
        }
        SignDigest::Sha384 => {
            let verifying_key = RsaVerifyingKey::<Sha384>::new_unprefixed(public_key.clone());
            let sig = rsa::pkcs1v15::Signature::try_from(signature)
                .map_err(|e| SignError::InvalidKey(e.to_string()))?;
            Ok(verifying_key.verify(data, &sig).is_ok())
        }
        SignDigest::Sha512 => {
            let verifying_key = RsaVerifyingKey::<Sha512>::new_unprefixed(public_key.clone());
            let sig = rsa::pkcs1v15::Signature::try_from(signature)
                .map_err(|e| SignError::InvalidKey(e.to_string()))?;
            Ok(verifying_key.verify(data, &sig).is_ok())
        }
    }
}

/// Sign data using RSA key pair.
pub fn sign_rsa_key_pair(
    key_pair: &RsaKeyPair,
    data: &[u8],
    digest: SignDigest,
) -> Result<Vec<u8>, SignError> {
    sign_rsa(&key_pair.private_key, data, digest)
}

/// Verify signature using RSA key pair.
pub fn verify_rsa_key_pair(
    key_pair: &RsaKeyPair,
    data: &[u8],
    signature: &[u8],
    digest: SignDigest,
) -> Result<bool, SignError> {
    verify_rsa(&key_pair.public_key, data, signature, digest)
}

/// Sign data using EC P-256 (ECDSA).
pub fn sign_ec_p256(key_pair: &EcP256KeyPair, data: &[u8]) -> Result<Vec<u8>, SignError> {
    use ecdsa::signature::Signer;
    let signature: p256::ecdsa::Signature = key_pair.signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

/// Verify an EC P-256 signature (ECDSA).
pub fn verify_ec_p256(
    key_pair: &EcP256KeyPair,
    data: &[u8],
    signature: &[u8],
) -> Result<bool, SignError> {
    use ecdsa::signature::Verifier;
    let sig = p256::ecdsa::Signature::from_slice(signature)
        .map_err(|e| SignError::InvalidKey(e.to_string()))?;
    Ok(key_pair.verifying_key.verify(data, &sig).is_ok())
}

/// Sign data using EC P-384 (ECDSA).
pub fn sign_ec_p384(key_pair: &EcP384KeyPair, data: &[u8]) -> Result<Vec<u8>, SignError> {
    use ecdsa::signature::Signer;
    let signature: p384::ecdsa::Signature = key_pair.signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

/// Verify an EC P-384 signature (ECDSA).
pub fn verify_ec_p384(
    key_pair: &EcP384KeyPair,
    data: &[u8],
    signature: &[u8],
) -> Result<bool, SignError> {
    use ecdsa::signature::Verifier;
    let sig = p384::ecdsa::Signature::from_slice(signature)
        .map_err(|e| SignError::InvalidKey(e.to_string()))?;
    Ok(key_pair.verifying_key.verify(data, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keygen::{
        generate_ec_p256_key_pair, generate_ec_p384_key_pair, generate_rsa_key_pair,
        RsaKeyGenOptions,
    };

    #[test]
    fn test_rsa_sign_verify_sha256() {
        let options = RsaKeyGenOptions::new().modulus_length(2048);
        let key_pair = generate_rsa_key_pair(&options).unwrap();
        let data = b"Hello, World!";

        let signature = sign_rsa_key_pair(&key_pair, data, SignDigest::Sha256).unwrap();
        assert!(!signature.is_empty());

        let valid = verify_rsa_key_pair(&key_pair, data, &signature, SignDigest::Sha256).unwrap();
        assert!(valid);

        // Verify with wrong data should fail
        let invalid = verify_rsa_key_pair(&key_pair, b"Wrong data", &signature, SignDigest::Sha256).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_rsa_sign_verify_sha512() {
        let options = RsaKeyGenOptions::new().modulus_length(2048);
        let key_pair = generate_rsa_key_pair(&options).unwrap();
        let data = b"Hello, World!";

        let signature = sign_rsa_key_pair(&key_pair, data, SignDigest::Sha512).unwrap();
        let valid = verify_rsa_key_pair(&key_pair, data, &signature, SignDigest::Sha512).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_ec_p256_sign_verify() {
        let key_pair = generate_ec_p256_key_pair().unwrap();
        let data = b"Hello, World!";

        let signature = sign_ec_p256(&key_pair, data).unwrap();
        assert!(!signature.is_empty());

        let valid = verify_ec_p256(&key_pair, data, &signature).unwrap();
        assert!(valid);

        // Verify with wrong data should fail
        let invalid = verify_ec_p256(&key_pair, b"Wrong data", &signature).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_ec_p384_sign_verify() {
        let key_pair = generate_ec_p384_key_pair().unwrap();
        let data = b"Hello, World!";

        let signature = sign_ec_p384(&key_pair, data).unwrap();
        assert!(!signature.is_empty());

        let valid = verify_ec_p384(&key_pair, data, &signature).unwrap();
        assert!(valid);

        // Verify with wrong data should fail
        let invalid = verify_ec_p384(&key_pair, b"Wrong data", &signature).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_signature_determinism() {
        // EC signatures are NOT deterministic by default (they use random k)
        // RSA PKCS#1 v1.5 signatures ARE deterministic
        let options = RsaKeyGenOptions::new().modulus_length(2048);
        let key_pair = generate_rsa_key_pair(&options).unwrap();
        let data = b"Hello, World!";

        let sig1 = sign_rsa_key_pair(&key_pair, data, SignDigest::Sha256).unwrap();
        let sig2 = sign_rsa_key_pair(&key_pair, data, SignDigest::Sha256).unwrap();
        assert_eq!(sig1, sig2); // RSA PKCS#1 v1.5 is deterministic
    }
}
