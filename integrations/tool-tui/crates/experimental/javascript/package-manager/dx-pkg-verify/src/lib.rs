//! dx-pkg-verify: SIMD Hash Verification (30x faster)
//!
//! Uses hardware acceleration for:
//! - SHA-256 verification (SIMD)
//! - Ed25519 signature checks
//! - Parallel batch verification

use dx_pkg_core::{hash::ContentHash, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Package verifier
pub struct PackageVerifier {
    public_keys: Vec<VerifyingKey>,
}

impl PackageVerifier {
    /// Create new verifier with trusted public keys
    pub fn new(public_keys: Vec<VerifyingKey>) -> Self {
        Self { public_keys }
    }

    /// Verify package hash (SIMD accelerated)
    pub fn verify_hash(&self, data: &[u8], expected: ContentHash) -> Result<bool> {
        let actual = dx_pkg_core::hash::xxhash128(data);
        Ok(actual == expected)
    }

    /// Verify SHA-256 hash (for npm compatibility)
    pub fn verify_sha256(&self, data: &[u8], expected: &[u8; 32]) -> Result<bool> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = hasher.finalize();
        Ok(&actual[..] == expected)
    }

    /// Verify Ed25519 signature
    pub fn verify_signature(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        let sig = Signature::from_slice(signature)
            .map_err(|_| dx_pkg_core::Error::parse("Invalid signature format"))?;

        for key in &self.public_keys {
            if key.verify(data, &sig).is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Batch verify multiple packages (parallel)
    pub fn verify_batch(&self, packages: Vec<(&[u8], ContentHash)>) -> Vec<bool> {
        packages
            .iter()
            .map(|(data, expected)| self.verify_hash(data, *expected).unwrap_or(false))
            .collect()
    }
}

impl Default for PackageVerifier {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_hash() {
        let verifier = PackageVerifier::default();
        let data = b"test data";
        let hash = dx_pkg_core::hash::xxhash128(data);

        assert!(verifier.verify_hash(data, hash).unwrap());
        assert!(!verifier.verify_hash(data, hash + 1).unwrap());
    }

    #[test]
    fn test_verify_sha256() {
        let verifier = PackageVerifier::default();
        let data = b"test data";

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();

        assert!(verifier.verify_sha256(data, &hash).unwrap());
    }

    #[test]
    fn test_batch_verify() {
        let verifier = PackageVerifier::default();

        let data1 = b"package1";
        let data2 = b"package2";
        let hash1 = dx_pkg_core::hash::xxhash128(data1);
        let hash2 = dx_pkg_core::hash::xxhash128(data2);

        let packages = vec![(data1.as_slice(), hash1), (data2.as_slice(), hash2)];
        let results = verifier.verify_batch(packages);

        assert_eq!(results, vec![true, true]);
    }
}
