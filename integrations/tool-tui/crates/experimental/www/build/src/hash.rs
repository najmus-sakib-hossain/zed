//! Content hashing utilities for cache keys

use blake3::Hasher;

/// Compute a content hash for the given data
///
/// Uses BLAKE3 for fast, cryptographically secure hashing.
///
/// # Arguments
///
/// * `data` - The data to hash
///
/// # Returns
///
/// A hex-encoded hash string
pub fn content_hash(data: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hash.to_hex().to_string()
}

/// Compute a content hash for a file
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// A hex-encoded hash string, or an error if the file cannot be read
///
/// # Errors
///
/// Returns an error if the file cannot be read
#[allow(dead_code)]
pub fn file_hash(path: &std::path::Path) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    Ok(content_hash(&data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_deterministic() {
        let data = b"hello world";
        let hash1 = content_hash(data);
        let hash2 = content_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_different_data() {
        let hash1 = content_hash(b"hello");
        let hash2 = content_hash(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_length() {
        let hash = content_hash(b"test");
        // BLAKE3 produces 32-byte hashes, which is 64 hex characters
        assert_eq!(hash.len(), 64);
    }
}
