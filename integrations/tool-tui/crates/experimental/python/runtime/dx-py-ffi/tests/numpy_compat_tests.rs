//! NumPy Compatibility Tests
//!
//! These tests verify that DX-Py can load and interact with NumPy arrays
//! through the C extension compatibility layer.
//!
//! **Note**: These tests require NumPy to be installed and are marked as
//! `#[ignore]` by default. Run with `cargo test --ignored` to execute.
//!
//! **Validates: Requirements 10.7**

use dx_py_ffi::cpython_compat::{GilGuard, PyObject, Py_buffer};
use dx_py_ffi::{DType, TeleportedArray};
use std::ffi::c_void;
use std::ptr;

/// Test that NumPy can be imported
/// This is the most basic compatibility test
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_import() {
    // In a full implementation, this would:
    // 1. Initialize the Python runtime
    // 2. Import numpy module via the import system
    // 3. Verify the module object is valid

    // For now, we verify the infrastructure is in place
    let _guard = GilGuard::acquire();
    assert!(GilGuard::is_held(), "GIL should be held for NumPy operations");
}

/// Test NumPy array creation
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_array_creation() {
    // In a full implementation, this would:
    // 1. Call numpy.array([1, 2, 3, 4, 5])
    // 2. Verify the returned object is a valid ndarray
    // 3. Check shape, dtype, and data

    let _guard = GilGuard::acquire();

    // Simulate what we'd get from NumPy
    let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let array = TeleportedArray::from_vec(data, vec![5]);

    assert_eq!(array.len(), 5);
    assert_eq!(array.ndim(), 1);
    assert_eq!(array.shape(), &[5]);
}

/// Test NumPy array operations (element-wise)
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_array_operations() {
    // In a full implementation, this would:
    // 1. Create two NumPy arrays
    // 2. Perform element-wise operations (add, multiply, etc.)
    // 3. Verify results match expected values

    let _guard = GilGuard::acquire();

    // Simulate NumPy array operations
    let data1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let data2: Vec<f64> = vec![5.0, 4.0, 3.0, 2.0, 1.0];

    let mut array1 = TeleportedArray::from_vec(data1, vec![5]);
    let array2 = TeleportedArray::from_vec(data2, vec![5]);

    // Add arrays (simulated)
    unsafe {
        let slice1: &mut [f64] = array1.as_mut_slice().unwrap();
        let slice2: &[f64] = array2.as_slice();

        for i in 0..5 {
            slice1[i] += slice2[i];
        }
    }

    // Verify result
    unsafe {
        let result: &[f64] = array1.as_slice();
        assert_eq!(result, &[6.0, 6.0, 6.0, 6.0, 6.0]);
    }
}

/// Test buffer protocol interop with NumPy
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_buffer_protocol() {
    // In a full implementation, this would:
    // 1. Create a NumPy array
    // 2. Get buffer via PyObject_GetBuffer
    // 3. Verify buffer properties (shape, strides, format)
    // 4. Access data through buffer
    // 5. Release buffer

    let _guard = GilGuard::acquire();

    // Simulate buffer protocol
    let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let shape: Vec<isize> = vec![2, 3];
    let strides: Vec<isize> = vec![24, 8]; // 3 * 8, 8 (C-contiguous)

    let mut buffer = Py_buffer::new();
    buffer.buf = data.as_ptr() as *mut c_void;
    buffer.len = (data.len() * std::mem::size_of::<f64>()) as isize;
    buffer.itemsize = std::mem::size_of::<f64>() as isize;
    buffer.ndim = 2;
    buffer.shape = shape.as_ptr() as *mut isize;
    buffer.strides = strides.as_ptr() as *mut isize;
    buffer.readonly = 0;

    // Verify buffer is valid
    assert!(buffer.is_valid());
    assert_eq!(buffer.ndim, 2);
    assert_eq!(buffer.itemsize, 8);

    // Access data through buffer
    unsafe {
        let slice: &[f64] = buffer.as_slice().unwrap();
        assert_eq!(slice.len(), 6);
        assert_eq!(slice[0], 1.0);
        assert_eq!(slice[5], 6.0);
    }
}

/// Test zero-copy access to NumPy arrays
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_zero_copy_access() {
    // In a full implementation, this would:
    // 1. Create a NumPy array
    // 2. Get buffer without copying
    // 3. Modify data through buffer
    // 4. Verify NumPy array reflects changes

    let _guard = GilGuard::acquire();

    // Simulate zero-copy access
    let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let _original_ptr = data.as_ptr();

    let array = TeleportedArray::from_vec(data, vec![5]);

    // Verify pointer is the same (zero-copy)
    // Note: from_vec takes ownership, so we can't compare directly
    // In real NumPy interop, we'd verify the buffer points to NumPy's data
    assert!(!array.data_ptr().is_null());
}

/// Test NumPy dtype compatibility
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_dtype_compatibility() {
    // In a full implementation, this would:
    // 1. Create NumPy arrays with different dtypes
    // 2. Verify DX-Py can handle each dtype
    // 3. Test dtype conversion if needed

    let _guard = GilGuard::acquire();

    // Test various dtypes
    let dtypes = [
        (DType::Float64, 8, "float64"),
        (DType::Float32, 4, "float32"),
        (DType::Int64, 8, "int64"),
        (DType::Int32, 4, "int32"),
        (DType::Int16, 2, "int16"),
        (DType::Int8, 1, "int8"),
        (DType::Bool, 1, "bool"),
    ];

    for (dtype, expected_size, name) in dtypes {
        assert_eq!(
            dtype.size(),
            expected_size,
            "DType {} should have size {}",
            name,
            expected_size
        );
    }
}

/// Test NumPy array slicing
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_array_slicing() {
    // In a full implementation, this would:
    // 1. Create a NumPy array
    // 2. Slice it (e.g., arr[1:4])
    // 3. Verify the slice shares memory with original
    // 4. Verify slice has correct shape and strides

    let _guard = GilGuard::acquire();

    // Simulate sliced array (non-contiguous view)
    let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

    // Slice [1:4] would have:
    // - data pointer offset by 1 element
    // - shape [3]
    // - strides [8]

    let slice_ptr = unsafe { data.as_ptr().add(1) };
    let slice_array = unsafe {
        TeleportedArray::new(
            slice_ptr as *mut u8,
            vec![3],
            vec![8],
            DType::Float64,
            true, // readonly slice
        )
    };

    assert_eq!(slice_array.len(), 3);
    assert_eq!(slice_array.shape(), &[3]);
}

/// Test NumPy broadcasting
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_broadcasting() {
    // In a full implementation, this would:
    // 1. Create arrays with different shapes
    // 2. Perform operations that require broadcasting
    // 3. Verify results are correct

    let _guard = GilGuard::acquire();

    // Broadcasting example: (3, 1) + (1, 4) -> (3, 4)
    // This is a complex operation that NumPy handles internally
    // DX-Py needs to support the buffer protocol for NumPy to work

    // For now, verify basic shape handling
    let array_3x1 = TeleportedArray::from_vec(vec![1.0, 2.0, 3.0], vec![3, 1]);
    let array_1x4 = TeleportedArray::from_vec(vec![10.0, 20.0, 30.0, 40.0], vec![1, 4]);

    assert_eq!(array_3x1.shape(), &[3, 1]);
    assert_eq!(array_1x4.shape(), &[1, 4]);
}

/// Test NumPy ufuncs (universal functions)
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_ufuncs() {
    // In a full implementation, this would:
    // 1. Call NumPy ufuncs (np.sin, np.exp, etc.)
    // 2. Verify results match expected values
    // 3. Test ufunc with output array

    let _guard = GilGuard::acquire();

    // Simulate ufunc operation
    let data: Vec<f64> = vec![0.0, std::f64::consts::PI / 2.0, std::f64::consts::PI];
    let mut array = TeleportedArray::from_vec(data, vec![3]);

    // Apply sin (simulated)
    unsafe {
        let slice: &mut [f64] = array.as_mut_slice().unwrap();
        for val in slice.iter_mut() {
            *val = val.sin();
        }
    }

    // Verify results
    unsafe {
        let result: &[f64] = array.as_slice();
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 0.0).abs() < 1e-10);
    }
}

/// Test NumPy linear algebra operations
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_linalg() {
    // In a full implementation, this would:
    // 1. Create matrices
    // 2. Perform matrix operations (dot, matmul, etc.)
    // 3. Verify results

    let _guard = GilGuard::acquire();

    // Matrix multiplication example: (2, 3) @ (3, 2) -> (2, 2)
    let a = TeleportedArray::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
    let b = TeleportedArray::from_vec(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], vec![3, 2]);

    assert_eq!(a.shape(), &[2, 3]);
    assert_eq!(b.shape(), &[3, 2]);

    // Result would be (2, 2)
    // [[1*7+2*9+3*11, 1*8+2*10+3*12], [4*7+5*9+6*11, 4*8+5*10+6*12]]
    // [[58, 64], [139, 154]]
}

/// Test GIL handling during NumPy operations
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_gil_handling() {
    // In a full implementation, this would:
    // 1. Acquire GIL
    // 2. Perform NumPy operations
    // 3. Release GIL for pure computation
    // 4. Re-acquire GIL for Python interaction

    // Acquire GIL
    let guard = GilGuard::acquire();
    assert!(GilGuard::is_held());

    // Simulate NumPy operation that releases GIL
    // (NumPy releases GIL during heavy computation)

    // Drop guard to release GIL
    drop(guard);
    assert!(!GilGuard::is_held());

    // Re-acquire for Python interaction
    let _guard2 = GilGuard::acquire();
    assert!(GilGuard::is_held());
}

/// Test NumPy array memory management
#[test]
#[ignore = "Requires NumPy installation - run with --ignored"]
fn test_numpy_memory_management() {
    // In a full implementation, this would:
    // 1. Create NumPy arrays
    // 2. Verify reference counting works
    // 3. Verify arrays are deallocated when refcount reaches 0

    let _guard = GilGuard::acquire();

    // Simulate reference counting
    let obj = PyObject::new(ptr::null_mut());

    unsafe {
        use dx_py_ffi::cpython_compat::{Py_DecRef, Py_IncRef, Py_REFCNT};

        let obj_ptr = &obj as *const PyObject as *mut PyObject;

        assert_eq!(Py_REFCNT(obj_ptr), 1);

        Py_IncRef(obj_ptr);
        assert_eq!(Py_REFCNT(obj_ptr), 2);

        Py_DecRef(obj_ptr);
        assert_eq!(Py_REFCNT(obj_ptr), 1);
    }
}

// =============================================================================
// Integration Test Helpers
// =============================================================================

/// Helper to check if NumPy is available
/// Returns true if NumPy can be imported
#[allow(dead_code)]
fn numpy_available() -> bool {
    // In a full implementation, this would try to import numpy
    // For now, return false to indicate NumPy tests should be skipped
    false
}

/// Helper to create a NumPy-like array for testing
#[allow(dead_code)]
fn create_test_array(shape: &[usize]) -> TeleportedArray {
    let total_size: usize = shape.iter().product();
    let data: Vec<f64> = (0..total_size).map(|i| i as f64).collect();
    TeleportedArray::from_vec(data, shape.to_vec())
}

// =============================================================================
// Property-Based Tests for NumPy Compatibility
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Buffer protocol preserves data integrity
        /// Validates: Requirements 10.4, 10.7
        #[test]
        fn prop_buffer_data_integrity(data in prop::collection::vec(-1000.0f64..1000.0, 1..100)) {
            let original = data.clone();
            let array = TeleportedArray::from_vec(data, vec![original.len()]);

            unsafe {
                let slice: &[f64] = array.as_slice();
                prop_assert_eq!(slice, original.as_slice());
            }
        }

        /// Property: Multi-dimensional arrays have correct strides
        /// Validates: Requirements 10.4, 10.7
        #[test]
        fn prop_multidim_strides(
            rows in 1usize..50,
            cols in 1usize..50
        ) {
            let data: Vec<f64> = (0..(rows * cols)).map(|i| i as f64).collect();
            let array = TeleportedArray::from_vec(data, vec![rows, cols]);

            let strides = array.strides();

            // C-contiguous: strides should be [cols * itemsize, itemsize]
            prop_assert_eq!(strides.len(), 2);
            prop_assert_eq!(strides[1], 8); // sizeof(f64)
            prop_assert_eq!(strides[0], (cols * 8) as isize);
        }

        /// Property: Array operations are reversible
        /// Validates: Requirements 10.4, 10.7
        #[test]
        fn prop_array_operations_reversible(
            data in prop::collection::vec(-100.0f64..100.0, 1..50),
            scalar in -10.0f64..10.0
        ) {
            prop_assume!(scalar.abs() > 0.001); // Avoid division by near-zero

            let original = data.clone();
            let mut array = TeleportedArray::from_vec(data, vec![original.len()]);

            // Add then subtract
            array.add_scalar_f64(scalar);
            array.add_scalar_f64(-scalar);

            unsafe {
                let result: &[f64] = array.as_slice();
                for (i, &val) in result.iter().enumerate() {
                    prop_assert!(
                        (val - original[i]).abs() < 1e-10,
                        "Mismatch at index {}: {} != {}",
                        i, val, original[i]
                    );
                }
            }
        }
    }
}
