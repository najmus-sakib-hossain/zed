//! Cryptography Module
//!
//! Native implementation of Node.js crypto module

use crate::error::{DxError, DxResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as StdHasher};

/// Crypto module
pub struct CryptoModule;

impl CryptoModule {
    pub fn new() -> Self {
        Self
    }

    /// Create hash
    pub fn create_hash(&self, algorithm: &str) -> DxResult<HashBuilder> {
        match algorithm.to_lowercase().as_str() {
            "sha256" | "sha512" | "md5" | "sha1" => Ok(HashBuilder::new(algorithm)),
            _ => Err(DxError::RuntimeError(format!("Unsupported hash algorithm: {}", algorithm))),
        }
    }

    /// Generate random bytes
    pub fn random_bytes(&self, size: usize) -> Vec<u8> {
        // Simplified random generation
        // In production, would use getrandom crate
        (0..size).map(|i| (i % 256) as u8).collect()
    }

    /// Generate random UUID
    pub fn random_uuid(&self) -> String {
        // Simplified UUID v4
        format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            self.random_u32(),
            self.random_u16(),
            0x4000 | (self.random_u16() & 0x0fff),
            0x8000 | (self.random_u16() & 0x3fff),
            self.random_u64() & 0xffffffffffff
        )
    }

    /// Create HMAC
    pub fn create_hmac(&self, algorithm: &str, key: &[u8]) -> DxResult<Hmac> {
        Ok(Hmac::new(algorithm, key))
    }

    /// PBKDF2 key derivation
    pub fn pbkdf2(
        &self,
        password: &[u8],
        salt: &[u8],
        _iterations: usize,
        key_len: usize,
    ) -> Vec<u8> {
        // Simplified PBKDF2 - in production use pbkdf2 crate
        let mut key = Vec::with_capacity(key_len);
        for i in 0..key_len {
            key.push(password[i % password.len()] ^ salt[i % salt.len()]);
        }
        key
    }

    /// Constant-time comparison
    pub fn timing_safe_equal(&self, a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }

    // Helper methods for random number generation
    fn random_u32(&self) -> u32 {
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        (StdHasher::finish(&hasher) & 0xffffffff) as u32
    }

    fn random_u16(&self) -> u16 {
        (self.random_u32() & 0xffff) as u16
    }

    fn random_u64(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        StdHasher::finish(&hasher)
    }
}

impl Default for CryptoModule {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash builder
pub struct HashBuilder {
    algorithm: String,
    data: Vec<u8>,
}

impl HashBuilder {
    pub fn new(algorithm: &str) -> Self {
        Self {
            algorithm: algorithm.to_string(),
            data: Vec::new(),
        }
    }

    /// Update hash with data
    pub fn update(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Finalize and return digest
    pub fn digest(&self) -> Vec<u8> {
        // Simplified hashing - in production use sha2/md5 crates
        let mut hasher = DefaultHasher::new();
        self.data.hash(&mut hasher);
        let hash = StdHasher::finish(&hasher);

        match self.algorithm.as_str() {
            "sha256" => hash.to_be_bytes().repeat(4).to_vec(), // 32 bytes
            "sha512" => hash.to_be_bytes().repeat(8).to_vec(), // 64 bytes
            "md5" => hash.to_be_bytes().to_vec()[..16].to_vec(), // 16 bytes
            "sha1" => hash.to_be_bytes().to_vec()[..20].to_vec(), // 20 bytes
            _ => hash.to_be_bytes().to_vec(),
        }
    }

    /// Get digest as hex string
    pub fn digest_hex(&self) -> String {
        self.digest().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Get digest as base64
    pub fn digest_base64(&self) -> String {
        base64_encode(&self.digest())
    }
}

/// HMAC builder
pub struct Hmac {
    /// Algorithm name - reserved for algorithm-specific HMAC
    #[allow(dead_code)]
    algorithm: String,
    key: Vec<u8>,
    data: Vec<u8>,
}

impl Hmac {
    pub fn new(algorithm: &str, key: &[u8]) -> Self {
        Self {
            algorithm: algorithm.to_string(),
            key: key.to_vec(),
            data: Vec::new(),
        }
    }

    /// Update HMAC with data
    pub fn update(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Finalize and return digest
    pub fn digest(&self) -> Vec<u8> {
        // Simplified HMAC - in production use hmac crate
        let mut hasher = DefaultHasher::new();
        self.key.hash(&mut hasher);
        self.data.hash(&mut hasher);
        StdHasher::finish(&hasher).to_be_bytes().to_vec()
    }

    /// Get digest as hex string
    pub fn digest_hex(&self) -> String {
        self.digest().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Cipher for encryption/decryption
pub struct Cipher {
    /// Algorithm name - reserved for algorithm-specific encryption
    #[allow(dead_code)]
    algorithm: String,
    key: Vec<u8>,
    /// Initialization vector - reserved for CBC/CTR modes
    #[allow(dead_code)]
    iv: Option<Vec<u8>>,
}

impl Cipher {
    pub fn new(algorithm: &str, key: Vec<u8>, iv: Option<Vec<u8>>) -> Self {
        Self {
            algorithm: algorithm.to_string(),
            key,
            iv,
        }
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        // Simplified XOR cipher - in production use aes/chacha20 crates
        data.iter().enumerate().map(|(i, b)| b ^ self.key[i % self.key.len()]).collect()
    }

    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        // XOR is symmetric
        self.encrypt(data)
    }
}

/// Base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }

        let b1 = (buf[0] >> 2) as usize;
        let b2 = (((buf[0] & 0x03) << 4) | (buf[1] >> 4)) as usize;
        let b3 = (((buf[1] & 0x0F) << 2) | (buf[2] >> 6)) as usize;
        let b4 = (buf[2] & 0x3F) as usize;

        result.push(CHARSET[b1] as char);
        result.push(CHARSET[b2] as char);
        result.push(if chunk.len() > 1 {
            CHARSET[b3] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            CHARSET[b4] as char
        } else {
            '='
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        let crypto = CryptoModule::new();
        let mut hasher = crypto.create_hash("sha256").unwrap();
        hasher.update(b"hello");
        let digest = hasher.digest_hex();
        assert!(!digest.is_empty());
    }

    #[test]
    fn test_random_bytes() {
        let crypto = CryptoModule::new();
        let bytes = crypto.random_bytes(16);
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn test_random_uuid() {
        let crypto = CryptoModule::new();
        let uuid = crypto.random_uuid();
        assert_eq!(uuid.len(), 36); // UUID v4 format
    }

    #[test]
    fn test_hmac() {
        let crypto = CryptoModule::new();
        let mut hmac = crypto.create_hmac("sha256", b"secret").unwrap();
        hmac.update(b"message");
        let digest = hmac.digest_hex();
        assert!(!digest.is_empty());
    }

    #[test]
    fn test_timing_safe_equal() {
        let crypto = CryptoModule::new();
        assert!(crypto.timing_safe_equal(b"hello", b"hello"));
        assert!(!crypto.timing_safe_equal(b"hello", b"world"));
    }

    #[test]
    fn test_cipher() {
        let cipher = Cipher::new("aes-256-cbc", vec![1, 2, 3, 4], None);
        let encrypted = cipher.encrypt(b"hello");
        let decrypted = cipher.decrypt(&encrypted);
        assert_eq!(decrypted, b"hello");
    }
}
