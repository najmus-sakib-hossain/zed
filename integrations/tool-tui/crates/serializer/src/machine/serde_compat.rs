//! Serde-compatible serialization for DX-Machine format
//!
//! This module provides high-level serialization/deserialization functions
//! compatible with serde traits for easy integration with existing code.

use crate::machine::DxMachineError;
use serde::{Deserialize, Serialize};

/// Serialize a value to DX-Machine binary format using native binary serialization
pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, DxMachineError> {
    // Serialize directly with bincode
    let payload = bincode::serialize(value)
        .map_err(|e| DxMachineError::InvalidData(format!("Serialization failed: {}", e)))?;

    // Pre-allocate exact size needed
    let total_size = 16 + payload.len();
    let mut buffer = Vec::with_capacity(total_size);

    // Write header (16 bytes total)
    buffer.extend_from_slice(&[
        0x5A, 0x44, // Magic bytes "ZD"
        0x01, // Version
        0x01, // Flags: 0x01 = native binary format
    ]);

    // Write payload length (4 bytes, little-endian)
    buffer.extend_from_slice(&(payload.len() as u32).to_le_bytes());

    // Padding to 16 bytes (8 zero bytes)
    buffer.extend_from_slice(&[0u8; 8]);

    // Write binary payload (no copy, move ownership)
    buffer.extend_from_slice(&payload);

    Ok(buffer)
}

/// Deserialize a value from DX-Machine binary format
pub fn from_bytes<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, DxMachineError> {
    // Fast path validation
    if bytes.len() < 16 {
        return Err(DxMachineError::BufferTooSmall {
            required: 16,
            actual: bytes.len(),
        });
    }

    // Check magic bytes (branchless on modern CPUs)
    if bytes[0] != 0x5A || bytes[1] != 0x44 {
        return Err(DxMachineError::InvalidMagic);
    }

    // Read payload length (single u32 load)
    let payload_len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;

    // Validate buffer size
    let required = 16 + payload_len;
    if bytes.len() < required {
        return Err(DxMachineError::BufferTooSmall {
            required,
            actual: bytes.len(),
        });
    }

    // Extract payload slice (zero-copy)
    let payload_bytes = &bytes[16..required];

    // Deserialize from binary
    bincode::deserialize(payload_bytes)
        .map_err(|e| DxMachineError::InvalidData(format!("Deserialization failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        id: u64,
        name: String,
        active: bool,
    }

    #[test]
    fn test_roundtrip() {
        let original = TestStruct {
            id: 42,
            name: "test".to_string(),
            active: true,
        };

        let bytes = to_bytes(&original).unwrap();

        // Verify magic bytes
        assert_eq!(bytes[0], 0x5A);
        assert_eq!(bytes[1], 0x44);

        let decoded: TestStruct = from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_vec_roundtrip() {
        let original = vec![1, 2, 3, 4, 5];
        let bytes = to_bytes(&original).unwrap();
        let decoded: Vec<i32> = from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
