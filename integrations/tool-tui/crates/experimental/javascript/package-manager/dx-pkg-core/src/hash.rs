use xxhash_rust::xxh3::{xxh3_128, xxh3_64};

/// Content hash type (128-bit xxhash)
pub type ContentHash = u128;

/// Compute xxhash64 of data
#[inline]
pub fn xxhash64(data: &[u8]) -> u64 {
    xxh3_64(data)
}

/// Compute xxhash128 of data
#[inline]
pub fn xxhash128(data: &[u8]) -> u128 {
    xxh3_128(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xxhash64() {
        let data = b"hello world";
        let hash1 = xxhash64(data);
        let hash2 = xxhash64(data);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, 0);
    }

    #[test]
    fn test_xxhash128() {
        let data = b"hello world";
        let hash1 = xxhash128(data);
        let hash2 = xxhash128(data);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, 0);
    }

    #[test]
    fn test_different_inputs() {
        let hash1 = xxhash64(b"hello");
        let hash2 = xxhash64(b"world");
        assert_ne!(hash1, hash2);
    }
}
