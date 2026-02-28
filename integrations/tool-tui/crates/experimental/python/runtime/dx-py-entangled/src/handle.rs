//! Serializable handle for cross-process transfer

use crate::object::TypeInfo;
use crate::{EntangledError, EntangledObject, SharedMemoryRegion};
use std::sync::Arc;
use uuid::Uuid;

/// Serializable handle for transferring entangled objects between processes
#[derive(Debug, Clone)]
pub struct EntangledHandle {
    /// Object UUID
    pub id: Uuid,
    /// Region name
    pub region_name: String,
    /// Offset in the region
    pub offset: u64,
    /// Type information
    pub type_info: u8,
    /// Data size
    pub size: u64,
}

impl EntangledHandle {
    /// Create a handle from an entangled object
    pub fn from_object(obj: &EntangledObject) -> Self {
        Self {
            id: obj.id(),
            region_name: obj.region_name().to_string(),
            offset: obj.offset(),
            type_info: obj.type_info() as u8,
            size: obj.size() as u64,
        }
    }

    /// Reconstruct an entangled object from a handle
    pub fn to_object(&self) -> Result<EntangledObject, EntangledError> {
        let region = Arc::new(SharedMemoryRegion::open(&self.region_name)?);
        EntangledObject::open(region, self.offset)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(64);

        // UUID (16 bytes)
        bytes.extend_from_slice(self.id.as_bytes());

        // Region name length and data
        let name_bytes = self.region_name.as_bytes();
        bytes.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name_bytes);

        // Offset
        bytes.extend_from_slice(&self.offset.to_le_bytes());

        // Type info
        bytes.push(self.type_info);

        // Size
        bytes.extend_from_slice(&self.size.to_le_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 + 4 {
            return None;
        }

        let id = Uuid::from_bytes(bytes[0..16].try_into().ok()?);

        let name_len = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
        if bytes.len() < 20 + name_len + 8 + 1 + 8 {
            return None;
        }

        let region_name = String::from_utf8(bytes[20..20 + name_len].to_vec()).ok()?;

        let offset_start = 20 + name_len;
        let offset = u64::from_le_bytes(bytes[offset_start..offset_start + 8].try_into().ok()?);

        let type_info = bytes[offset_start + 8];

        let size = u64::from_le_bytes(bytes[offset_start + 9..offset_start + 17].try_into().ok()?);

        Some(Self {
            id,
            region_name,
            offset,
            type_info,
            size,
        })
    }

    /// Get the type info
    pub fn get_type_info(&self) -> Option<TypeInfo> {
        TypeInfo::from_u8(self.type_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    // Global counter for unique test names
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn create_test_region() -> (Arc<SharedMemoryRegion>, String) {
        let counter = TEST_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        let name = format!("test_handle_{}_{}", std::process::id(), counter);
        let region = Arc::new(SharedMemoryRegion::create(&name, 1024 * 1024).unwrap());
        (region, name)
    }

    fn cleanup_region(name: &str) {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("dx-py-entangled").join(format!("{}.shm", name));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_handle_roundtrip() {
        let handle = EntangledHandle {
            id: Uuid::new_v4(),
            region_name: "test_region".to_string(),
            offset: 0x1234,
            type_info: TypeInfo::FloatArray as u8,
            size: 1000,
        };

        let bytes = handle.to_bytes();
        let restored = EntangledHandle::from_bytes(&bytes).unwrap();

        assert_eq!(handle.id, restored.id);
        assert_eq!(handle.region_name, restored.region_name);
        assert_eq!(handle.offset, restored.offset);
        assert_eq!(handle.type_info, restored.type_info);
        assert_eq!(handle.size, restored.size);
    }

    #[test]
    fn test_handle_from_object() {
        let (region, name) = create_test_region();

        let obj = EntangledObject::create(region.clone(), TypeInfo::Bytes, &[1, 2, 3, 4]).unwrap();

        let handle = EntangledHandle::from_object(&obj);

        assert_eq!(handle.id, obj.id());
        assert_eq!(handle.region_name, obj.region_name());
        assert_eq!(handle.offset, obj.offset());
        assert_eq!(handle.type_info, TypeInfo::Bytes as u8);
        assert_eq!(handle.size, 4);

        drop(obj);
        drop(region);
        cleanup_region(&name);
    }
}
