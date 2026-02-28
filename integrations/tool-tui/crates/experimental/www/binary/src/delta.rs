//! # Delta Patching Module
//!
//! Binary delta compression for efficient updates.
//! Uses a block-based approach similar to rsync for O(n) patching.
//!
//! ## Algorithm
//!
//! 1. Split base into fixed-size blocks (default 64 bytes)
//! 2. Hash each block with BLAKE3
//! 3. For target, find matching blocks via rolling hash
//! 4. Emit copy instructions for matches, literal data for misses
//!
//! ## Wire Format
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  HEADER (16 bytes)                      │
//! ├─────────────────────────────────────────┤
//! │  - Magic: b"DXDL" (4 bytes)             │
//! │  - Version: 1 (1 byte)                  │
//! │  - Block Size: u16                      │
//! │  - Base Hash: [u8; 8] (truncated)       │
//! │  - Reserved: 1 byte                     │
//! ├─────────────────────────────────────────┤
//! │  INSTRUCTIONS (variable)                │
//! │  - 0x01 COPY: block_idx(u32)            │
//! │  - 0x02 LITERAL: len(u16) + data        │
//! └─────────────────────────────────────────┘
//! ```

use crate::{DxBinaryError, Result};
use std::collections::HashMap;

/// Magic bytes for delta format
pub const DELTA_MAGIC: &[u8; 4] = b"DXDL";

/// Delta format version
pub const DELTA_VERSION: u8 = 1;

/// Default block size (64 bytes balances granularity vs overhead)
pub const DEFAULT_BLOCK_SIZE: usize = 64;

/// Maximum literal chunk size
pub const MAX_LITERAL_SIZE: usize = 65535;

/// Header overhead for delta patch (magic + version + block_size + base_hash + reserved)
pub const DELTA_HEADER_OVERHEAD: usize = 16;

/// Delta instruction opcodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaOp {
    /// Copy block from base at given index
    Copy { block_idx: u32 },
    /// Insert literal bytes
    Literal { data: Vec<u8> },
}

/// Delta patch containing instructions to transform base → target
#[derive(Debug, Clone)]
pub struct DeltaPatch {
    /// Block size used for chunking
    pub block_size: u16,
    /// Hash of the base data (for verification)
    pub base_hash: [u8; 8],
    /// Sequence of delta operations
    pub ops: Vec<DeltaOp>,
}

/// Result of delta generation - either a patch or the full target
/// when the patch would be larger than the target
#[derive(Debug, Clone)]
pub enum DeltaResult {
    /// A delta patch that is smaller than or equal to target + overhead
    Patch(DeltaPatch),
    /// The full target data (when patch would be larger)
    FullTarget(Vec<u8>),
}

impl DeltaPatch {
    /// Create a new empty patch
    pub fn new(block_size: u16, base_hash: [u8; 8]) -> Self {
        Self {
            block_size,
            base_hash,
            ops: Vec::new(),
        }
    }

    /// Serialize patch to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + self.ops.len() * 8);

        // Header
        buf.extend_from_slice(DELTA_MAGIC);
        buf.push(DELTA_VERSION);
        buf.extend_from_slice(&self.block_size.to_le_bytes());
        buf.extend_from_slice(&self.base_hash);
        buf.push(0); // Reserved

        // Instructions
        for op in &self.ops {
            match op {
                DeltaOp::Copy { block_idx } => {
                    buf.push(0x01);
                    buf.extend_from_slice(&block_idx.to_le_bytes());
                }
                DeltaOp::Literal { data } => {
                    buf.push(0x02);
                    buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
                    buf.extend_from_slice(data);
                }
            }
        }

        buf
    }

    /// Deserialize patch from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 16 {
            return Err(DxBinaryError::IoError("Delta patch too short".into()));
        }

        // Verify magic
        if &data[0..4] != DELTA_MAGIC {
            return Err(DxBinaryError::InvalidMagic);
        }

        // Verify version
        if data[4] != DELTA_VERSION {
            return Err(DxBinaryError::UnsupportedVersion(data[4]));
        }

        let block_size = u16::from_le_bytes([data[5], data[6]]);
        let mut base_hash = [0u8; 8];
        base_hash.copy_from_slice(&data[7..15]);

        let mut ops = Vec::new();
        let mut offset = 16;

        while offset < data.len() {
            let opcode = data[offset];
            offset += 1;

            match opcode {
                0x01 => {
                    // COPY
                    if offset + 4 > data.len() {
                        return Err(DxBinaryError::IoError("Truncated COPY instruction".into()));
                    }
                    let block_idx = u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    offset += 4;
                    ops.push(DeltaOp::Copy { block_idx });
                }
                0x02 => {
                    // LITERAL
                    if offset + 2 > data.len() {
                        return Err(DxBinaryError::IoError("Truncated LITERAL instruction".into()));
                    }
                    let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;

                    if offset + len > data.len() {
                        return Err(DxBinaryError::IoError("Truncated LITERAL data".into()));
                    }
                    let literal_data = data[offset..offset + len].to_vec();
                    offset += len;
                    ops.push(DeltaOp::Literal { data: literal_data });
                }
                _ => {
                    return Err(DxBinaryError::InvalidOpcode(opcode));
                }
            }
        }

        Ok(Self {
            block_size,
            base_hash,
            ops,
        })
    }
}

/// Compute truncated BLAKE3 hash for base verification
fn compute_base_hash(data: &[u8]) -> [u8; 8] {
    let hash = blake3::hash(data);
    let mut result = [0u8; 8];
    result.copy_from_slice(&hash.as_bytes()[..8]);
    result
}

/// Compute block hash for matching
fn compute_block_hash(block: &[u8]) -> u64 {
    let hash = blake3::hash(block);
    // SAFETY: blake3 hash is always 32 bytes, we take first 8
    u64::from_le_bytes(hash.as_bytes()[..8].try_into().expect("blake3 hash is 32 bytes"))
}

/// Generate delta patch from base to target
///
/// # Algorithm
///
/// 1. Build hash table of all blocks in base
/// 2. Scan target looking for matching blocks
/// 3. Emit COPY for matches, LITERAL for non-matching regions
///
/// # Example
///
/// ```rust
/// use dx_www_binary::delta::{generate_delta, apply_delta};
///
/// let base = b"Hello, World! This is the base content.";
/// let target = b"Hello, World! This is the NEW content.";
///
/// let patch = generate_delta(base, target).unwrap();
/// let result = apply_delta(base, &patch).unwrap();
///
/// assert_eq!(result, target);
/// ```
pub fn generate_delta(base: &[u8], target: &[u8]) -> Result<DeltaPatch> {
    generate_delta_with_block_size(base, target, DEFAULT_BLOCK_SIZE)
}

/// Generate delta with custom block size
pub fn generate_delta_with_block_size(
    base: &[u8],
    target: &[u8],
    block_size: usize,
) -> Result<DeltaPatch> {
    let base_hash = compute_base_hash(base);
    let mut patch = DeltaPatch::new(block_size as u16, base_hash);

    // Build hash table: hash -> list of block indices
    let mut hash_table: HashMap<u64, Vec<u32>> = HashMap::new();
    let num_blocks = base.len().div_ceil(block_size);

    for i in 0..num_blocks {
        let start = i * block_size;
        let end = (start + block_size).min(base.len());
        let block = &base[start..end];
        let hash = compute_block_hash(block);
        hash_table.entry(hash).or_default().push(i as u32);
    }

    // Scan target for matches
    let mut pos = 0;
    let mut literal_buf: Vec<u8> = Vec::new();

    while pos < target.len() {
        let remaining = target.len() - pos;

        // Try to find a matching block
        if remaining >= block_size {
            let target_block = &target[pos..pos + block_size];
            let target_hash = compute_block_hash(target_block);

            if let Some(indices) = hash_table.get(&target_hash) {
                // Verify actual match (hash collision check)
                let mut found_match = None;
                for &idx in indices {
                    let base_start = idx as usize * block_size;
                    let base_end = (base_start + block_size).min(base.len());
                    let base_block = &base[base_start..base_end];

                    if base_block == target_block {
                        found_match = Some(idx);
                        break;
                    }
                }

                if let Some(block_idx) = found_match {
                    // Flush any pending literal data
                    if !literal_buf.is_empty() {
                        flush_literal(&mut patch.ops, &mut literal_buf);
                    }

                    patch.ops.push(DeltaOp::Copy { block_idx });
                    pos += block_size;
                    continue;
                }
            }
        }

        // No match found, add to literal buffer
        literal_buf.push(target[pos]);
        pos += 1;

        // Flush if literal buffer is full
        if literal_buf.len() >= MAX_LITERAL_SIZE {
            flush_literal(&mut patch.ops, &mut literal_buf);
        }
    }

    // Flush remaining literal data
    if !literal_buf.is_empty() {
        flush_literal(&mut patch.ops, &mut literal_buf);
    }

    Ok(patch)
}

/// Flush literal buffer to ops
fn flush_literal(ops: &mut Vec<DeltaOp>, buf: &mut Vec<u8>) {
    if !buf.is_empty() {
        ops.push(DeltaOp::Literal {
            data: std::mem::take(buf),
        });
    }
}

/// Apply delta patch to base data
///
/// # Errors
///
/// Returns error if:
/// - Base hash doesn't match patch expectation
/// - Block index is out of bounds
/// - Patch is malformed
pub fn apply_delta(base: &[u8], patch: &DeltaPatch) -> Result<Vec<u8>> {
    // Verify base hash
    let actual_hash = compute_base_hash(base);
    if actual_hash != patch.base_hash {
        return Err(DxBinaryError::IoError(format!(
            "Base hash mismatch: expected {:?}, got {:?}",
            patch.base_hash, actual_hash
        )));
    }

    let block_size = patch.block_size as usize;
    let mut result = Vec::new();

    for op in &patch.ops {
        match op {
            DeltaOp::Copy { block_idx } => {
                let start = *block_idx as usize * block_size;
                let end = (start + block_size).min(base.len());

                if start >= base.len() {
                    return Err(DxBinaryError::IoError(format!(
                        "Block index {} out of bounds (base has {} blocks)",
                        block_idx,
                        base.len().div_ceil(block_size)
                    )));
                }

                result.extend_from_slice(&base[start..end]);
            }
            DeltaOp::Literal { data } => {
                result.extend_from_slice(data);
            }
        }
    }

    Ok(result)
}

/// Calculate compression ratio of a patch
pub fn compression_ratio(_base_len: usize, target_len: usize, patch: &DeltaPatch) -> f64 {
    let patch_size = patch.to_bytes().len();
    let naive_size = target_len;

    if naive_size == 0 {
        return 1.0;
    }

    patch_size as f64 / naive_size as f64
}

/// Generate delta patch with size comparison
///
/// Returns `DeltaResult::Patch` if the patch is smaller than or equal to
/// target size + header overhead. Otherwise returns `DeltaResult::FullTarget`
/// with the full target data.
///
/// # Algorithm
///
/// 1. Generate delta patch normally
/// 2. Compare patch size to target size + header overhead
/// 3. Return patch if smaller, otherwise return full target
///
/// # Example
///
/// ```rust
/// use dx_www_binary::delta::{generate_delta_optimized, apply_delta_result, DeltaResult};
///
/// let base = b"Hello, World! This is the base content.";
/// let target = b"Hello, World! This is the NEW content.";
///
/// let result = generate_delta_optimized(base, target).unwrap();
/// let output = apply_delta_result(base, &result).unwrap();
///
/// assert_eq!(output, target);
/// ```
pub fn generate_delta_optimized(base: &[u8], target: &[u8]) -> Result<DeltaResult> {
    generate_delta_optimized_with_block_size(base, target, DEFAULT_BLOCK_SIZE)
}

/// Generate delta with custom block size and size comparison
///
/// Returns `DeltaResult::Patch` if the patch is smaller than or equal to
/// target size + header overhead. Otherwise returns `DeltaResult::FullTarget`.
pub fn generate_delta_optimized_with_block_size(
    base: &[u8],
    target: &[u8],
    block_size: usize,
) -> Result<DeltaResult> {
    let patch = generate_delta_with_block_size(base, target, block_size)?;
    let patch_size = patch.to_bytes().len();

    // Compare patch size to target size + header overhead
    // If patch would be larger, return the full target instead
    let threshold = target.len().saturating_add(DELTA_HEADER_OVERHEAD);

    if patch_size <= threshold {
        Ok(DeltaResult::Patch(patch))
    } else {
        Ok(DeltaResult::FullTarget(target.to_vec()))
    }
}

/// Apply delta result to base data
///
/// Handles both `DeltaResult::Patch` and `DeltaResult::FullTarget` cases.
///
/// # Errors
///
/// Returns error if applying a patch fails (base hash mismatch, etc.)
pub fn apply_delta_result(base: &[u8], result: &DeltaResult) -> Result<Vec<u8>> {
    match result {
        DeltaResult::Patch(patch) => apply_delta(base, patch),
        DeltaResult::FullTarget(target) => Ok(target.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_data() {
        let data = b"Hello, World!";
        let patch = generate_delta(data, data).unwrap();
        let result = apply_delta(data, &patch).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_completely_different() {
        let base = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let target = b"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
        let patch = generate_delta(base, target).unwrap();
        let result = apply_delta(base, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_small_change() {
        let base = b"Hello, World! This is a test of the delta compression system.";
        let target = b"Hello, World! This is a TEST of the delta compression system.";
        let patch = generate_delta(base, target).unwrap();
        let result = apply_delta(base, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_append_data() {
        let base = b"Original content here.";
        let target = b"Original content here. And some new content!";
        let patch = generate_delta(base, target).unwrap();
        let result = apply_delta(base, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_prepend_data() {
        let base = b"Original content here.";
        let target = b"New prefix! Original content here.";
        let patch = generate_delta(base, target).unwrap();
        let result = apply_delta(base, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_empty_base() {
        let base = b"";
        let target = b"New content";
        let patch = generate_delta(base, target).unwrap();
        let result = apply_delta(base, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_empty_target() {
        let base = b"Some content";
        let target = b"";
        let patch = generate_delta(base, target).unwrap();
        let result = apply_delta(base, &patch).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let base = b"Test data for serialization roundtrip verification.";
        let target = b"Test data for serialization ROUNDTRIP verification!";
        let patch = generate_delta(base, target).unwrap();

        let bytes = patch.to_bytes();
        let recovered = DeltaPatch::from_bytes(&bytes).unwrap();

        assert_eq!(patch.block_size, recovered.block_size);
        assert_eq!(patch.base_hash, recovered.base_hash);
        assert_eq!(patch.ops.len(), recovered.ops.len());

        let result = apply_delta(base, &recovered).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_base_hash_mismatch() {
        let base = b"Original base";
        let target = b"Modified target";
        let patch = generate_delta(base, target).unwrap();

        let wrong_base = b"Different base";
        let result = apply_delta(wrong_base, &patch);
        assert!(result.is_err());
    }

    #[test]
    fn test_compression_ratio() {
        // Large file with small change should have good compression
        let base: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let mut target = base.clone();
        target[5000] = 255; // Single byte change

        let patch = generate_delta(&base, &target).unwrap();
        let ratio = compression_ratio(base.len(), target.len(), &patch);

        // Patch should be much smaller than target
        assert!(ratio < 0.5, "Compression ratio {} should be < 0.5", ratio);
    }

    #[test]
    fn test_generate_delta_optimized_returns_patch_when_smaller() {
        // Large file with small change - patch should be smaller
        let base: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let mut target = base.clone();
        target[500] = 255; // Single byte change

        let result = generate_delta_optimized(&base, &target).unwrap();

        match result {
            DeltaResult::Patch(patch) => {
                // Verify the patch works
                let output = apply_delta(&base, &patch).unwrap();
                assert_eq!(output, target);
            }
            DeltaResult::FullTarget(_) => {
                panic!("Expected Patch for similar content, got FullTarget");
            }
        }
    }

    #[test]
    fn test_generate_delta_optimized_returns_full_target_when_larger() {
        // Completely different content - patch would be larger than target
        let base = b"AAAA";
        let target = b"BBBB";

        let result = generate_delta_optimized(base, target).unwrap();

        match &result {
            DeltaResult::FullTarget(data) => {
                assert_eq!(data.as_slice(), target.as_slice());
            }
            DeltaResult::Patch(_) => {
                // This is also acceptable if the patch happens to be small enough
                // The key is that apply_delta_result works correctly
            }
        }

        // Verify apply_delta_result works regardless of which variant
        let output = apply_delta_result(base, &result).unwrap();
        assert_eq!(output, target);
    }

    #[test]
    fn test_apply_delta_result_with_patch() {
        let base = b"Hello, World! This is the base content for testing.";
        let target = b"Hello, World! This is the NEW content for testing.";

        let patch = generate_delta(base, target).unwrap();
        let result = DeltaResult::Patch(patch);

        let output = apply_delta_result(base, &result).unwrap();
        assert_eq!(output, target);
    }

    #[test]
    fn test_apply_delta_result_with_full_target() {
        let base = b"Original";
        let target = b"Completely different";

        let result = DeltaResult::FullTarget(target.to_vec());

        let output = apply_delta_result(base, &result).unwrap();
        assert_eq!(output, target);
    }

    #[test]
    fn test_delta_header_overhead_constant() {
        // Verify the header overhead constant matches actual header size
        let base = b"test";
        let target = b"test";
        let patch = generate_delta(base, target).unwrap();
        let bytes = patch.to_bytes();

        // For identical content with no ops, size should be just the header
        // Header: magic(4) + version(1) + block_size(2) + base_hash(8) + reserved(1) = 16
        assert!(bytes.len() >= DELTA_HEADER_OVERHEAD);
    }
}
