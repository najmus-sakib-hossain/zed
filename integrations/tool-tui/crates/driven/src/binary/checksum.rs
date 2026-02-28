//! Blake3 Integrity Verification
//!
//! Cryptographic checksums for rule integrity.

/// Blake3 checksum (first 16 bytes for compact storage)
pub type Blake3Checksum = [u8; 16];

/// Compute Blake3 hash of data
pub fn compute_blake3(data: &[u8]) -> Blake3Checksum {
    let hash = blake3::hash(data);
    let mut checksum = [0u8; 16];
    checksum.copy_from_slice(&hash.as_bytes()[..16]);
    checksum
}

/// Verify Blake3 checksum
pub fn verify_blake3(data: &[u8], expected: &Blake3Checksum) -> bool {
    let computed = compute_blake3(data);
    computed == *expected
}

/// Full Blake3 hash (32 bytes)
pub fn full_blake3(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Keyed Blake3 MAC
pub fn keyed_blake3(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    *blake3::keyed_hash(key, data).as_bytes()
}

/// Incremental hasher for streaming
#[derive(Debug)]
pub struct Blake3Hasher {
    hasher: blake3::Hasher,
}

impl Blake3Hasher {
    /// Create a new hasher
    pub fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
        }
    }

    /// Create a keyed hasher
    pub fn new_keyed(key: &[u8; 32]) -> Self {
        Self {
            hasher: blake3::Hasher::new_keyed(key),
        }
    }

    /// Update with more data
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Finalize and get checksum
    pub fn finalize(self) -> Blake3Checksum {
        let hash = self.hasher.finalize();
        let mut checksum = [0u8; 16];
        checksum.copy_from_slice(&hash.as_bytes()[..16]);
        checksum
    }

    /// Finalize and get full hash
    pub fn finalize_full(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_verify() {
        let data = b"Hello, World!";
        let checksum = compute_blake3(data);

        assert!(verify_blake3(data, &checksum));
        assert!(!verify_blake3(b"Different data", &checksum));
    }

    #[test]
    fn test_incremental() {
        let data = b"Hello, World!";

        let direct = compute_blake3(data);

        let mut hasher = Blake3Hasher::new();
        hasher.update(b"Hello, ");
        hasher.update(b"World!");
        let incremental = hasher.finalize();

        assert_eq!(direct, incremental);
    }

    #[test]
    fn test_full_hash() {
        let data = b"test";
        let hash = full_blake3(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_keyed_hash() {
        let key = [0u8; 32];
        let data = b"test";

        let hash1 = keyed_blake3(&key, data);
        let hash2 = keyed_blake3(&key, data);

        assert_eq!(hash1, hash2);
    }
}
