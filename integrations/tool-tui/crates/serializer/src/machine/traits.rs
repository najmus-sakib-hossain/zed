//! DX-Machine traits for serialization and deserialization

use super::types::Result;

/// Trait for types that can be deserialized from DX-Machine format
pub trait DxMachineDeserialize: Sized {
    /// Deserialize from byte slice (zero-copy)
    fn from_bytes(bytes: &[u8]) -> Result<&Self>;

    /// Get the minimum buffer size required
    fn min_size() -> usize;
}

/// Trait for types that can be serialized to DX-Machine format
pub trait DxMachineSerialize {
    /// Serialize to byte buffer (in-place)
    fn serialize_to(&self, buffer: &mut Vec<u8>);

    /// Get the estimated serialized size
    fn serialized_size(&self) -> usize;
}
