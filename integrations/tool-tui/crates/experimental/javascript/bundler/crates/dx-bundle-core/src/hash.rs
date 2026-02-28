//! High-performance hashing utilities for DX JS Bundler

use blake3::Hasher;

/// Content hash (128-bit for collision resistance)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ContentHash([u8; 16]);

impl ContentHash {
    /// Hash bytes using xxHash (fast) for module IDs
    #[inline(always)]
    pub fn xxhash(data: &[u8]) -> u64 {
        xxhash_rust::xxh64::xxh64(data, 0)
    }

    /// Hash bytes using xxh3 (faster) for content hashing
    #[inline(always)]
    pub fn xxh3(data: &[u8]) -> Self {
        let hash = xxhash_rust::xxh3::xxh3_128(data);
        Self(hash.to_le_bytes())
    }

    /// Hash bytes using BLAKE3 (crypto-strength) for cache keys
    pub fn blake3(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        let bytes = hash.as_bytes();
        let mut result = [0u8; 16];
        result.copy_from_slice(&bytes[..16]);
        Self(result)
    }

    /// Hash file contents efficiently
    pub fn hash_file(path: &std::path::Path) -> std::io::Result<Self> {
        use std::io::Read;

        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::with_capacity(64 * 1024, file);
        let mut hasher = Hasher::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        let bytes = hash.as_bytes();
        let mut result = [0u8; 16];
        result.copy_from_slice(&bytes[..16]);
        Ok(Self(result))
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Path hasher for module IDs
pub struct PathHasher;

impl PathHasher {
    /// Hash path to module ID
    #[inline(always)]
    pub fn hash(path: &std::path::Path) -> u64 {
        let path_str = path.to_string_lossy();
        ContentHash::xxhash(path_str.as_bytes())
    }

    /// Hash path bytes to module ID
    #[inline(always)]
    pub fn hash_bytes(path: &[u8]) -> u64 {
        ContentHash::xxhash(path)
    }
}

/// Incremental hasher for streaming
pub struct IncrementalHasher {
    hasher: Hasher,
}

impl IncrementalHasher {
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize(self) -> ContentHash {
        let hash = self.hasher.finalize();
        let bytes = hash.as_bytes();
        let mut result = [0u8; 16];
        result.copy_from_slice(&bytes[..16]);
        ContentHash(result)
    }
}

impl Default for IncrementalHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xxhash() {
        let hash1 = ContentHash::xxhash(b"hello");
        let hash2 = ContentHash::xxhash(b"hello");
        assert_eq!(hash1, hash2);

        let hash3 = ContentHash::xxhash(b"world");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_xxh3() {
        let hash1 = ContentHash::xxh3(b"hello world");
        let hash2 = ContentHash::xxh3(b"hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_blake3() {
        let hash = ContentHash::blake3(b"test content");
        assert_eq!(hash.as_bytes().len(), 16);
    }
}
