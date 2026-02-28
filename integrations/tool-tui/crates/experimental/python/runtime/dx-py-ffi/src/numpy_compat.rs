//! NumPy C API Compatibility Layer
//!
//! This module provides CPython-compatible NumPy C API functions for loading
//! and interacting with NumPy arrays from C extensions.
//!
//! ## Implemented Functions
//!
//! - `PyArray_DATA` - Get pointer to array data
//! - `PyArray_DIMS` - Get pointer to dimensions array
//! - `PyArray_STRIDES` - Get pointer to strides array
//! - `PyArray_NDIM` - Get number of dimensions
//! - `PyArray_SIZE` - Get total number of elements
//! - `PyArray_ITEMSIZE` - Get size of each element
//! - `PyArray_NBYTES` - Get total size in bytes
//! - `PyArray_TYPE` - Get type number
//! - `PyArray_DTYPE` - Get dtype object
//! - `PyArray_FLAGS` - Get array flags
//! - `PyArray_SimpleNew` - Create a new array
//! - `PyArray_SimpleNewFromData` - Create array from existing data
//! - `PyArray_ZEROS` - Create a zero-filled array
//! - `PyArray_EMPTY` - Create an uninitialized array

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};
use std::ptr;

use crate::cpython_compat::PyObject;
use crate::teleport::{ArrayFlags, DType, TeleportedArray};

// =============================================================================
// NumPy Type Numbers (NPY_TYPES)
// =============================================================================

/// NumPy type numbers
pub mod npy_types {
    use std::ffi::c_int;

    pub const NPY_BOOL: c_int = 0;
    pub const NPY_BYTE: c_int = 1;
    pub const NPY_UBYTE: c_int = 2;
    pub const NPY_SHORT: c_int = 3;
    pub const NPY_USHORT: c_int = 4;
    pub const NPY_INT: c_int = 5;
    pub const NPY_UINT: c_int = 6;
    pub const NPY_LONG: c_int = 7;
    pub const NPY_ULONG: c_int = 8;
    pub const NPY_LONGLONG: c_int = 9;
    pub const NPY_ULONGLONG: c_int = 10;
    pub const NPY_FLOAT: c_int = 11;
    pub const NPY_DOUBLE: c_int = 12;
    pub const NPY_LONGDOUBLE: c_int = 13;
    pub const NPY_CFLOAT: c_int = 14;
    pub const NPY_CDOUBLE: c_int = 15;
    pub const NPY_CLONGDOUBLE: c_int = 16;
    pub const NPY_OBJECT: c_int = 17;
    pub const NPY_STRING: c_int = 18;
    pub const NPY_UNICODE: c_int = 19;
    pub const NPY_VOID: c_int = 20;
    pub const NPY_DATETIME: c_int = 21;
    pub const NPY_TIMEDELTA: c_int = 22;
    pub const NPY_HALF: c_int = 23;

    // Platform-specific aliases
    pub const NPY_INT8: c_int = NPY_BYTE;
    pub const NPY_INT16: c_int = NPY_SHORT;
    pub const NPY_INT32: c_int = NPY_INT;
    pub const NPY_INT64: c_int = NPY_LONGLONG;
    pub const NPY_UINT8: c_int = NPY_UBYTE;
    pub const NPY_UINT16: c_int = NPY_USHORT;
    pub const NPY_UINT32: c_int = NPY_UINT;
    pub const NPY_UINT64: c_int = NPY_ULONGLONG;
    pub const NPY_FLOAT32: c_int = NPY_FLOAT;
    pub const NPY_FLOAT64: c_int = NPY_DOUBLE;
    pub const NPY_COMPLEX64: c_int = NPY_CFLOAT;
    pub const NPY_COMPLEX128: c_int = NPY_CDOUBLE;
}

/// Convert DType to NumPy type number
pub fn dtype_to_typenum(dtype: DType) -> c_int {
    use npy_types::*;
    match dtype {
        DType::Bool => NPY_BOOL,
        DType::Int8 => NPY_INT8,
        DType::Int16 => NPY_INT16,
        DType::Int32 => NPY_INT32,
        DType::Int64 => NPY_INT64,
        DType::UInt8 => NPY_UINT8,
        DType::UInt16 => NPY_UINT16,
        DType::UInt32 => NPY_UINT32,
        DType::UInt64 => NPY_UINT64,
        DType::Float16 => NPY_HALF,
        DType::Float32 => NPY_FLOAT32,
        DType::Float64 => NPY_FLOAT64,
        DType::Complex64 => NPY_COMPLEX64,
        DType::Complex128 => NPY_COMPLEX128,
        DType::String(_) => NPY_STRING,
        DType::Unicode(_) => NPY_UNICODE,
        DType::DateTime64 => NPY_DATETIME,
        DType::TimeDelta64 => NPY_TIMEDELTA,
        DType::Object => NPY_OBJECT,
    }
}

/// Convert NumPy type number to DType
pub fn typenum_to_dtype(typenum: c_int) -> Option<DType> {
    use npy_types::*;
    match typenum {
        NPY_BOOL => Some(DType::Bool),
        NPY_BYTE => Some(DType::Int8),
        NPY_UBYTE => Some(DType::UInt8),
        NPY_SHORT => Some(DType::Int16),
        NPY_USHORT => Some(DType::UInt16),
        NPY_INT => Some(DType::Int32),
        NPY_UINT => Some(DType::UInt32),
        NPY_LONGLONG => Some(DType::Int64),
        NPY_ULONGLONG => Some(DType::UInt64),
        NPY_HALF => Some(DType::Float16),
        NPY_FLOAT => Some(DType::Float32),
        NPY_DOUBLE => Some(DType::Float64),
        NPY_CFLOAT => Some(DType::Complex64),
        NPY_CDOUBLE => Some(DType::Complex128),
        NPY_STRING => Some(DType::String(0)),
        NPY_UNICODE => Some(DType::Unicode(0)),
        NPY_DATETIME => Some(DType::DateTime64),
        NPY_TIMEDELTA => Some(DType::TimeDelta64),
        NPY_OBJECT => Some(DType::Object),
        _ => None,
    }
}

// =============================================================================
// NumPy Array Flags
// =============================================================================

/// NumPy array flags
pub mod npy_flags {
    use std::ffi::c_int;

    pub const NPY_ARRAY_C_CONTIGUOUS: c_int = 0x0001;
    pub const NPY_ARRAY_F_CONTIGUOUS: c_int = 0x0002;
    pub const NPY_ARRAY_OWNDATA: c_int = 0x0004;
    pub const NPY_ARRAY_FORCECAST: c_int = 0x0010;
    pub const NPY_ARRAY_ENSURECOPY: c_int = 0x0020;
    pub const NPY_ARRAY_ENSUREARRAY: c_int = 0x0040;
    pub const NPY_ARRAY_ELEMENTSTRIDES: c_int = 0x0080;
    pub const NPY_ARRAY_ALIGNED: c_int = 0x0100;
    pub const NPY_ARRAY_NOTSWAPPED: c_int = 0x0200;
    pub const NPY_ARRAY_WRITEABLE: c_int = 0x0400;
    pub const NPY_ARRAY_UPDATEIFCOPY: c_int = 0x1000;
    pub const NPY_ARRAY_WRITEBACKIFCOPY: c_int = 0x2000;

    // Convenience combinations
    pub const NPY_ARRAY_BEHAVED: c_int = NPY_ARRAY_ALIGNED | NPY_ARRAY_WRITEABLE;
    pub const NPY_ARRAY_CARRAY: c_int = NPY_ARRAY_C_CONTIGUOUS | NPY_ARRAY_BEHAVED;
    pub const NPY_ARRAY_FARRAY: c_int = NPY_ARRAY_F_CONTIGUOUS | NPY_ARRAY_BEHAVED;
    pub const NPY_ARRAY_DEFAULT: c_int = NPY_ARRAY_CARRAY;
}

/// Convert ArrayFlags to NumPy flags
pub fn array_flags_to_npy(flags: &ArrayFlags) -> c_int {
    use npy_flags::*;
    let mut npy = 0;
    if flags.c_contiguous {
        npy |= NPY_ARRAY_C_CONTIGUOUS;
    }
    if flags.f_contiguous {
        npy |= NPY_ARRAY_F_CONTIGUOUS;
    }
    if flags.owndata {
        npy |= NPY_ARRAY_OWNDATA;
    }
    if flags.writeable {
        npy |= NPY_ARRAY_WRITEABLE;
    }
    if flags.aligned {
        npy |= NPY_ARRAY_ALIGNED;
    }
    npy
}

/// Convert NumPy flags to ArrayFlags
pub fn npy_to_array_flags(npy: c_int) -> ArrayFlags {
    use npy_flags::*;
    ArrayFlags {
        c_contiguous: (npy & NPY_ARRAY_C_CONTIGUOUS) != 0,
        f_contiguous: (npy & NPY_ARRAY_F_CONTIGUOUS) != 0,
        owndata: (npy & NPY_ARRAY_OWNDATA) != 0,
        writeable: (npy & NPY_ARRAY_WRITEABLE) != 0,
        aligned: (npy & NPY_ARRAY_ALIGNED) != 0,
        updateifcopy: (npy & NPY_ARRAY_UPDATEIFCOPY) != 0,
    }
}

// =============================================================================
// PyArrayObject - NumPy Array Structure
// =============================================================================

/// PyArrayObject structure compatible with NumPy's ndarray
///
/// This is a simplified version that wraps our TeleportedArray
#[repr(C)]
pub struct PyArrayObject {
    /// PyObject header
    pub ob_base: PyObject,
    /// Pointer to data buffer
    pub data: *mut u8,
    /// Number of dimensions
    pub nd: c_int,
    /// Pointer to dimensions array
    pub dimensions: *mut isize,
    /// Pointer to strides array
    pub strides: *mut isize,
    /// Base object (for views)
    pub base: *mut PyObject,
    /// Dtype descriptor
    pub descr: *mut PyObject,
    /// Array flags
    pub flags: c_int,
    /// Weak reference list
    pub weakreflist: *mut PyObject,
    /// Internal: our TeleportedArray
    _internal: Option<Box<TeleportedArray>>,
    /// Internal: owned dimensions
    _dimensions: Vec<isize>,
    /// Internal: owned strides
    _strides: Vec<isize>,
}

impl PyArrayObject {
    /// Create a new PyArrayObject from a TeleportedArray
    pub fn from_teleported(array: TeleportedArray) -> Box<Self> {
        let dimensions: Vec<isize> = array.shape().iter().map(|&s| s as isize).collect();
        let strides: Vec<isize> = array.strides().to_vec();
        let flags = array_flags_to_npy(&array.flags());
        let data = array.data_ptr() as *mut u8;
        let nd = array.ndim() as c_int;

        let mut obj = Box::new(Self {
            ob_base: PyObject::new(ptr::null_mut()),
            data,
            nd,
            dimensions: ptr::null_mut(),
            strides: ptr::null_mut(),
            base: ptr::null_mut(),
            descr: ptr::null_mut(),
            flags,
            weakreflist: ptr::null_mut(),
            _internal: Some(Box::new(array)),
            _dimensions: dimensions,
            _strides: strides,
        });

        // Set pointers to internal vectors
        obj.dimensions = obj._dimensions.as_mut_ptr();
        obj.strides = obj._strides.as_mut_ptr();

        obj
    }

    /// Get the internal TeleportedArray
    pub fn as_teleported(&self) -> Option<&TeleportedArray> {
        self._internal.as_ref().map(|b| b.as_ref())
    }

    /// Get the internal TeleportedArray mutably
    pub fn as_teleported_mut(&mut self) -> Option<&mut TeleportedArray> {
        self._internal.as_mut().map(|b| b.as_mut())
    }
}

// Safety: PyArrayObject is Send because TeleportedArray is Send
unsafe impl Send for PyArrayObject {}
unsafe impl Sync for PyArrayObject {}

// =============================================================================
// PyArray_* Accessor Functions
// =============================================================================

/// Get pointer to array data
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_DATA(arr: *mut PyArrayObject) -> *mut c_void {
    if arr.is_null() {
        return ptr::null_mut();
    }
    (*arr).data as *mut c_void
}

/// Get pointer to dimensions array
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_DIMS(arr: *mut PyArrayObject) -> *mut isize {
    if arr.is_null() {
        return ptr::null_mut();
    }
    (*arr).dimensions
}

/// Get pointer to strides array
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_STRIDES(arr: *mut PyArrayObject) -> *mut isize {
    if arr.is_null() {
        return ptr::null_mut();
    }
    (*arr).strides
}

/// Get number of dimensions
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_NDIM(arr: *mut PyArrayObject) -> c_int {
    if arr.is_null() {
        return 0;
    }
    (*arr).nd
}

/// Get dimension at index
///
/// # Safety
/// The array pointer must be valid and index must be in bounds.
#[no_mangle]
pub unsafe extern "C" fn PyArray_DIM(arr: *mut PyArrayObject, index: c_int) -> isize {
    if arr.is_null() || index < 0 || index >= (*arr).nd {
        return 0;
    }
    *(*arr).dimensions.add(index as usize)
}

/// Get stride at index
///
/// # Safety
/// The array pointer must be valid and index must be in bounds.
#[no_mangle]
pub unsafe extern "C" fn PyArray_STRIDE(arr: *mut PyArrayObject, index: c_int) -> isize {
    if arr.is_null() || index < 0 || index >= (*arr).nd {
        return 0;
    }
    *(*arr).strides.add(index as usize)
}

/// Get total number of elements
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_SIZE(arr: *mut PyArrayObject) -> isize {
    if arr.is_null() {
        return 0;
    }
    let mut size: isize = 1;
    for i in 0..(*arr).nd as usize {
        size *= *(*arr).dimensions.add(i);
    }
    size
}

/// Get size of each element in bytes
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_ITEMSIZE(arr: *mut PyArrayObject) -> isize {
    if arr.is_null() {
        return 0;
    }
    if let Some(teleported) = (*arr).as_teleported() {
        teleported.dtype().size() as isize
    } else {
        0
    }
}

/// Get total size in bytes
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_NBYTES(arr: *mut PyArrayObject) -> isize {
    PyArray_SIZE(arr) * PyArray_ITEMSIZE(arr)
}

/// Get array flags
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_FLAGS(arr: *mut PyArrayObject) -> c_int {
    if arr.is_null() {
        return 0;
    }
    (*arr).flags
}

/// Check if array is C-contiguous
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_ISCONTIGUOUS(arr: *mut PyArrayObject) -> c_int {
    if arr.is_null() {
        return 0;
    }
    if ((*arr).flags & npy_flags::NPY_ARRAY_C_CONTIGUOUS) != 0 {
        1
    } else {
        0
    }
}

/// Check if array is Fortran-contiguous
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_ISFORTRAN(arr: *mut PyArrayObject) -> c_int {
    if arr.is_null() {
        return 0;
    }
    if ((*arr).flags & npy_flags::NPY_ARRAY_F_CONTIGUOUS) != 0 {
        1
    } else {
        0
    }
}

/// Check if array is writeable
///
/// # Safety
/// The array pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn PyArray_ISWRITEABLE(arr: *mut PyArrayObject) -> c_int {
    if arr.is_null() {
        return 0;
    }
    if ((*arr).flags & npy_flags::NPY_ARRAY_WRITEABLE) != 0 {
        1
    } else {
        0
    }
}

// =============================================================================
// PyArray_* Creation Functions
// =============================================================================

/// Create a new array with given shape and type
///
/// # Safety
/// The dims pointer must be valid for nd elements.
#[no_mangle]
pub unsafe extern "C" fn PyArray_SimpleNew(
    nd: c_int,
    dims: *const isize,
    typenum: c_int,
) -> *mut PyArrayObject {
    if nd < 0 || dims.is_null() {
        return ptr::null_mut();
    }

    let dtype = match typenum_to_dtype(typenum) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    let shape: Vec<usize> = (0..nd as usize).map(|i| *dims.add(i) as usize).collect();

    let array = TeleportedArray::zeros(shape, dtype);
    let boxed = PyArrayObject::from_teleported(array);
    Box::into_raw(boxed)
}

/// Create a new array from existing data (no copy)
///
/// # Safety
/// The data pointer must be valid and remain valid for the array's lifetime.
/// The dims pointer must be valid for nd elements.
#[no_mangle]
pub unsafe extern "C" fn PyArray_SimpleNewFromData(
    nd: c_int,
    dims: *const isize,
    typenum: c_int,
    data: *mut c_void,
) -> *mut PyArrayObject {
    if nd < 0 || dims.is_null() || data.is_null() {
        return ptr::null_mut();
    }

    let dtype = match typenum_to_dtype(typenum) {
        Some(d) => d,
        None => return ptr::null_mut(),
    };

    let shape: Vec<usize> = (0..nd as usize).map(|i| *dims.add(i) as usize).collect();

    // Calculate C-contiguous strides
    let mut strides = vec![0isize; nd as usize];
    let mut stride = dtype.size() as isize;
    for i in (0..nd as usize).rev() {
        strides[i] = stride;
        stride *= shape[i] as isize;
    }

    let array = TeleportedArray::new(
        data as *mut u8,
        shape,
        strides,
        dtype,
        false, // writeable
    );

    let boxed = PyArrayObject::from_teleported(array);
    Box::into_raw(boxed)
}

/// Create a zero-filled array
///
/// # Safety
/// The dims pointer must be valid for nd elements.
#[no_mangle]
pub unsafe extern "C" fn PyArray_ZEROS(
    nd: c_int,
    dims: *const isize,
    typenum: c_int,
    _fortran: c_int,
) -> *mut PyArrayObject {
    // For now, always create C-contiguous arrays
    PyArray_SimpleNew(nd, dims, typenum)
}

/// Create an empty (uninitialized) array
///
/// # Safety
/// The dims pointer must be valid for nd elements.
#[no_mangle]
pub unsafe extern "C" fn PyArray_EMPTY(
    nd: c_int,
    dims: *const isize,
    typenum: c_int,
    _fortran: c_int,
) -> *mut PyArrayObject {
    // For safety, we still zero-initialize
    PyArray_SimpleNew(nd, dims, typenum)
}

/// Free a PyArrayObject
///
/// # Safety
/// The array pointer must have been created by one of the PyArray_* functions.
#[no_mangle]
pub unsafe extern "C" fn PyArray_Free(arr: *mut PyArrayObject) {
    if !arr.is_null() {
        let _ = Box::from_raw(arr);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtype_to_typenum() {
        assert_eq!(dtype_to_typenum(DType::Float64), npy_types::NPY_DOUBLE);
        assert_eq!(dtype_to_typenum(DType::Int32), npy_types::NPY_INT);
        assert_eq!(dtype_to_typenum(DType::Bool), npy_types::NPY_BOOL);
    }

    #[test]
    fn test_typenum_to_dtype() {
        assert_eq!(typenum_to_dtype(npy_types::NPY_DOUBLE), Some(DType::Float64));
        assert_eq!(typenum_to_dtype(npy_types::NPY_INT), Some(DType::Int32));
        assert_eq!(typenum_to_dtype(npy_types::NPY_BOOL), Some(DType::Bool));
        assert_eq!(typenum_to_dtype(999), None);
    }

    #[test]
    fn test_array_flags_conversion() {
        let flags = ArrayFlags {
            c_contiguous: true,
            f_contiguous: false,
            owndata: true,
            writeable: true,
            aligned: true,
            updateifcopy: false,
        };

        let npy = array_flags_to_npy(&flags);
        let restored = npy_to_array_flags(npy);

        assert_eq!(flags.c_contiguous, restored.c_contiguous);
        assert_eq!(flags.f_contiguous, restored.f_contiguous);
        assert_eq!(flags.owndata, restored.owndata);
        assert_eq!(flags.writeable, restored.writeable);
    }

    #[test]
    fn test_pyarray_simple_new() {
        unsafe {
            let dims: [isize; 2] = [3, 4];
            let arr = PyArray_SimpleNew(2, dims.as_ptr(), npy_types::NPY_DOUBLE);

            assert!(!arr.is_null());
            assert_eq!(PyArray_NDIM(arr), 2);
            assert_eq!(PyArray_DIM(arr, 0), 3);
            assert_eq!(PyArray_DIM(arr, 1), 4);
            assert_eq!(PyArray_SIZE(arr), 12);
            assert_eq!(PyArray_ITEMSIZE(arr), 8);
            assert_eq!(PyArray_NBYTES(arr), 96);
            assert_ne!(PyArray_ISCONTIGUOUS(arr), 0);

            PyArray_Free(arr);
        }
    }

    #[test]
    fn test_pyarray_from_data() {
        unsafe {
            let mut data: [f64; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
            let dims: [isize; 2] = [2, 3];

            let arr = PyArray_SimpleNewFromData(
                2,
                dims.as_ptr(),
                npy_types::NPY_DOUBLE,
                data.as_mut_ptr() as *mut c_void,
            );

            assert!(!arr.is_null());
            assert_eq!(PyArray_NDIM(arr), 2);
            assert_eq!(PyArray_SIZE(arr), 6);

            // Verify data pointer
            let data_ptr = PyArray_DATA(arr) as *const f64;
            assert_eq!(*data_ptr, 1.0);
            assert_eq!(*data_ptr.add(1), 2.0);

            PyArray_Free(arr);
        }
    }

    #[test]
    fn test_pyarray_strides() {
        unsafe {
            let dims: [isize; 3] = [2, 3, 4];
            let arr = PyArray_SimpleNew(3, dims.as_ptr(), npy_types::NPY_FLOAT);

            assert!(!arr.is_null());

            // C-contiguous strides for float32 (4 bytes)
            // Shape [2, 3, 4] -> strides [48, 16, 4]
            assert_eq!(PyArray_STRIDE(arr, 2), 4); // innermost
            assert_eq!(PyArray_STRIDE(arr, 1), 16); // 4 * 4
            assert_eq!(PyArray_STRIDE(arr, 0), 48); // 4 * 4 * 3

            PyArray_Free(arr);
        }
    }

    #[test]
    fn test_pyarray_null_safety() {
        unsafe {
            // All functions should handle null gracefully
            assert!(PyArray_DATA(ptr::null_mut()).is_null());
            assert!(PyArray_DIMS(ptr::null_mut()).is_null());
            assert!(PyArray_STRIDES(ptr::null_mut()).is_null());
            assert_eq!(PyArray_NDIM(ptr::null_mut()), 0);
            assert_eq!(PyArray_SIZE(ptr::null_mut()), 0);
            assert_eq!(PyArray_ITEMSIZE(ptr::null_mut()), 0);
            assert_eq!(PyArray_FLAGS(ptr::null_mut()), 0);
        }
    }
}
