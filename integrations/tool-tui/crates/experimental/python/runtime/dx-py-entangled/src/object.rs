//! Entangled object with optimistic concurrency

use crate::{EntangledError, SharedMemoryRegion};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Type information for entangled objects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeInfo {
    /// Raw bytes
    Bytes = 0,
    /// Integer
    Int = 1,
    /// Float
    Float = 2,
    /// String (UTF-8)
    String = 3,
    /// Array of f64
    FloatArray = 4,
    /// Array of i64
    IntArray = 5,
    /// Generic object
    Object = 6,
}

impl TypeInfo {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Bytes),
            1 => Some(Self::Int),
            2 => Some(Self::Float),
            3 => Some(Self::String),
            4 => Some(Self::FloatArray),
            5 => Some(Self::IntArray),
            6 => Some(Self::Object),
            _ => None,
        }
    }
}

/// Object header in shared memory
#[repr(C)]
struct ObjectHeader {
    /// Object UUID (16 bytes)
    id: [u8; 16],
    /// Version for optimistic concurrency
    version: AtomicU64,
    /// Type information
    type_info: u8,
    /// Reserved
    _reserved: [u8; 7],
    /// Data size
    size: u64,
}

const OBJECT_HEADER_SIZE: usize = std::mem::size_of::<ObjectHeader>();

/// Entangled object that can be shared across processes
pub struct EntangledObject {
    /// Unique identifier
    id: Uuid,
    /// Shared memory region
    region: Arc<SharedMemoryRegion>,
    /// Offset in the region
    offset: u64,
    /// Type information
    type_info: TypeInfo,
    /// Data size
    size: usize,
}

impl EntangledObject {
    /// Create a new entangled object
    pub fn create(
        region: Arc<SharedMemoryRegion>,
        type_info: TypeInfo,
        data: &[u8],
    ) -> Result<Self, EntangledError> {
        let id = Uuid::new_v4();
        let total_size = OBJECT_HEADER_SIZE + data.len();

        let offset = region.allocate(total_size, 8)?;

        // Write header
        let header_ptr = region.get_mut_ptr(offset) as *mut ObjectHeader;
        unsafe {
            (*header_ptr).id = *id.as_bytes();
            (*header_ptr).version = AtomicU64::new(1);
            (*header_ptr).type_info = type_info as u8;
            (*header_ptr)._reserved = [0; 7];
            (*header_ptr).size = data.len() as u64;
        }

        // Write data
        region.write(offset + OBJECT_HEADER_SIZE as u64, data)?;

        Ok(Self {
            id,
            region,
            offset,
            type_info,
            size: data.len(),
        })
    }

    /// Open an existing entangled object by offset
    pub fn open(region: Arc<SharedMemoryRegion>, offset: u64) -> Result<Self, EntangledError> {
        let header_ptr = region.get_ptr(offset) as *const ObjectHeader;

        let (id, type_info, size) = unsafe {
            let id = Uuid::from_bytes((*header_ptr).id);
            let type_info =
                TypeInfo::from_u8((*header_ptr).type_info).ok_or(EntangledError::TypeMismatch)?;
            let size = (*header_ptr).size as usize;
            (id, type_info, size)
        };

        Ok(Self {
            id,
            region,
            offset,
            type_info,
            size,
        })
    }

    /// Get the object ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the type info
    pub fn type_info(&self) -> TypeInfo {
        self.type_info
    }

    /// Get the data size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        let header = self.header();
        header.version.load(Ordering::Acquire)
    }

    /// Read the object data (zero-copy)
    pub fn read(&self) -> &[u8] {
        self.region.memory_barrier();
        let data_offset = self.offset + OBJECT_HEADER_SIZE as u64;
        self.region.read(data_offset, self.size).unwrap()
    }

    /// Write data with optimistic concurrency control
    pub fn write(&self, data: &[u8], expected_version: u64) -> Result<u64, EntangledError> {
        if data.len() != self.size {
            return Err(EntangledError::TypeMismatch);
        }

        let header = self.header();

        // Try to increment version atomically
        let new_version = expected_version + 1;
        if header
            .version
            .compare_exchange(expected_version, new_version, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(EntangledError::ConcurrencyConflict);
        }

        // Write data
        let data_offset = self.offset + OBJECT_HEADER_SIZE as u64;
        self.region.write(data_offset, data)?;

        self.region.memory_barrier();

        Ok(new_version)
    }

    /// Compare-and-swap write
    pub fn cas_write(&self, expected: &[u8], new_data: &[u8]) -> Result<bool, EntangledError> {
        if expected.len() != self.size || new_data.len() != self.size {
            return Err(EntangledError::TypeMismatch);
        }

        let current = self.read();
        if current != expected {
            return Ok(false);
        }

        let version = self.version();
        match self.write(new_data, version) {
            Ok(_) => Ok(true),
            Err(EntangledError::ConcurrencyConflict) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get the object header
    fn header(&self) -> &ObjectHeader {
        unsafe { &*(self.region.get_ptr(self.offset) as *const ObjectHeader) }
    }

    /// Get the offset in the region
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get the region name
    pub fn region_name(&self) -> &str {
        self.region.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Global counter for unique test names
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn create_test_region() -> (Arc<SharedMemoryRegion>, String) {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let name = format!("test_obj_{}_{}", std::process::id(), counter);
        let region = Arc::new(SharedMemoryRegion::create(&name, 1024 * 1024).unwrap());
        (region, name)
    }

    fn cleanup_region(name: &str) {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("dx-py-entangled").join(format!("{}.shm", name));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_create_object() {
        let (region, name) = create_test_region();
        let data = b"Hello, World!";

        let obj = EntangledObject::create(region.clone(), TypeInfo::String, data).unwrap();

        assert_eq!(obj.type_info(), TypeInfo::String);
        assert_eq!(obj.size(), data.len());
        assert_eq!(obj.version(), 1);
        assert_eq!(obj.read(), data);

        drop(obj);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_write_with_version() {
        let (region, name) = create_test_region();
        let data = vec![0u8; 100];

        let obj = EntangledObject::create(region.clone(), TypeInfo::Bytes, &data).unwrap();

        let new_data = vec![1u8; 100];
        let new_version = obj.write(&new_data, 1).unwrap();

        assert_eq!(new_version, 2);
        assert_eq!(obj.read(), &new_data[..]);

        drop(obj);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_version_conflict() {
        let (region, name) = create_test_region();
        let data = vec![0u8; 50];

        let obj = EntangledObject::create(region.clone(), TypeInfo::Bytes, &data).unwrap();

        // First write succeeds
        obj.write(&[1u8; 50], 1).unwrap();

        // Second write with old version fails
        let result = obj.write(&[2u8; 50], 1);
        assert!(matches!(result, Err(EntangledError::ConcurrencyConflict)));

        drop(obj);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_cas_write() {
        let (region, name) = create_test_region();
        let data = vec![0u8; 20];

        let obj = EntangledObject::create(region.clone(), TypeInfo::Bytes, &data).unwrap();

        // CAS with correct expected value
        let success = obj.cas_write(&data, &[1u8; 20]).unwrap();
        assert!(success);

        // CAS with wrong expected value
        let success = obj.cas_write(&data, &[2u8; 20]).unwrap();
        assert!(!success);

        drop(obj);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_open_existing() {
        let (region, name) = create_test_region();
        let data = b"test data";

        let obj1 = EntangledObject::create(region.clone(), TypeInfo::String, data).unwrap();

        let offset = obj1.offset();

        let obj2 = EntangledObject::open(region.clone(), offset).unwrap();

        assert_eq!(obj2.id(), obj1.id());
        assert_eq!(obj2.type_info(), obj1.type_info());
        assert_eq!(obj2.read(), data);

        drop(obj1);
        drop(obj2);
        drop(region);
        cleanup_region(&name);
    }
}
