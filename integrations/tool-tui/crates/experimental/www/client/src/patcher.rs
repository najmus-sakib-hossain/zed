/// Client-side binary patcher
///
/// Applies XOR-based block patches to reconstruct updated binaries
/// in-place. Designed for sub-millisecond performance on typical patches.
///
/// # Algorithm
///
/// 1. Old binary divided into 4KB blocks
/// 2. Server sends XOR diff for changed blocks
/// 3. Client XORs each block: new = old ^ xor_data
/// 4. Result is the updated binary
///
/// # Performance
///
/// - Block size: 4096 bytes (cache-friendly)
/// - Operation: XOR (CPU-level instruction)
/// - Target: < 1ms for 20KB patch (~5 blocks)

use dx_packet::{PatchHeader, BLOCK_SIZE};

/// Block size constant (re-exported from dx-packet)
pub const PATCH_BLOCK_SIZE: usize = BLOCK_SIZE;

/// Patch block entry
///
/// Represents a single block that needs to be patched
#[derive(Debug, Clone)]
pub struct PatchBlock {
    /// Block index (multiply by BLOCK_SIZE to get byte offset)
    pub index: u32,
    /// XOR data to apply to this block
    pub xor_data: Vec<u8>,
}

/// Complete patch operation
///
/// Contains header + all blocks to patch
#[derive(Debug, Clone)]
pub struct Patch {
    /// Patch metadata
    pub header: PatchHeader,
    /// Blocks to patch
    pub blocks: Vec<PatchBlock>,
}

/// Patcher state
///
/// Holds the old binary and accumulated patch data
pub struct Patcher {
    /// The original binary to patch
    pub(crate) old_binary: Option<Vec<u8>>,
    /// Accumulated patch data
    patch_data: Option<Patch>,
}

impl Patcher {
    /// Create new patcher
    pub fn new() -> Self {
        Self {
            old_binary: None,
            patch_data: None,
        }
    }

    /// Set the old binary to patch
    pub fn set_old_binary(&mut self, binary: Vec<u8>) {
        self.old_binary = Some(binary);
    }

    /// Parse and store patch data
    ///
    /// # Format
    ///
    /// ```text
    /// [PatchHeader: 17 bytes]
    /// [Block Count: 4 bytes]
    /// [Block 1: index:4 + length:2 + data:N]
    /// [Block 2: index:4 + length:2 + data:N]
    /// ...
    /// ```
    pub fn set_patch_data(&mut self, data: &[u8]) -> Result<(), u8> {
        if data.len() < 21 {
            // Need at least header (17) + block count (4)
            return Err(1); // ErrorCode::InvalidPatchData
        }

        // Parse header
        let header = PatchHeader::from_bytes(&data[0..17])
            .ok_or(2)?; // ErrorCode::InvalidPatchHeader

        // Parse block count
        let block_count = u32::from_le_bytes([data[17], data[18], data[19], data[20]]) as usize;

        // Parse blocks
        let mut blocks = Vec::with_capacity(block_count);
        let mut offset = 21;

        for _ in 0..block_count {
            if offset + 6 > data.len() {
                return Err(3); // ErrorCode::TruncatedPatch
            }

            // Read block index (4 bytes)
            let index = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);

            // Read XOR data length (2 bytes)
            let length = u16::from_le_bytes([data[offset + 4], data[offset + 5]]) as usize;

            offset += 6;

            if offset + length > data.len() {
                return Err(4); // ErrorCode::InvalidBlockData
            }

            // Read XOR data
            let xor_data = data[offset..offset + length].to_vec();
            offset += length;

            blocks.push(PatchBlock { index, xor_data });
        }

        self.patch_data = Some(Patch { header, blocks });
        Ok(())
    }

    /// Apply patch and return new binary
    ///
    /// # Performance
    ///
    /// - O(n) where n = total XOR bytes
    /// - XOR is CPU-level instruction (very fast)
    /// - In-place modification minimizes allocations
    ///
    /// # Returns
    ///
    /// - Ok(new_binary) on success
    /// - Err(code) if patch fails
    pub fn apply_patch(&mut self) -> Result<Vec<u8>, u8> {
        let old_binary = self.old_binary.as_ref().ok_or(5)?; // ErrorCode::NoBinarySet
        let patch = self.patch_data.as_ref().ok_or(6)?; // ErrorCode::NoPatchSet

        // Clone old binary (we'll modify it in place)
        let mut new_binary = old_binary.clone();

        // Apply each block
        for block in &patch.blocks {
            let offset = (block.index as usize) * PATCH_BLOCK_SIZE;

            // Bounds check
            if offset >= new_binary.len() {
                return Err(7); // ErrorCode::BlockOutOfBounds
            }

            let end = (offset + block.xor_data.len()).min(new_binary.len());
            let target_len = end - offset;

            // XOR in place
            for i in 0..target_len.min(block.xor_data.len()) {
                new_binary[offset + i] ^= block.xor_data[i];
            }
        }

        Ok(new_binary)
    }

    /// Apply patch directly to a buffer without cloning
    ///
    /// # Performance
    ///
    /// Faster than apply_patch() as it modifies in-place.
    /// Use when you can mutate the original buffer.
    ///
    /// # Safety
    ///
    /// Buffer must be large enough for all patch blocks
    pub fn apply_patch_inplace(buffer: &mut [u8], patch_data: &[u8]) -> Result<(), u8> {
        if patch_data.len() < 21 {
            return Err(1); // ErrorCode::InvalidPatchData
        }

        // Parse header (we validate but don't use it for now)
        let _header = PatchHeader::from_bytes(&patch_data[0..17])
            .ok_or(2)?; // ErrorCode::InvalidPatchHeader

        // Parse block count
        let block_count =
            u32::from_le_bytes([patch_data[17], patch_data[18], patch_data[19], patch_data[20]])
                as usize;

        // Parse and apply blocks
        let mut offset = 21;

        for _ in 0..block_count {
            if offset + 6 > patch_data.len() {
                return Err(3); // ErrorCode::TruncatedPatch
            }

            // Read block index
            let index = u32::from_le_bytes([
                patch_data[offset],
                patch_data[offset + 1],
                patch_data[offset + 2],
                patch_data[offset + 3],
            ]);

            // Read length
            let length =
                u16::from_le_bytes([patch_data[offset + 4], patch_data[offset + 5]]) as usize;

            offset += 6;

            if offset + length > patch_data.len() {
                return Err(4); // ErrorCode::InvalidBlockData
            }

            // Calculate buffer offset
            let buf_offset = (index as usize) * PATCH_BLOCK_SIZE;

            if buf_offset >= buffer.len() {
                return Err(7); // ErrorCode::BlockOutOfBounds
            }

            // XOR in place
            let end = (buf_offset + length).min(buffer.len());
            let target_len = end - buf_offset;

            for i in 0..target_len.min(length) {
                buffer[buf_offset + i] ^= patch_data[offset + i];
            }

            offset += length;
        }

        Ok(())
    }

    /// Get the patched binary (consumes patcher)
    pub fn take_patched_binary(mut self) -> Result<Vec<u8>, u8> {
        self.apply_patch()
    }
}

impl Default for Patcher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Helper: Create a simple patch for testing
    fn create_test_patch(block_index: u32, xor_data: Vec<u8>) -> Vec<u8> {
        let mut patch = Vec::new();

        // Header (17 bytes)
        let header = PatchHeader::new(0x1111, 0x2222, 1); // algo=1 for XOR
        patch.extend_from_slice(&header.to_bytes());

        // Block count (1 block)
        patch.extend_from_slice(&1u32.to_le_bytes());

        // Block data
        patch.extend_from_slice(&block_index.to_le_bytes()); // index
        patch.extend_from_slice(&(xor_data.len() as u16).to_le_bytes()); // length
        patch.extend_from_slice(&xor_data); // data

        patch
    }

    #[test]
    fn test_patcher_single_block() {
        let mut patcher = Patcher::new();

        // Old binary: 8KB of zeros
        let old_binary = vec![0u8; 8192];
        patcher.set_old_binary(old_binary);

        // Patch: XOR first 10 bytes of block 0 with pattern
        let xor_data = vec![0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66];
        let patch = create_test_patch(0, xor_data.clone());

        patcher.set_patch_data(&patch).unwrap();

        // Apply patch
        let new_binary = patcher.apply_patch().unwrap();

        // Verify: First 10 bytes should be XORed
        assert_eq!(new_binary[0], 0xFF);
        assert_eq!(new_binary[1], 0xEE);
        assert_eq!(new_binary[9], 0x66);

        // Rest should be unchanged (zeros)
        assert_eq!(new_binary[10], 0x00);
        assert_eq!(new_binary[100], 0x00);
    }

    #[test]
    fn test_patcher_multiple_blocks() {
        let mut patcher = Patcher::new();

        // Old binary: 12KB
        let old_binary = vec![0x00u8; 12288];
        patcher.set_old_binary(old_binary);

        // Patch with 2 blocks
        let mut patch = Vec::new();

        // Header
        let header = PatchHeader::new(0x1111, 0x2222, 1);
        patch.extend_from_slice(&header.to_bytes());

        // Block count (2 blocks)
        patch.extend_from_slice(&2u32.to_le_bytes());

        // Block 0: Change first byte
        patch.extend_from_slice(&0u32.to_le_bytes()); // index 0
        patch.extend_from_slice(&1u16.to_le_bytes()); // length 1
        patch.push(0xAA); // XOR data

        // Block 1: Change byte at offset 4096 (second block)
        patch.extend_from_slice(&1u32.to_le_bytes()); // index 1
        patch.extend_from_slice(&1u16.to_le_bytes()); // length 1
        patch.push(0xBB); // XOR data

        patcher.set_patch_data(&patch).unwrap();
        let new_binary = patcher.apply_patch().unwrap();

        // Verify
        assert_eq!(new_binary[0], 0xAA); // Block 0
        assert_eq!(new_binary[4096], 0xBB); // Block 1
        assert_eq!(new_binary[1], 0x00); // Unchanged
        assert_eq!(new_binary[4097], 0x00); // Unchanged
    }

    #[test]
    fn test_patcher_inplace() {
        // Create buffer
        let mut buffer = vec![0x00u8; 8192];

        // Create patch
        let xor_data = vec![0xFF, 0xEE, 0xDD];
        let patch = create_test_patch(0, xor_data);

        // Apply in-place
        Patcher::apply_patch_inplace(&mut buffer, &patch).unwrap();

        // Verify
        assert_eq!(buffer[0], 0xFF);
        assert_eq!(buffer[1], 0xEE);
        assert_eq!(buffer[2], 0xDD);
        assert_eq!(buffer[3], 0x00);
    }

    #[test]
    fn test_patcher_xor_property() {
        // XOR property: old ^ diff = new, new ^ diff = old

        let old = vec![0x12, 0x34, 0x56, 0x78];
        let new = vec![0xAB, 0xCD, 0xEF, 0x01];

        // Calculate diff: old ^ new
        let diff: Vec<u8> = old.iter().zip(new.iter()).map(|(a, b)| a ^ b).collect();

        // Apply patch
        let mut patcher = Patcher::new();
        patcher.set_old_binary(old.clone());

        let patch = create_test_patch(0, diff.clone());
        patcher.set_patch_data(&patch).unwrap();

        let result = patcher.apply_patch().unwrap();

        // Should equal new
        assert_eq!(&result[0..4], &new[..]);

        // Reverse: Apply same patch to new, should get old
        let mut patcher2 = Patcher::new();
        patcher2.set_old_binary(new);
        let patch2 = create_test_patch(0, diff);
        patcher2.set_patch_data(&patch2).unwrap();
        let result2 = patcher2.apply_patch().unwrap();

        assert_eq!(&result2[0..4], &old[..]);
    }

    #[test]
    fn test_patcher_empty_patch() {
        let mut patcher = Patcher::new();
        let old_binary = vec![0xAAu8; 4096];
        patcher.set_old_binary(old_binary.clone());

        // Patch with 0 blocks
        let mut patch = Vec::new();
        let header = PatchHeader::new(0x1111, 0x2222, 1);
        patch.extend_from_slice(&header.to_bytes());
        patch.extend_from_slice(&0u32.to_le_bytes()); // 0 blocks

        patcher.set_patch_data(&patch).unwrap();
        let new_binary = patcher.apply_patch().unwrap();

        // Should be unchanged
        assert_eq!(new_binary, old_binary);
    }

    #[test]
    fn test_patcher_large_block() {
        let mut patcher = Patcher::new();

        // 8KB binary
        let mut old_binary = vec![0x00u8; 8192];
        // Fill with pattern
        for i in 0..old_binary.len() {
            old_binary[i] = (i % 256) as u8;
        }

        patcher.set_old_binary(old_binary.clone());

        // Create large XOR data (2KB)
        let xor_data: Vec<u8> = (0..2048).map(|i| ((i * 3) % 256) as u8).collect();
        let patch = create_test_patch(0, xor_data.clone());

        patcher.set_patch_data(&patch).unwrap();
        let new_binary = patcher.apply_patch().unwrap();

        // Verify XOR was applied
        for i in 0..2048 {
            let expected = old_binary[i] ^ xor_data[i];
            assert_eq!(new_binary[i], expected);
        }

        // Rest unchanged
        assert_eq!(new_binary[2048], old_binary[2048]);
    }
}
