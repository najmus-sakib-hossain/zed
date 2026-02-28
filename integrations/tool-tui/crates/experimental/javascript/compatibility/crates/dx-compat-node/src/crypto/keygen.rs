//! Key generation for Node.js crypto compatibility.
//!
//! This module provides RSA and EC key pair generation compatible
//! with Node.js crypto.generateKeyPair API.

use rand::rngs::OsRng;
use rsa::{RsaPrivateKey, RsaPublicKey};

/// Error type for key generation operations.
#[derive(Debug, thiserror::Error)]
pub enum KeyGenError {
    /// Invalid parameters provided.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    /// Key generation failed.
    #[error("Key generation failed: {0}")]
    GenerationFailed(String),
    /// Unsupported algorithm.
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}

/// Key algorithm type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    /// RSA key pair
    Rsa,
    /// Elliptic Curve P-256 (secp256r1)
    EcP256,
    /// Elliptic Curve P-384 (secp384r1)
    EcP384,
}

/// RSA key generation options.
#[derive(Debug, Clone)]
pub struct RsaKeyGenOptions {
    /// Key size in bits (2048, 3072, 4096)
    pub modulus_length: usize,
    /// Public exponent (default: 65537)
    pub public_exponent: u64,
}

impl Default for RsaKeyGenOptions {
    fn default() -> Self {
        Self {
            modulus_length: 2048,
            public_exponent: 65537,
        }
    }
}

impl RsaKeyGenOptions {
    /// Create new RSA options with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the modulus length (key size in bits).
    pub fn modulus_length(mut self, bits: usize) -> Self {
        self.modulus_length = bits;
        self
    }

    /// Set the public exponent.
    pub fn public_exponent(mut self, exp: u64) -> Self {
        self.public_exponent = exp;
        self
    }
}

/// EC key generation options.
#[derive(Debug, Clone)]
pub struct EcKeyGenOptions {
    /// Named curve (P-256 or P-384)
    pub named_curve: EcCurve,
}

/// Elliptic curve type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcCurve {
    /// P-256 (secp256r1, prime256v1)
    P256,
    /// P-384 (secp384r1)
    P384,
}

impl Default for EcKeyGenOptions {
    fn default() -> Self {
        Self {
            named_curve: EcCurve::P256,
        }
    }
}

impl EcKeyGenOptions {
    /// Create new EC options with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the named curve.
    pub fn named_curve(mut self, curve: EcCurve) -> Self {
        self.named_curve = curve;
        self
    }
}

/// An RSA key pair.
#[derive(Debug, Clone)]
pub struct RsaKeyPair {
    /// The private key
    pub private_key: RsaPrivateKey,
    /// The public key
    pub public_key: RsaPublicKey,
}

impl RsaKeyPair {
    /// Get the private key in PKCS#8 DER format.
    pub fn private_key_der(&self) -> Vec<u8> {
        use rsa::pkcs8::EncodePrivateKey;
        self.private_key
            .to_pkcs8_der()
            .map(|doc| doc.as_bytes().to_vec())
            .unwrap_or_default()
    }

    /// Get the public key in SPKI DER format.
    pub fn public_key_der(&self) -> Vec<u8> {
        use rsa::pkcs8::EncodePublicKey;
        self.public_key
            .to_public_key_der()
            .map(|doc| doc.as_bytes().to_vec())
            .unwrap_or_default()
    }

    /// Get the private key in PEM format.
    pub fn private_key_pem(&self) -> String {
        use rsa::pkcs8::EncodePrivateKey;
        self.private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// Get the public key in PEM format.
    pub fn public_key_pem(&self) -> String {
        use rsa::pkcs8::EncodePublicKey;
        self.public_key
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap_or_default()
    }
}

/// An EC key pair for P-256.
#[derive(Debug, Clone)]
pub struct EcP256KeyPair {
    /// The signing key (private)
    pub signing_key: p256::ecdsa::SigningKey,
    /// The verifying key (public)
    pub verifying_key: p256::ecdsa::VerifyingKey,
}

impl EcP256KeyPair {
    /// Get the private key bytes.
    pub fn private_key_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    /// Get the public key bytes (uncompressed).
    pub fn public_key_bytes(&self) -> Vec<u8> {
        use p256::elliptic_curve::sec1::ToEncodedPoint;
        self.verifying_key.to_encoded_point(false).as_bytes().to_vec()
    }
}

/// An EC key pair for P-384.
#[derive(Debug, Clone)]
pub struct EcP384KeyPair {
    /// The signing key (private)
    pub signing_key: p384::ecdsa::SigningKey,
    /// The verifying key (public)
    pub verifying_key: p384::ecdsa::VerifyingKey,
}

impl EcP384KeyPair {
    /// Get the private key bytes.
    pub fn private_key_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    /// Get the public key bytes (uncompressed).
    pub fn public_key_bytes(&self) -> Vec<u8> {
        use p384::elliptic_curve::sec1::ToEncodedPoint;
        self.verifying_key.to_encoded_point(false).as_bytes().to_vec()
    }
}

/// A generic key pair that can hold any supported key type.
#[derive(Debug, Clone)]
pub enum KeyPair {
    /// RSA key pair
    Rsa(RsaKeyPair),
    /// EC P-256 key pair
    EcP256(EcP256KeyPair),
    /// EC P-384 key pair
    EcP384(EcP384KeyPair),
}

impl KeyPair {
    /// Get the algorithm type.
    pub fn algorithm(&self) -> KeyAlgorithm {
        match self {
            KeyPair::Rsa(_) => KeyAlgorithm::Rsa,
            KeyPair::EcP256(_) => KeyAlgorithm::EcP256,
            KeyPair::EcP384(_) => KeyAlgorithm::EcP384,
        }
    }
}

/// Generate an RSA key pair (synchronous).
pub fn generate_rsa_key_pair(options: &RsaKeyGenOptions) -> Result<RsaKeyPair, KeyGenError> {
    // Validate modulus length
    if options.modulus_length < 1024 {
        return Err(KeyGenError::InvalidParams(
            "modulus_length must be at least 1024 bits".into(),
        ));
    }

    let private_key = RsaPrivateKey::new(&mut OsRng, options.modulus_length)
        .map_err(|e| KeyGenError::GenerationFailed(e.to_string()))?;

    let public_key = RsaPublicKey::from(&private_key);

    Ok(RsaKeyPair {
        private_key,
        public_key,
    })
}

/// Generate an EC P-256 key pair (synchronous).
pub fn generate_ec_p256_key_pair() -> Result<EcP256KeyPair, KeyGenError> {
    let signing_key = p256::ecdsa::SigningKey::random(&mut OsRng);
    let verifying_key = p256::ecdsa::VerifyingKey::from(&signing_key);

    Ok(EcP256KeyPair {
        signing_key,
        verifying_key,
    })
}

/// Generate an EC P-384 key pair (synchronous).
pub fn generate_ec_p384_key_pair() -> Result<EcP384KeyPair, KeyGenError> {
    let signing_key = p384::ecdsa::SigningKey::random(&mut OsRng);
    let verifying_key = p384::ecdsa::VerifyingKey::from(&signing_key);

    Ok(EcP384KeyPair {
        signing_key,
        verifying_key,
    })
}

/// Generate a key pair (synchronous).
pub fn generate_key_pair_sync(algorithm: KeyAlgorithm) -> Result<KeyPair, KeyGenError> {
    match algorithm {
        KeyAlgorithm::Rsa => {
            let pair = generate_rsa_key_pair(&RsaKeyGenOptions::default())?;
            Ok(KeyPair::Rsa(pair))
        }
        KeyAlgorithm::EcP256 => {
            let pair = generate_ec_p256_key_pair()?;
            Ok(KeyPair::EcP256(pair))
        }
        KeyAlgorithm::EcP384 => {
            let pair = generate_ec_p384_key_pair()?;
            Ok(KeyPair::EcP384(pair))
        }
    }
}

/// Generate a key pair (asynchronous).
pub async fn generate_key_pair_async(algorithm: KeyAlgorithm) -> Result<KeyPair, KeyGenError> {
    tokio::task::spawn_blocking(move || generate_key_pair_sync(algorithm))
        .await
        .map_err(|e| KeyGenError::GenerationFailed(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rsa_2048() {
        let options = RsaKeyGenOptions::new().modulus_length(2048);
        let pair = generate_rsa_key_pair(&options).unwrap();
        
        // Verify we can export keys
        assert!(!pair.private_key_der().is_empty());
        assert!(!pair.public_key_der().is_empty());
        assert!(pair.private_key_pem().contains("BEGIN PRIVATE KEY"));
        assert!(pair.public_key_pem().contains("BEGIN PUBLIC KEY"));
    }

    #[test]
    fn test_generate_rsa_invalid_size() {
        let options = RsaKeyGenOptions::new().modulus_length(512);
        let result = generate_rsa_key_pair(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_ec_p256() {
        let pair = generate_ec_p256_key_pair().unwrap();
        
        // P-256 private key is 32 bytes
        assert_eq!(pair.private_key_bytes().len(), 32);
        // P-256 uncompressed public key is 65 bytes (1 + 32 + 32)
        assert_eq!(pair.public_key_bytes().len(), 65);
    }

    #[test]
    fn test_generate_ec_p384() {
        let pair = generate_ec_p384_key_pair().unwrap();
        
        // P-384 private key is 48 bytes
        assert_eq!(pair.private_key_bytes().len(), 48);
        // P-384 uncompressed public key is 97 bytes (1 + 48 + 48)
        assert_eq!(pair.public_key_bytes().len(), 97);
    }

    #[test]
    fn test_generate_key_pair_sync() {
        let rsa = generate_key_pair_sync(KeyAlgorithm::Rsa).unwrap();
        assert_eq!(rsa.algorithm(), KeyAlgorithm::Rsa);

        let ec256 = generate_key_pair_sync(KeyAlgorithm::EcP256).unwrap();
        assert_eq!(ec256.algorithm(), KeyAlgorithm::EcP256);

        let ec384 = generate_key_pair_sync(KeyAlgorithm::EcP384).unwrap();
        assert_eq!(ec384.algorithm(), KeyAlgorithm::EcP384);
    }

    #[tokio::test]
    async fn test_generate_key_pair_async() {
        let pair = generate_key_pair_async(KeyAlgorithm::EcP256).await.unwrap();
        assert_eq!(pair.algorithm(), KeyAlgorithm::EcP256);
    }
}
