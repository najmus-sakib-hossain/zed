//! RKYV compatibility layer with DX-Machine ultra-performance enhancements
//!
//! This module provides RKYV-compatible APIs while enabling access to DX-Machine's
//! advanced performance features through feature flags.

// Re-export RKYV directly for zero-overhead performance
pub use rkyv::Archive;
pub use rkyv::Deserialize as RkyvDeserialize;
pub use rkyv::Serialize as RkyvSerialize;
pub use rkyv::access_unchecked;
pub use rkyv::from_bytes;
pub use rkyv::to_bytes;

use crate::machine::DxMachineError;

/// # Safety
///
/// UNSAFE: Deserialize without validation (trust mode)
/// Only use with data from trusted sources (e.g., your own database).
/// Skips all validation.
#[inline(always)]
pub unsafe fn from_bytes_unchecked<T>(bytes: &[u8]) -> Result<T, DxMachineError>
where
    T: rkyv::Archive,
    T::Archived: rkyv::Deserialize<T, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    // SAFETY: Caller guarantees data is from trusted source
    unsafe {
        let archived = rkyv::access_unchecked::<T::Archived>(bytes);
        let mut deserializer = rkyv::de::Pool::new();
        archived
            .deserialize(rkyv::rancor::Strategy::wrap(&mut deserializer))
            .map_err(|_| DxMachineError::InvalidData("Deserialization failed".into()))
    }
}

/// Zero-copy access to archived data (no deserialization!)
///
/// This is the fastest way to access data - no allocation, no copying.
/// Returns a reference to the archived representation.
#[inline(always)]
pub fn access_archived<T>(bytes: &[u8]) -> Result<&T::Archived, DxMachineError>
where
    T: rkyv::Archive,
{
    // SAFETY: RKYV handles validation internally
    unsafe { Ok(rkyv::access_unchecked::<T::Archived>(bytes)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

    #[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
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

        let bytes = to_bytes::<rkyv::rancor::Error>(&original).unwrap();
        let decoded: TestStruct = from_bytes::<TestStruct, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_trust_mode() {
        let original = vec![1u64, 2, 3, 4, 5];
        let bytes = to_bytes::<rkyv::rancor::Error>(&original).unwrap();

        // Safe mode
        let decoded_safe: Vec<u64> = from_bytes::<Vec<u64>, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(original, decoded_safe);

        // Trust mode (faster)
        let decoded_trust: Vec<u64> = unsafe { from_bytes_unchecked(&bytes).unwrap() };
        assert_eq!(original, decoded_trust);
    }

    #[test]
    fn test_zero_copy_access() {
        let original = TestStruct {
            id: 42,
            name: "test".to_string(),
            active: true,
        };

        let bytes = to_bytes::<rkyv::rancor::Error>(&original).unwrap();

        // Zero-copy access (no deserialization!)
        let archived = access_archived::<TestStruct>(&bytes).unwrap();
        assert_eq!(archived.id, 42);
        assert_eq!(archived.active, true);
    }
}
