//! Entangled arrays for NumPy-like shared data

use crate::object::TypeInfo;
use crate::{EntangledError, EntangledHandle, EntangledObject, SharedMemoryRegion};
use std::sync::Arc;

/// Entangled array for sharing NumPy-like data across processes
pub struct EntangledArray {
    /// Underlying entangled object
    object: EntangledObject,
    /// Array shape
    shape: Vec<u64>,
    /// Element count
    len: usize,
}

/// Array header stored before data
#[repr(C)]
struct ArrayHeader {
    /// Number of dimensions
    ndim: u8,
    /// Element type (0 = f64, 1 = i64)
    dtype: u8,
    /// Reserved
    _reserved: [u8; 6],
    /// Shape (up to 8 dimensions)
    shape: [u64; 8],
}

const ARRAY_HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

impl EntangledArray {
    /// Create a new entangled array from f64 data
    pub fn from_f64(
        region: Arc<SharedMemoryRegion>,
        data: &[f64],
        shape: &[u64],
    ) -> Result<Self, EntangledError> {
        let len: u64 = shape.iter().product();
        if len as usize != data.len() {
            return Err(EntangledError::TypeMismatch);
        }

        // Prepare header + data
        let mut bytes = Vec::with_capacity(ARRAY_HEADER_SIZE + data.len() * 8);

        // Header
        bytes.push(shape.len() as u8); // ndim
        bytes.push(0); // dtype = f64
        bytes.extend_from_slice(&[0u8; 6]); // reserved

        // Shape (padded to 8 dimensions)
        for i in 0..8 {
            let dim = shape.get(i).copied().unwrap_or(0);
            bytes.extend_from_slice(&dim.to_le_bytes());
        }

        // Data
        for &val in data {
            bytes.extend_from_slice(&val.to_le_bytes());
        }

        let object = EntangledObject::create(region, TypeInfo::FloatArray, &bytes)?;

        Ok(Self {
            object,
            shape: shape.to_vec(),
            len: data.len(),
        })
    }

    /// Create a new entangled array from i64 data
    pub fn from_i64(
        region: Arc<SharedMemoryRegion>,
        data: &[i64],
        shape: &[u64],
    ) -> Result<Self, EntangledError> {
        let len: u64 = shape.iter().product();
        if len as usize != data.len() {
            return Err(EntangledError::TypeMismatch);
        }

        let mut bytes = Vec::with_capacity(ARRAY_HEADER_SIZE + data.len() * 8);

        // Header
        bytes.push(shape.len() as u8);
        bytes.push(1); // dtype = i64
        bytes.extend_from_slice(&[0u8; 6]);

        for i in 0..8 {
            let dim = shape.get(i).copied().unwrap_or(0);
            bytes.extend_from_slice(&dim.to_le_bytes());
        }

        for &val in data {
            bytes.extend_from_slice(&val.to_le_bytes());
        }

        let object = EntangledObject::create(region, TypeInfo::IntArray, &bytes)?;

        Ok(Self {
            object,
            shape: shape.to_vec(),
            len: data.len(),
        })
    }

    /// Open an existing entangled array
    pub fn open(region: Arc<SharedMemoryRegion>, offset: u64) -> Result<Self, EntangledError> {
        let object = EntangledObject::open(region, offset)?;

        let data = object.read();
        if data.len() < ARRAY_HEADER_SIZE {
            return Err(EntangledError::TypeMismatch);
        }

        let ndim = data[0] as usize;
        let mut shape = Vec::with_capacity(ndim);

        for i in 0..ndim {
            let offset = 8 + i * 8;
            let dim = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            shape.push(dim);
        }

        let len: u64 = shape.iter().product();

        Ok(Self {
            object,
            shape,
            len: len as usize,
        })
    }

    /// Get the array shape
    pub fn shape(&self) -> &[u64] {
        &self.shape
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Check if this is a float array
    pub fn is_float(&self) -> bool {
        let data = self.object.read();
        data.len() > 1 && data[1] == 0
    }

    /// Check if this is an int array
    pub fn is_int(&self) -> bool {
        let data = self.object.read();
        data.len() > 1 && data[1] == 1
    }

    /// Read as f64 slice
    pub fn as_f64_slice(&self) -> Option<&[f64]> {
        if !self.is_float() {
            return None;
        }

        let data = self.object.read();
        let data_start = ARRAY_HEADER_SIZE;
        let data_bytes = &data[data_start..];

        // Safety: we verified the type and the data is properly aligned
        let ptr = data_bytes.as_ptr() as *const f64;
        Some(unsafe { std::slice::from_raw_parts(ptr, self.len) })
    }

    /// Read as i64 slice
    pub fn as_i64_slice(&self) -> Option<&[i64]> {
        if !self.is_int() {
            return None;
        }

        let data = self.object.read();
        let data_start = ARRAY_HEADER_SIZE;
        let data_bytes = &data[data_start..];

        let ptr = data_bytes.as_ptr() as *const i64;
        Some(unsafe { std::slice::from_raw_parts(ptr, self.len) })
    }

    /// Add a scalar to all elements (f64 array)
    pub fn add_scalar_f64(&self, scalar: f64) -> Result<(), EntangledError> {
        if !self.is_float() {
            return Err(EntangledError::TypeMismatch);
        }

        let version = self.object.version();
        let data = self.object.read().to_vec();

        let mut new_data = data.clone();
        let data_start = ARRAY_HEADER_SIZE;

        // SIMD-friendly loop
        for i in 0..self.len {
            let offset = data_start + i * 8;
            let val = f64::from_le_bytes(new_data[offset..offset + 8].try_into().unwrap());
            let new_val = val + scalar;
            new_data[offset..offset + 8].copy_from_slice(&new_val.to_le_bytes());
        }

        self.object.write(&new_data, version)?;
        Ok(())
    }

    /// Multiply all elements by a scalar (f64 array)
    pub fn mul_scalar_f64(&self, scalar: f64) -> Result<(), EntangledError> {
        if !self.is_float() {
            return Err(EntangledError::TypeMismatch);
        }

        let version = self.object.version();
        let data = self.object.read().to_vec();

        let mut new_data = data.clone();
        let data_start = ARRAY_HEADER_SIZE;

        for i in 0..self.len {
            let offset = data_start + i * 8;
            let val = f64::from_le_bytes(new_data[offset..offset + 8].try_into().unwrap());
            let new_val = val * scalar;
            new_data[offset..offset + 8].copy_from_slice(&new_val.to_le_bytes());
        }

        self.object.write(&new_data, version)?;
        Ok(())
    }

    /// Get a handle for cross-process transfer
    pub fn get_handle(&self) -> EntangledHandle {
        EntangledHandle::from_object(&self.object)
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        self.object.version()
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
        let name = format!("test_array_{}_{}", std::process::id(), counter);
        let region = Arc::new(SharedMemoryRegion::create(&name, 1024 * 1024).unwrap());
        (region, name)
    }

    fn cleanup_region(name: &str) {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("dx-py-entangled").join(format!("{}.shm", name));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_create_f64_array() {
        let (region, name) = create_test_region();
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let shape = vec![2, 3];

        let arr = EntangledArray::from_f64(region.clone(), &data, &shape).unwrap();

        assert_eq!(arr.shape(), &[2, 3]);
        assert_eq!(arr.len(), 6);
        assert_eq!(arr.ndim(), 2);
        assert!(arr.is_float());

        let slice = arr.as_f64_slice().unwrap();
        assert_eq!(slice, &data[..]);

        drop(arr);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_create_i64_array() {
        let (region, name) = create_test_region();
        let data = vec![1i64, 2, 3, 4];
        let shape = vec![4];

        let arr = EntangledArray::from_i64(region.clone(), &data, &shape).unwrap();

        assert_eq!(arr.shape(), &[4]);
        assert_eq!(arr.len(), 4);
        assert!(arr.is_int());

        let slice = arr.as_i64_slice().unwrap();
        assert_eq!(slice, &data[..]);

        drop(arr);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_add_scalar() {
        let (region, name) = create_test_region();
        let data = vec![1.0, 2.0, 3.0];

        let arr = EntangledArray::from_f64(region.clone(), &data, &[3]).unwrap();
        arr.add_scalar_f64(10.0).unwrap();

        let slice = arr.as_f64_slice().unwrap();
        assert_eq!(slice, &[11.0, 12.0, 13.0]);

        drop(arr);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_mul_scalar() {
        let (region, name) = create_test_region();
        let data = vec![2.0, 3.0, 4.0];

        let arr = EntangledArray::from_f64(region.clone(), &data, &[3]).unwrap();
        arr.mul_scalar_f64(2.0).unwrap();

        let slice = arr.as_f64_slice().unwrap();
        assert_eq!(slice, &[4.0, 6.0, 8.0]);

        drop(arr);
        drop(region);
        cleanup_region(&name);
    }

    #[test]
    fn test_handle_roundtrip() {
        let (region, name) = create_test_region();
        let data = vec![1.0, 2.0, 3.0, 4.0];

        let arr = EntangledArray::from_f64(region.clone(), &data, &[2, 2]).unwrap();
        let handle = arr.get_handle();

        let arr2 = EntangledArray::open(region.clone(), handle.offset).unwrap();

        assert_eq!(arr2.shape(), arr.shape());
        assert_eq!(arr2.as_f64_slice().unwrap(), arr.as_f64_slice().unwrap());

        drop(arr);
        drop(arr2);
        drop(region);
        cleanup_region(&name);
    }
}
