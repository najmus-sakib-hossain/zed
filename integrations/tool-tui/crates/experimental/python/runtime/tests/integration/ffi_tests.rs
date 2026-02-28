//! FFI Integration Tests
//!
//! Tests for zero-copy array operations and NumPy interop.

use dx_py_ffi::{TeleportedArray, DType};

#[test]
fn test_teleported_array_creation() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0, 5.0];
    let array = TeleportedArray::from_vec(data, vec![5]);
    
    assert_eq!(array.shape(), &[5]);
    assert_eq!(array.dtype(), DType::Float64);
    assert_eq!(array.len(), 5);
    assert_eq!(array.ndim(), 1);
    assert!(array.is_contiguous());
}

#[test]
fn test_teleported_array_2d() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
    let array = TeleportedArray::from_vec(data, vec![2, 3]);
    
    assert_eq!(array.shape(), &[2, 3]);
    assert_eq!(array.ndim(), 2);
    assert_eq!(array.len(), 6);
    assert!(array.is_contiguous());
}

#[test]
fn test_teleported_array_zero_copy_read() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0];
    let array = TeleportedArray::from_vec(data, vec![4]);
    
    unsafe {
        let slice: &[f64] = array.as_slice();
        assert_eq!(slice, &[1.0, 2.0, 3.0, 4.0]);
    }
}

#[test]
fn test_teleported_array_zero_copy_write() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0];
    let mut array = TeleportedArray::from_vec(data, vec![4]);
    
    unsafe {
        if let Some(slice) = array.as_mut_slice::<f64>() {
            slice[0] = 10.0;
            slice[3] = 40.0;
        }
        
        let slice: &[f64] = array.as_slice();
        assert_eq!(slice, &[10.0, 2.0, 3.0, 40.0]);
    }
}

#[test]
fn test_teleported_array_add_scalar() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0];
    let mut array = TeleportedArray::from_vec(data, vec![4]);
    
    assert!(array.add_scalar_f64(10.0));
    
    unsafe {
        let slice: &[f64] = array.as_slice();
        assert_eq!(slice, &[11.0, 12.0, 13.0, 14.0]);
    }
}

#[test]
fn test_teleported_array_mul_scalar() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0];
    let mut array = TeleportedArray::from_vec(data, vec![4]);
    
    assert!(array.mul_scalar_f64(2.0));
    
    unsafe {
        let slice: &[f64] = array.as_slice();
        assert_eq!(slice, &[2.0, 4.0, 6.0, 8.0]);
    }
}

#[test]
fn test_teleported_array_large() {
    // Test with larger array to exercise SIMD paths
    let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let mut array = TeleportedArray::from_vec(data, vec![1000]);
    
    assert!(array.add_scalar_f64(1.0));
    
    unsafe {
        let slice: &[f64] = array.as_slice();
        assert_eq!(slice[0], 1.0);
        assert_eq!(slice[999], 1000.0);
    }
}

#[test]
fn test_dtype_sizes() {
    assert_eq!(DType::Float64.size(), 8);
    assert_eq!(DType::Float32.size(), 4);
    assert_eq!(DType::Int64.size(), 8);
    assert_eq!(DType::Int32.size(), 4);
    assert_eq!(DType::Int16.size(), 2);
    assert_eq!(DType::Int8.size(), 1);
    assert_eq!(DType::Bool.size(), 1);
    assert_eq!(DType::Complex128.size(), 16);
}

#[test]
fn test_teleported_array_int64() {
    let data = vec![1i64, 2, 3, 4, 5];
    let array = TeleportedArray::from_vec(data, vec![5]);
    
    assert_eq!(array.dtype(), DType::Int64);
    assert_eq!(array.len(), 5);
    
    unsafe {
        let slice: &[i64] = array.as_slice();
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    }
}

#[test]
fn test_teleported_array_f32() {
    let data = vec![1.0f32, 2.0, 3.0, 4.0];
    let array = TeleportedArray::from_vec(data, vec![4]);
    
    assert_eq!(array.dtype(), DType::Float32);
    
    unsafe {
        let slice: &[f32] = array.as_slice();
        assert_eq!(slice, &[1.0f32, 2.0, 3.0, 4.0]);
    }
}

#[test]
fn test_teleported_array_empty() {
    let data: Vec<f64> = vec![];
    let array = TeleportedArray::from_vec(data, vec![0]);
    
    assert!(array.is_empty());
    assert_eq!(array.len(), 0);
}

#[test]
fn test_teleported_array_byte_size() {
    let data = vec![1.0f64, 2.0, 3.0, 4.0];
    let array = TeleportedArray::from_vec(data, vec![4]);
    
    assert_eq!(array.byte_size(), 32); // 4 * 8 bytes
}
