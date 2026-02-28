//! Key derivation functions for Node.js crypto compatibility.
//!
//! This module provides PBKDF2 and scrypt key derivation functions
//! compatible with Node.js crypto API.

use hmac::Hmac;
use sha2::{Sha256, Sha512};

/// Error type for key derivation operations.
#[derive(Debug, thiserror::Error)]
pub enum KdfError {
    /// Invalid parameters provided.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    /// Key derivation failed.
    #[error("Key derivation failed: {0}")]
    DerivationFailed(String),
    /// Unsupported algorithm.
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}

/// Digest algorithm for PBKDF2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pbkdf2Digest {
    /// SHA-256
    Sha256,
    /// SHA-512
    Sha512,
}

/// Options for scrypt key derivation.
#[derive(Debug, Clone)]
pub struct ScryptOptions {
    /// CPU/memory cost parameter (N). Must be a power of 2.
    pub cost: u32,
    /// Block size parameter (r).
    pub block_size: u32,
    /// Parallelization parameter (p).
    pub parallelization: u32,
    /// Maximum memory to use in bytes (optional).
    pub max_memory: Option<usize>,
}

impl Default for ScryptOptions {
    fn default() -> Self {
        Self {
            cost: 16384,        // 2^14, reasonable default
            block_size: 8,
            parallelization: 1,
            max_memory: None,
        }
    }
}

impl ScryptOptions {
    /// Create new scrypt options with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cost parameter (N).
    pub fn cost(mut self, cost: u32) -> Self {
        self.cost = cost;
        self
    }

    /// Set the block size parameter (r).
    pub fn block_size(mut self, block_size: u32) -> Self {
        self.block_size = block_size;
        self
    }

    /// Set the parallelization parameter (p).
    pub fn parallelization(mut self, parallelization: u32) -> Self {
        self.parallelization = parallelization;
        self
    }

    /// Set the maximum memory limit.
    pub fn max_memory(mut self, max_memory: usize) -> Self {
        self.max_memory = Some(max_memory);
        self
    }
}

/// Derive a key using PBKDF2 (synchronous).
///
/// # Arguments
/// * `password` - The password to derive from
/// * `salt` - The salt value
/// * `iterations` - Number of iterations
/// * `key_length` - Desired key length in bytes
/// * `digest` - Hash algorithm to use
///
/// # Returns
/// The derived key as a byte vector.
pub fn pbkdf2_sync(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    key_length: usize,
    digest: Pbkdf2Digest,
) -> Result<Vec<u8>, KdfError> {
    if iterations == 0 {
        return Err(KdfError::InvalidParams("iterations must be > 0".into()));
    }
    if key_length == 0 {
        return Err(KdfError::InvalidParams("key_length must be > 0".into()));
    }

    let mut key = vec![0u8; key_length];

    match digest {
        Pbkdf2Digest::Sha256 => {
            pbkdf2::pbkdf2::<Hmac<Sha256>>(password, salt, iterations, &mut key)
                .map_err(|e| KdfError::DerivationFailed(e.to_string()))?;
        }
        Pbkdf2Digest::Sha512 => {
            pbkdf2::pbkdf2::<Hmac<Sha512>>(password, salt, iterations, &mut key)
                .map_err(|e| KdfError::DerivationFailed(e.to_string()))?;
        }
    }

    Ok(key)
}

/// Derive a key using PBKDF2 (asynchronous).
///
/// This runs the derivation in a blocking task to avoid blocking the async runtime.
pub async fn pbkdf2_async(
    password: Vec<u8>,
    salt: Vec<u8>,
    iterations: u32,
    key_length: usize,
    digest: Pbkdf2Digest,
) -> Result<Vec<u8>, KdfError> {
    tokio::task::spawn_blocking(move || {
        pbkdf2_sync(&password, &salt, iterations, key_length, digest)
    })
    .await
    .map_err(|e| KdfError::DerivationFailed(e.to_string()))?
}

/// Derive a key using scrypt (synchronous).
///
/// # Arguments
/// * `password` - The password to derive from
/// * `salt` - The salt value
/// * `key_length` - Desired key length in bytes
/// * `options` - Scrypt parameters
///
/// # Returns
/// The derived key as a byte vector.
pub fn scrypt_sync(
    password: &[u8],
    salt: &[u8],
    key_length: usize,
    options: &ScryptOptions,
) -> Result<Vec<u8>, KdfError> {
    if key_length == 0 {
        return Err(KdfError::InvalidParams("key_length must be > 0".into()));
    }

    // Validate cost is a power of 2
    if options.cost == 0 || (options.cost & (options.cost - 1)) != 0 {
        return Err(KdfError::InvalidParams("cost must be a power of 2".into()));
    }

    // Calculate log2(N) for scrypt params
    let log_n = (options.cost as f64).log2() as u8;

    let params = scrypt::Params::new(log_n, options.block_size, options.parallelization, key_length)
        .map_err(|e| KdfError::InvalidParams(e.to_string()))?;

    let mut key = vec![0u8; key_length];
    scrypt::scrypt(password, salt, &params, &mut key)
        .map_err(|e| KdfError::DerivationFailed(e.to_string()))?;

    Ok(key)
}

/// Derive a key using scrypt (asynchronous).
///
/// This runs the derivation in a blocking task to avoid blocking the async runtime.
pub async fn scrypt_async(
    password: Vec<u8>,
    salt: Vec<u8>,
    key_length: usize,
    options: ScryptOptions,
) -> Result<Vec<u8>, KdfError> {
    tokio::task::spawn_blocking(move || {
        scrypt_sync(&password, &salt, key_length, &options)
    })
    .await
    .map_err(|e| KdfError::DerivationFailed(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2_sha256() {
        let password = b"password";
        let salt = b"salt";
        let iterations = 1000;
        let key_length = 32;

        let key = pbkdf2_sync(password, salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
        assert_eq!(key.len(), key_length);
    }

    #[test]
    fn test_pbkdf2_sha512() {
        let password = b"password";
        let salt = b"salt";
        let iterations = 1000;
        let key_length = 64;

        let key = pbkdf2_sync(password, salt, iterations, key_length, Pbkdf2Digest::Sha512).unwrap();
        assert_eq!(key.len(), key_length);
    }

    #[test]
    fn test_pbkdf2_deterministic() {
        let password = b"password";
        let salt = b"salt";
        let iterations = 1000;
        let key_length = 32;

        let key1 = pbkdf2_sync(password, salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
        let key2 = pbkdf2_sync(password, salt, iterations, key_length, Pbkdf2Digest::Sha256).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_pbkdf2_invalid_iterations() {
        let result = pbkdf2_sync(b"password", b"salt", 0, 32, Pbkdf2Digest::Sha256);
        assert!(result.is_err());
    }

    #[test]
    fn test_scrypt_basic() {
        let password = b"password";
        let salt = b"salt";
        let key_length = 32;
        let options = ScryptOptions::new().cost(1024).block_size(8).parallelization(1);

        let key = scrypt_sync(password, salt, key_length, &options).unwrap();
        assert_eq!(key.len(), key_length);
    }

    #[test]
    fn test_scrypt_deterministic() {
        let password = b"password";
        let salt = b"salt";
        let key_length = 32;
        let options = ScryptOptions::new().cost(1024);

        let key1 = scrypt_sync(password, salt, key_length, &options).unwrap();
        let key2 = scrypt_sync(password, salt, key_length, &options).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_scrypt_invalid_cost() {
        let options = ScryptOptions::new().cost(1000); // Not a power of 2
        let result = scrypt_sync(b"password", b"salt", 32, &options);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pbkdf2_async() {
        let key = pbkdf2_async(
            b"password".to_vec(),
            b"salt".to_vec(),
            1000,
            32,
            Pbkdf2Digest::Sha256,
        )
        .await
        .unwrap();
        assert_eq!(key.len(), 32);
    }

    #[tokio::test]
    async fn test_scrypt_async() {
        let key = scrypt_async(
            b"password".to_vec(),
            b"salt".to_vec(),
            32,
            ScryptOptions::new().cost(1024),
        )
        .await
        .unwrap();
        assert_eq!(key.len(), 32);
    }
}
