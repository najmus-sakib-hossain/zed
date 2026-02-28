//! XOR Delta state synchronization.
//!
//! Provides efficient incremental state updates using XOR differences
//! with run-length encoding for sparse changes.

use crate::DCPError;
use blake3;

/// XOR delta for state synchronization
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XorDelta {
    /// Hash of previous state
    pub prev_hash: [u8; 32],
    /// Hash of new state
    pub new_hash: [u8; 32],
    /// Run-length encoded XOR patch
    pub patch: Vec<u8>,
    /// Original new state length (needed for applying delta)
    pub new_len: usize,
}

impl XorDelta {
    /// Compute delta between two states
    pub fn compute(prev: &[u8], new: &[u8]) -> Self {
        // Compute XOR difference
        let max_len = prev.len().max(new.len());
        let mut xor = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let prev_byte = prev.get(i).copied().unwrap_or(0);
            let new_byte = new.get(i).copied().unwrap_or(0);
            xor.push(prev_byte ^ new_byte);
        }

        Self {
            prev_hash: *blake3::hash(prev).as_bytes(),
            new_hash: *blake3::hash(new).as_bytes(),
            patch: rle_compress(&xor),
            new_len: new.len(),
        }
    }

    /// Apply delta to state
    /// Returns the new state if successful
    pub fn apply(&self, state: &[u8]) -> Result<Vec<u8>, DCPError> {
        // Verify previous hash
        let current_hash = *blake3::hash(state).as_bytes();
        if current_hash != self.prev_hash {
            return Err(DCPError::HashMismatch);
        }

        // Decompress the XOR patch
        let xor = rle_decompress(&self.patch);

        // Apply XOR to create new state
        let mut new_state = Vec::with_capacity(self.new_len);
        for i in 0..self.new_len {
            let state_byte = state.get(i).copied().unwrap_or(0);
            let xor_byte = xor.get(i).copied().unwrap_or(0);
            new_state.push(state_byte ^ xor_byte);
        }

        // Verify new hash
        let result_hash = *blake3::hash(&new_state).as_bytes();
        if result_hash != self.new_hash {
            return Err(DCPError::HashMismatch);
        }

        Ok(new_state)
    }

    /// Get the size of the compressed patch
    pub fn patch_size(&self) -> usize {
        self.patch.len()
    }

    /// Check if this delta represents a sparse change
    /// (patch is smaller than the full state)
    pub fn is_sparse(&self, full_state_size: usize) -> bool {
        self.patch.len() < full_state_size
    }

    /// Verify the delta can be applied to a state
    pub fn verify_prev_hash(&self, state: &[u8]) -> bool {
        let current_hash = *blake3::hash(state).as_bytes();
        current_hash == self.prev_hash
    }
}

/// Run-length encode a byte slice
/// Format: [count, value] pairs where count is 1-255
/// For runs > 255, multiple pairs are used
fn rle_compress(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut i = 0;

    while i < data.len() {
        let value = data[i];
        let mut count = 1u8;

        // Count consecutive identical bytes (max 255)
        while i + (count as usize) < data.len()
            && data[i + (count as usize)] == value
            && count < 255
        {
            count += 1;
        }

        result.push(count);
        result.push(value);
        i += count as usize;
    }

    result
}

/// Run-length decode a byte slice
fn rle_decompress(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;

    while i + 1 < data.len() {
        let count = data[i] as usize;
        let value = data[i + 1];

        for _ in 0..count {
            result.push(value);
        }

        i += 2;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_compress_decompress() {
        let data = vec![0, 0, 0, 1, 1, 2, 2, 2, 2, 0];
        let compressed = rle_compress(&data);
        let decompressed = rle_decompress(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_rle_empty() {
        let data: Vec<u8> = vec![];
        let compressed = rle_compress(&data);
        let decompressed = rle_decompress(&compressed);
        assert!(compressed.is_empty());
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_rle_single_byte() {
        let data = vec![42];
        let compressed = rle_compress(&data);
        let decompressed = rle_decompress(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_rle_long_run() {
        // Test run longer than 255
        let data = vec![0u8; 300];
        let compressed = rle_compress(&data);
        let decompressed = rle_decompress(&compressed);
        assert_eq!(decompressed, data);
        // Should be compressed to 4 bytes: [255, 0, 45, 0]
        assert_eq!(compressed.len(), 4);
    }

    #[test]
    fn test_delta_basic() {
        let prev = b"hello world";
        let new = b"hello rust!";

        let delta = XorDelta::compute(prev, new);
        let result = delta.apply(prev).unwrap();

        assert_eq!(result, new);
    }

    #[test]
    fn test_delta_sparse_change() {
        // Large state with small change
        let mut prev = vec![0u8; 1000];
        let mut new = prev.clone();
        new[500] = 1; // Single byte change

        let delta = XorDelta::compute(&prev, &new);

        // Delta should be smaller than full state
        assert!(delta.is_sparse(new.len()));

        let result = delta.apply(&prev).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_delta_hash_mismatch() {
        let prev = b"hello";
        let new = b"world";

        let delta = XorDelta::compute(prev, new);

        // Try to apply to wrong state
        let wrong_state = b"wrong";
        let result = delta.apply(wrong_state);

        assert_eq!(result, Err(DCPError::HashMismatch));
    }

    #[test]
    fn test_delta_different_lengths() {
        let prev = b"short";
        let new = b"much longer string";

        let delta = XorDelta::compute(prev, new);
        let result = delta.apply(prev).unwrap();

        assert_eq!(result, new);
    }

    #[test]
    fn test_delta_shrinking() {
        let prev = b"much longer string";
        let new = b"short";

        let delta = XorDelta::compute(prev, new);
        let result = delta.apply(prev).unwrap();

        assert_eq!(result, new);
    }

    #[test]
    fn test_delta_empty_states() {
        let prev: &[u8] = b"";
        let new = b"new data";

        let delta = XorDelta::compute(prev, new);
        let result = delta.apply(prev).unwrap();

        assert_eq!(result, new);
    }

    #[test]
    fn test_delta_to_empty() {
        let prev = b"old data";
        let new: &[u8] = b"";

        let delta = XorDelta::compute(prev, new);
        let result = delta.apply(prev).unwrap();

        assert_eq!(result, new);
    }

    #[test]
    fn test_verify_prev_hash() {
        let prev = b"hello";
        let new = b"world";

        let delta = XorDelta::compute(prev, new);

        assert!(delta.verify_prev_hash(prev));
        assert!(!delta.verify_prev_hash(new));
    }
}
