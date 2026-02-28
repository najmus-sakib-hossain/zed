//! Buffer protocol implementation
//!
//! Provides zero-copy access to object data for interoperability with
//! C extensions like NumPy.

#![allow(non_upper_case_globals)]

use crate::capi::{PyObject_C, Py_buffer};
use crate::pylist::PyValue;
use std::ffi::{c_int, c_void};
use std::ptr;

/// Buffer flags for different access patterns
pub mod buffer_flags {
    use std::ffi::c_int;

    pub const PyBUF_SIMPLE: c_int = 0;
    pub const PyBUF_WRITABLE: c_int = 0x0001;
    pub const PyBUF_WRITEABLE: c_int = PyBUF_WRITABLE; // Alias
    pub const PyBUF_FORMAT: c_int = 0x0004;
    pub const PyBUF_ND: c_int = 0x0008;
    pub const PyBUF_STRIDES: c_int = 0x0010 | PyBUF_ND;
    pub const PyBUF_C_CONTIGUOUS: c_int = 0x0020 | PyBUF_STRIDES;
    pub const PyBUF_F_CONTIGUOUS: c_int = 0x0040 | PyBUF_STRIDES;
    pub const PyBUF_ANY_CONTIGUOUS: c_int = 0x0080 | PyBUF_STRIDES;
    pub const PyBUF_INDIRECT: c_int = 0x0100 | PyBUF_STRIDES;
    pub const PyBUF_CONTIG: c_int = PyBUF_ND | PyBUF_WRITABLE;
    pub const PyBUF_CONTIG_RO: c_int = PyBUF_ND;
    pub const PyBUF_STRIDED: c_int = PyBUF_STRIDES | PyBUF_WRITABLE;
    pub const PyBUF_STRIDED_RO: c_int = PyBUF_STRIDES;
    pub const PyBUF_RECORDS: c_int = PyBUF_STRIDES | PyBUF_WRITABLE | PyBUF_FORMAT;
    pub const PyBUF_RECORDS_RO: c_int = PyBUF_STRIDES | PyBUF_FORMAT;
    pub const PyBUF_FULL: c_int = PyBUF_INDIRECT | PyBUF_WRITABLE | PyBUF_FORMAT;
    pub const PyBUF_FULL_RO: c_int = PyBUF_INDIRECT | PyBUF_FORMAT;
    pub const PyBUF_READ: c_int = 0x100;
    pub const PyBUF_WRITE: c_int = 0x200;
}

/// Buffer information for a PyValue object
#[derive(Debug)]
pub struct BufferInfo {
    /// Pointer to the data
    pub data: *mut c_void,
    /// Length in bytes
    pub len: isize,
    /// Size of each item
    pub itemsize: isize,
    /// Format string (e.g., "i" for int, "f" for float)
    pub format: String,
    /// Number of dimensions
    pub ndim: c_int,
    /// Shape of each dimension
    pub shape: Vec<isize>,
    /// Strides for each dimension
    pub strides: Vec<isize>,
    /// Whether the buffer is read-only
    pub readonly: bool,
}

impl BufferInfo {
    /// Create a new buffer info for a 1D array
    pub fn new_1d(
        data: *mut c_void,
        len: isize,
        itemsize: isize,
        format: String,
        readonly: bool,
    ) -> Self {
        Self {
            data,
            len,
            itemsize,
            format,
            ndim: 1,
            shape: vec![len / itemsize],
            strides: vec![itemsize],
            readonly,
        }
    }

    /// Create a new buffer info for a multi-dimensional array
    pub fn new_nd(
        data: *mut c_void,
        itemsize: isize,
        format: String,
        shape: Vec<isize>,
        strides: Vec<isize>,
        readonly: bool,
    ) -> Self {
        let len = shape.iter().zip(strides.iter()).map(|(s, st)| s * st).sum();
        Self {
            data,
            len,
            itemsize,
            format,
            ndim: shape.len() as c_int,
            shape,
            strides,
            readonly,
        }
    }

    /// Check if the buffer is C-contiguous
    pub fn is_c_contiguous(&self) -> bool {
        if self.ndim <= 1 {
            return true;
        }

        let mut expected_stride = self.itemsize;
        for i in (0..self.ndim as usize).rev() {
            if self.strides[i] != expected_stride {
                return false;
            }
            expected_stride *= self.shape[i];
        }
        true
    }

    /// Check if the buffer is Fortran-contiguous
    pub fn is_f_contiguous(&self) -> bool {
        if self.ndim <= 1 {
            return true;
        }

        let mut expected_stride = self.itemsize;
        for i in 0..self.ndim as usize {
            if self.strides[i] != expected_stride {
                return false;
            }
            expected_stride *= self.shape[i];
        }
        true
    }
}

/// Trait for objects that support the buffer protocol
pub trait BufferProvider {
    /// Get buffer information for this object
    fn get_buffer(&self, flags: c_int) -> Result<BufferInfo, String>;

    /// Release buffer resources (called when buffer is no longer needed)
    fn release_buffer(&self, _buffer: &BufferInfo) {
        // Default implementation does nothing
    }
}

/// Buffer protocol implementation for PyValue
impl BufferProvider for PyValue {
    fn get_buffer(&self, flags: c_int) -> Result<BufferInfo, String> {
        match self {
            PyValue::Str(s) => {
                // String as bytes buffer
                let bytes = s.as_bytes();
                let data = bytes.as_ptr() as *mut c_void;
                let len = bytes.len() as isize;

                // Check if write access is requested
                if (flags & buffer_flags::PyBUF_WRITABLE) != 0 {
                    return Err("String buffer is read-only".to_string());
                }

                Ok(BufferInfo::new_1d(data, len, 1, "c".to_string(), true))
            }
            PyValue::List(list) => {
                // List as array of PyValue pointers
                let vec = list.to_vec();
                let data = vec.as_ptr() as *mut c_void;
                let len = vec.len() as isize * std::mem::size_of::<PyValue>() as isize;
                let itemsize = std::mem::size_of::<PyValue>() as isize;

                // Lists are always writable if the buffer protocol allows it
                let readonly = (flags & buffer_flags::PyBUF_WRITABLE) == 0;

                Ok(BufferInfo::new_1d(data, len, itemsize, "O".to_string(), readonly))
            }
            _ => {
                Err(format!("Object of type {} does not support buffer protocol", self.type_name()))
            }
        }
    }
}

/// Fill a Py_buffer structure from BufferInfo
pub fn fill_py_buffer(buffer: &mut Py_buffer, info: &BufferInfo, obj: *mut PyObject_C) {
    buffer.buf = info.data;
    buffer.obj = obj;
    buffer.len = info.len;
    buffer.itemsize = info.itemsize;
    buffer.readonly = if info.readonly { 1 } else { 0 };
    buffer.ndim = info.ndim;

    // Format string
    if !info.format.is_empty() {
        // In a real implementation, we'd need to manage the lifetime of this string
        // For now, we'll use a static string or null
        buffer.format = ptr::null_mut();
    } else {
        buffer.format = ptr::null_mut();
    }

    // Shape and strides
    if info.ndim > 0 {
        // In a real implementation, we'd need to allocate and manage these arrays
        // For now, we'll set them to null (which means 1D array)
        buffer.shape = ptr::null_mut();
        buffer.strides = ptr::null_mut();
    } else {
        buffer.shape = ptr::null_mut();
        buffer.strides = ptr::null_mut();
    }

    buffer.suboffsets = ptr::null_mut();
    buffer.internal = ptr::null_mut();
}

/// C-compatible buffer protocol functions
pub mod c_buffer_api {
    use super::*;
    use crate::capi::{getbufferproc, releasebufferproc};

    /// Get buffer implementation for C API
    pub extern "C" fn py_getbuffer(
        obj: *mut PyObject_C,
        buffer: *mut Py_buffer,
        flags: c_int,
    ) -> c_int {
        if obj.is_null() || buffer.is_null() {
            return -1;
        }

        // In a real implementation, we would:
        // 1. Convert the C object to a Rust PyValue
        // 2. Call get_buffer on it
        // 3. Fill the Py_buffer structure
        // 4. Return 0 on success, -1 on failure

        // For now, return failure
        -1
    }

    /// Release buffer implementation for C API
    pub extern "C" fn py_releasebuffer(obj: *mut PyObject_C, buffer: *mut Py_buffer) {
        if obj.is_null() || buffer.is_null() {
            return;
        }

        // In a real implementation, we would:
        // 1. Convert the C object to a Rust PyValue
        // 2. Call release_buffer on it
        // 3. Clean up any allocated memory in the buffer structure

        // For now, do nothing
    }
}

/// Buffer manager for tracking active buffers
pub struct BufferManager {
    /// Active buffers (object pointer -> buffer info)
    active_buffers: std::collections::HashMap<*mut PyObject_C, BufferInfo>,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new() -> Self {
        Self {
            active_buffers: std::collections::HashMap::new(),
        }
    }

    /// Register a buffer
    pub fn register_buffer(&mut self, obj: *mut PyObject_C, info: BufferInfo) {
        self.active_buffers.insert(obj, info);
    }

    /// Unregister a buffer
    pub fn unregister_buffer(&mut self, obj: *mut PyObject_C) -> Option<BufferInfo> {
        self.active_buffers.remove(&obj)
    }

    /// Get active buffer count
    pub fn active_count(&self) -> usize {
        self.active_buffers.len()
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PyList;
    use std::sync::Arc;

    #[test]
    fn test_buffer_info_1d() {
        let data = [1i32, 2, 3, 4];
        let info = BufferInfo::new_1d(
            data.as_ptr() as *mut c_void,
            (data.len() * std::mem::size_of::<i32>()) as isize,
            std::mem::size_of::<i32>() as isize,
            "i".to_string(),
            true,
        );

        assert_eq!(info.ndim, 1);
        assert_eq!(info.shape[0], 4);
        assert_eq!(info.itemsize, 4);
        assert!(info.readonly);
        assert!(info.is_c_contiguous());
    }

    #[test]
    fn test_buffer_info_2d() {
        let shape = vec![2, 3];
        let strides = vec![12, 4]; // 3 * 4 bytes, 4 bytes
        let info = BufferInfo::new_nd(ptr::null_mut(), 4, "i".to_string(), shape, strides, false);

        assert_eq!(info.ndim, 2);
        assert_eq!(info.shape, vec![2, 3]);
        assert!(!info.readonly);
        assert!(info.is_c_contiguous());
    }

    #[test]
    fn test_string_buffer() {
        let s = Arc::from("hello");
        let value = PyValue::Str(s);

        let buffer = value.get_buffer(buffer_flags::PyBUF_SIMPLE);
        assert!(buffer.is_ok());

        let info = buffer.unwrap();
        assert_eq!(info.len, 5);
        assert_eq!(info.itemsize, 1);
        assert!(info.readonly);
    }

    #[test]
    fn test_string_buffer_writable_fails() {
        let s = Arc::from("hello");
        let value = PyValue::Str(s);

        let buffer = value.get_buffer(buffer_flags::PyBUF_WRITABLE);
        assert!(buffer.is_err());
    }

    #[test]
    fn test_list_buffer() {
        let list = Arc::new(PyList::new());
        list.append(PyValue::Int(1));
        list.append(PyValue::Int(2));
        let value = PyValue::List(list);

        let buffer = value.get_buffer(buffer_flags::PyBUF_SIMPLE);
        assert!(buffer.is_ok());

        let info = buffer.unwrap();
        assert_eq!(info.ndim, 1);
        assert_eq!(info.shape[0], 2);
    }

    #[test]
    fn test_buffer_manager() {
        let mut manager = BufferManager::new();
        assert_eq!(manager.active_count(), 0);

        let info = BufferInfo::new_1d(ptr::null_mut(), 10, 1, "c".to_string(), true);
        let obj_ptr = ptr::null_mut();

        manager.register_buffer(obj_ptr, info);
        assert_eq!(manager.active_count(), 1);

        let removed = manager.unregister_buffer(obj_ptr);
        assert!(removed.is_some());
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_unsupported_buffer() {
        let value = PyValue::Int(42);
        let buffer = value.get_buffer(buffer_flags::PyBUF_SIMPLE);
        assert!(buffer.is_err());
    }
}
