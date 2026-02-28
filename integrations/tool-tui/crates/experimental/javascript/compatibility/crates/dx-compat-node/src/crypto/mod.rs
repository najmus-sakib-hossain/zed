//! Cryptographic operations.
//!
//! This module provides Node.js-compatible crypto APIs including:
//! - Hashing (MD5, SHA-1, SHA-256, SHA-512)
//! - HMAC
//! - Key derivation (PBKDF2, scrypt)
//! - Key generation (RSA, EC P-256, EC P-384)
//! - Digital signatures (RSA, ECDSA)
//! - Encryption/decryption (AES-CBC, AES-CTR, ChaCha20-Poly1305)

pub mod cipher;
pub mod kdf;
pub mod keygen;
pub mod sign;

pub use cipher::{decrypt, encrypt, CipherAlgorithm, CipherError};
pub use kdf::{
    pbkdf2_async, pbkdf2_sync, scrypt_async, scrypt_sync, KdfError, Pbkdf2Digest, ScryptOptions,
};
pub use keygen::{
    generate_ec_p256_key_pair, generate_ec_p384_key_pair, generate_key_pair_async,
    generate_key_pair_sync, generate_rsa_key_pair, EcCurve, EcKeyGenOptions, EcP256KeyPair,
    EcP384KeyPair, KeyAlgorithm, KeyGenError, KeyPair, RsaKeyGenOptions, RsaKeyPair,
};
pub use sign::{
    sign_ec_p256, sign_ec_p384, sign_rsa, sign_rsa_key_pair, verify_ec_p256, verify_ec_p384,
    verify_rsa, verify_rsa_key_pair, SignDigest, SignError,
};

use hmac::{Hmac, Mac};
use md5::Md5; // md-5 crate re-exports as md5
use sha2::{Digest, Sha256, Sha512};

/// Hash algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// MD5
    Md5,
    /// SHA-1
    Sha1,
    /// SHA-256
    Sha256,
    /// SHA-512
    Sha512,
}

/// Streaming hash interface (like Node.js crypto.createHash)
pub struct Hash {
    #[allow(dead_code)]
    algorithm: HashAlgorithm,
    state: HashState,
}

enum HashState {
    Md5(Md5),
    Sha1(sha1::Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
}

impl Hash {
    /// Create a new hash instance
    pub fn new(algorithm: HashAlgorithm) -> Self {
        let state = match algorithm {
            HashAlgorithm::Md5 => HashState::Md5(Md5::new()),
            HashAlgorithm::Sha1 => HashState::Sha1(sha1::Sha1::new()),
            HashAlgorithm::Sha256 => HashState::Sha256(Sha256::new()),
            HashAlgorithm::Sha512 => HashState::Sha512(Sha512::new()),
        };
        Self { algorithm, state }
    }

    /// Update the hash with data
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        match &mut self.state {
            HashState::Md5(h) => h.update(data),
            HashState::Sha1(h) => h.update(data),
            HashState::Sha256(h) => h.update(data),
            HashState::Sha512(h) => h.update(data),
        }
        self
    }

    /// Finalize and return the digest
    pub fn digest(self) -> Vec<u8> {
        match self.state {
            HashState::Md5(h) => h.finalize().to_vec(),
            HashState::Sha1(h) => h.finalize().to_vec(),
            HashState::Sha256(h) => h.finalize().to_vec(),
            HashState::Sha512(h) => h.finalize().to_vec(),
        }
    }

    /// Finalize and return the digest as hex string
    pub fn digest_hex(self) -> String {
        self.digest().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Finalize and return the digest as base64 string
    pub fn digest_base64(self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.digest())
    }
}

/// HMAC interface (like Node.js crypto.createHmac)
pub struct HmacHash {
    #[allow(dead_code)]
    algorithm: HashAlgorithm,
    state: HmacState,
}

enum HmacState {
    Sha256(Hmac<Sha256>),
    Sha512(Hmac<Sha512>),
}

impl HmacHash {
    /// Create a new HMAC instance
    pub fn new(algorithm: HashAlgorithm, key: &[u8]) -> Result<Self, &'static str> {
        let state = match algorithm {
            HashAlgorithm::Sha256 => HmacState::Sha256(
                Hmac::<Sha256>::new_from_slice(key).map_err(|_| "Invalid key length")?,
            ),
            HashAlgorithm::Sha512 => HmacState::Sha512(
                Hmac::<Sha512>::new_from_slice(key).map_err(|_| "Invalid key length")?,
            ),
            _ => return Err("Unsupported algorithm for HMAC"),
        };
        Ok(Self { algorithm, state })
    }

    /// Update the HMAC with data
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        match &mut self.state {
            HmacState::Sha256(h) => h.update(data),
            HmacState::Sha512(h) => h.update(data),
        }
        self
    }

    /// Finalize and return the digest
    pub fn digest(self) -> Vec<u8> {
        match self.state {
            HmacState::Sha256(h) => h.finalize().into_bytes().to_vec(),
            HmacState::Sha512(h) => h.finalize().into_bytes().to_vec(),
        }
    }

    /// Finalize and return the digest as hex string
    pub fn digest_hex(self) -> String {
        self.digest().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Create a hash of data.
pub fn create_hash(algorithm: HashAlgorithm, data: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Md5 => {
            let mut hasher = Md5::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Sha1 => {
            use sha1::Sha1;
            let mut hasher = Sha1::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
    }
}

/// Create HMAC of data
pub fn create_hmac(
    algorithm: HashAlgorithm,
    key: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let mut hmac = HmacHash::new(algorithm, key)?;
    hmac.update(data);
    Ok(hmac.digest())
}

/// Generate cryptographically secure random bytes.
pub fn random_bytes(size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    getrandom::getrandom(&mut buf).expect("Failed to generate random bytes");
    buf
}

/// Generate random UUID v4.
pub fn random_uuid() -> String {
    let bytes = random_bytes(16);
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        (bytes[6] & 0x0f) | 0x40, bytes[7],
        (bytes[8] & 0x3f) | 0x80, bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// Timing-safe comparison of two buffers
pub fn timing_safe_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// PBKDF2 key derivation
pub fn pbkdf2(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    key_length: usize,
    algorithm: HashAlgorithm,
) -> Result<Vec<u8>, &'static str> {
    let mut key = vec![0u8; key_length];

    match algorithm {
        HashAlgorithm::Sha256 => {
            pbkdf2::pbkdf2::<Hmac<Sha256>>(password, salt, iterations, &mut key)
                .map_err(|_| "PBKDF2 failed")?;
        }
        HashAlgorithm::Sha512 => {
            pbkdf2::pbkdf2::<Hmac<Sha512>>(password, salt, iterations, &mut key)
                .map_err(|_| "PBKDF2 failed")?;
        }
        _ => return Err("Unsupported algorithm for PBKDF2"),
    }

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = create_hash(HashAlgorithm::Sha256, b"hello");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_streaming_hash() {
        let mut hash = Hash::new(HashAlgorithm::Sha256);
        hash.update(b"hello").update(b" world");
        let digest = hash.digest_hex();

        let direct = create_hash(HashAlgorithm::Sha256, b"hello world");
        let direct_hex: String = direct.iter().map(|b| format!("{:02x}", b)).collect();

        assert_eq!(digest, direct_hex);
    }

    #[test]
    fn test_hmac() {
        let hmac = create_hmac(HashAlgorithm::Sha256, b"secret", b"message").unwrap();
        assert_eq!(hmac.len(), 32);
    }

    #[test]
    fn test_random_bytes() {
        let bytes = random_bytes(32);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_random_uuid() {
        let uuid = random_uuid();
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_timing_safe_equal() {
        assert!(timing_safe_equal(b"hello", b"hello"));
        assert!(!timing_safe_equal(b"hello", b"world"));
        assert!(!timing_safe_equal(b"hello", b"hell"));
    }
}
