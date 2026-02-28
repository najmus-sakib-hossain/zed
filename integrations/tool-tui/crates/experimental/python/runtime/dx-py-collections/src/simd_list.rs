//! SIMD-accelerated list implementation
//!
//! Provides SIMD-accelerated operations for homogeneous lists:
//! - x86_64: AVX2/AVX-512 acceleration
//! - aarch64: NEON acceleration
//! - Other: Scalar fallback

use crate::simd_storage::{PyObjectRef, SimdStorage};

#[cfg(target_arch = "aarch64")]
use crate::neon_ops;

/// A list with SIMD-accelerated operations for homogeneous data
#[derive(Debug, Clone)]
pub struct SimdList {
    /// Storage for the list elements
    storage: SimdStorage,
}

impl SimdList {
    /// Create a new empty list
    pub fn new() -> Self {
        Self {
            storage: SimdStorage::Empty,
        }
    }

    /// Create from a vector of integers
    pub fn from_ints(values: Vec<i64>) -> Self {
        Self {
            storage: SimdStorage::from_ints(values),
        }
    }

    /// Create from a vector of floats
    pub fn from_floats(values: Vec<f64>) -> Self {
        Self {
            storage: SimdStorage::from_floats(values),
        }
    }

    /// Create from a Python list (detects homogeneous types)
    pub fn from_py_list(items: Vec<PyObjectRef>) -> Self {
        if items.is_empty() {
            return Self::new();
        }

        // Check if all items are the same type
        let first_type = items[0].type_tag;
        let homogeneous = items.iter().all(|item| item.type_tag == first_type);

        if homogeneous {
            match first_type {
                1 => {
                    // All integers
                    let ints: Vec<i64> = items.iter().filter_map(|item| item.as_int()).collect();
                    Self::from_ints(ints)
                }
                2 => {
                    // All floats
                    let floats: Vec<f64> =
                        items.iter().filter_map(|item| item.as_float()).collect();
                    Self::from_floats(floats)
                }
                _ => {
                    // Other homogeneous type - use mixed storage
                    Self {
                        storage: SimdStorage::from_mixed(items),
                    }
                }
            }
        } else {
            // Mixed types
            Self {
                storage: SimdStorage::from_mixed(items),
            }
        }
    }

    /// Get the length
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Check if storage is homogeneous integers
    pub fn is_int_list(&self) -> bool {
        self.storage.is_ints()
    }

    /// Check if storage is homogeneous floats
    pub fn is_float_list(&self) -> bool {
        self.storage.is_floats()
    }

    /// Sum all elements (SIMD-accelerated for homogeneous types)
    pub fn sum(&self) -> Option<f64> {
        match &self.storage {
            SimdStorage::Ints(v) => Some(sum_ints_simd(v) as f64),
            SimdStorage::Floats(v) => Some(sum_floats_simd(v)),
            SimdStorage::Mixed(v) => {
                let mut sum = 0.0;
                for item in v {
                    if let Some(i) = item.as_int() {
                        sum += i as f64;
                    } else if let Some(f) = item.as_float() {
                        sum += f;
                    } else {
                        return None; // Non-numeric type
                    }
                }
                Some(sum)
            }
            SimdStorage::Empty => Some(0.0),
        }
    }

    /// Filter elements greater than a value (SIMD-accelerated)
    pub fn filter_gt_int(&self, threshold: i64) -> Vec<usize> {
        match &self.storage {
            SimdStorage::Ints(v) => filter_gt_int_simd(v, threshold),
            _ => Vec::new(),
        }
    }

    /// Map multiply by 2 (SIMD-accelerated)
    pub fn map_mul2_int(&self) -> Option<SimdList> {
        match &self.storage {
            SimdStorage::Ints(v) => {
                let result = map_mul2_int_simd(v);
                Some(SimdList::from_ints(result))
            }
            _ => None,
        }
    }

    /// Find index of value (SIMD-accelerated)
    pub fn index_int(&self, value: i64) -> Option<usize> {
        match &self.storage {
            SimdStorage::Ints(v) => index_int_simd(v, value),
            _ => None,
        }
    }

    /// Count occurrences of value (SIMD-accelerated)
    pub fn count_int(&self, value: i64) -> usize {
        match &self.storage {
            SimdStorage::Ints(v) => count_int_simd(v, value),
            _ => 0,
        }
    }

    /// Get the underlying storage
    pub fn storage(&self) -> &SimdStorage {
        &self.storage
    }
}

impl Default for SimdList {
    fn default() -> Self {
        Self::new()
    }
}

/// SIMD-accelerated integer sum
fn sum_ints_simd(values: &[i64]) -> i64 {
    // Use NEON on aarch64, auto-vectorization on other platforms
    #[cfg(target_arch = "aarch64")]
    {
        neon_ops::sum_i64_neon(values)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        values.iter().sum()
    }
}

/// SIMD-accelerated float sum
fn sum_floats_simd(values: &[f64]) -> f64 {
    #[cfg(target_arch = "aarch64")]
    {
        neon_ops::sum_f64_neon(values)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        values.iter().sum()
    }
}

/// SIMD-accelerated filter greater than
#[cfg(target_arch = "x86_64")]
fn filter_gt_int_simd(values: &[i64], threshold: i64) -> Vec<usize> {
    #[cfg(target_feature = "avx2")]
    {
        use std::arch::x86_64::*;

        if values.len() >= 4 {
            unsafe {
                let mut indices = Vec::new();
                let threshold_vec = _mm256_set1_epi64x(threshold);
                let chunks = values.len() / 4;

                for i in 0..chunks {
                    let ptr = values.as_ptr().add(i * 4);
                    let v = _mm256_loadu_si256(ptr as *const __m256i);
                    let cmp = _mm256_cmpgt_epi64(v, threshold_vec);
                    let mask = _mm256_movemask_pd(_mm256_castsi256_pd(cmp)) as u32;

                    for j in 0..4 {
                        if mask & (1 << j) != 0 {
                            indices.push(i * 4 + j);
                        }
                    }
                }

                // Handle remainder
                for (j, &v) in values[chunks * 4..].iter().enumerate() {
                    if v > threshold {
                        indices.push(chunks * 4 + j);
                    }
                }

                return indices;
            }
        }
    }

    // Scalar fallback
    values
        .iter()
        .enumerate()
        .filter(|(_, &v)| v > threshold)
        .map(|(i, _)| i)
        .collect()
}

#[cfg(target_arch = "aarch64")]
fn filter_gt_int_simd(values: &[i64], threshold: i64) -> Vec<usize> {
    neon_ops::filter_gt_i64_neon(values, threshold)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn filter_gt_int_simd(values: &[i64], threshold: i64) -> Vec<usize> {
    values
        .iter()
        .enumerate()
        .filter(|(_, &v)| v > threshold)
        .map(|(i, _)| i)
        .collect()
}

/// SIMD-accelerated multiply by 2
fn map_mul2_int_simd(values: &[i64]) -> Vec<i64> {
    #[cfg(target_arch = "aarch64")]
    {
        neon_ops::map_mul2_i64_neon(values)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        // Use auto-vectorization friendly loop
        values.iter().map(|&v| v * 2).collect()
    }
}

/// SIMD-accelerated index search
#[cfg(target_arch = "x86_64")]
fn index_int_simd(values: &[i64], target: i64) -> Option<usize> {
    #[cfg(target_feature = "avx2")]
    {
        use std::arch::x86_64::*;

        if values.len() >= 4 {
            unsafe {
                let target_vec = _mm256_set1_epi64x(target);
                let chunks = values.len() / 4;

                for i in 0..chunks {
                    let ptr = values.as_ptr().add(i * 4);
                    let v = _mm256_loadu_si256(ptr as *const __m256i);
                    let cmp = _mm256_cmpeq_epi64(v, target_vec);
                    let mask = _mm256_movemask_pd(_mm256_castsi256_pd(cmp)) as u32;

                    if mask != 0 {
                        for j in 0..4 {
                            if mask & (1 << j) != 0 {
                                return Some(i * 4 + j);
                            }
                        }
                    }
                }

                // Handle remainder
                for (j, &v) in values[chunks * 4..].iter().enumerate() {
                    if v == target {
                        return Some(chunks * 4 + j);
                    }
                }

                return None;
            }
        }
    }

    // Scalar fallback
    values.iter().position(|&v| v == target)
}

#[cfg(target_arch = "aarch64")]
fn index_int_simd(values: &[i64], target: i64) -> Option<usize> {
    neon_ops::index_i64_neon(values, target)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn index_int_simd(values: &[i64], target: i64) -> Option<usize> {
    values.iter().position(|&v| v == target)
}

/// SIMD-accelerated count
#[cfg(target_arch = "x86_64")]
fn count_int_simd(values: &[i64], target: i64) -> usize {
    #[cfg(target_feature = "avx2")]
    {
        use std::arch::x86_64::*;

        if values.len() >= 4 {
            unsafe {
                let target_vec = _mm256_set1_epi64x(target);
                let chunks = values.len() / 4;
                let mut count = 0usize;

                for i in 0..chunks {
                    let ptr = values.as_ptr().add(i * 4);
                    let v = _mm256_loadu_si256(ptr as *const __m256i);
                    let cmp = _mm256_cmpeq_epi64(v, target_vec);
                    let mask = _mm256_movemask_pd(_mm256_castsi256_pd(cmp)) as u32;
                    count += mask.count_ones() as usize;
                }

                // Handle remainder
                for &v in &values[chunks * 4..] {
                    if v == target {
                        count += 1;
                    }
                }

                return count;
            }
        }
    }

    // Scalar fallback
    values.iter().filter(|&&v| v == target).count()
}

#[cfg(target_arch = "aarch64")]
fn count_int_simd(values: &[i64], target: i64) -> usize {
    neon_ops::count_i64_neon(values, target)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn count_int_simd(values: &[i64], target: i64) -> usize {
    values.iter().filter(|&&v| v == target).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ints() {
        let list = SimdList::from_ints(vec![1, 2, 3, 4, 5]);
        assert!(list.is_int_list());
        assert_eq!(list.len(), 5);
        assert_eq!(list.sum(), Some(15.0));
    }

    #[test]
    fn test_from_floats() {
        let list = SimdList::from_floats(vec![1.5, 2.5, 3.0]);
        assert!(list.is_float_list());
        assert_eq!(list.sum(), Some(7.0));
    }

    #[test]
    fn test_filter_gt() {
        let list = SimdList::from_ints(vec![1, 5, 2, 8, 3, 9, 4]);
        let indices = list.filter_gt_int(4);
        assert_eq!(indices, vec![1, 3, 5]);
    }

    #[test]
    fn test_map_mul2() {
        let list = SimdList::from_ints(vec![1, 2, 3, 4]);
        let doubled = list.map_mul2_int().unwrap();
        assert_eq!(doubled.storage().as_ints(), Some(&[2i64, 4, 6, 8][..]));
    }

    #[test]
    fn test_index() {
        let list = SimdList::from_ints(vec![10, 20, 30, 40, 50]);
        assert_eq!(list.index_int(30), Some(2));
        assert_eq!(list.index_int(99), None);
    }

    #[test]
    fn test_count() {
        let list = SimdList::from_ints(vec![1, 2, 1, 3, 1, 4, 1]);
        assert_eq!(list.count_int(1), 4);
        assert_eq!(list.count_int(5), 0);
    }

    #[test]
    fn test_from_py_list_homogeneous() {
        let items = vec![
            PyObjectRef::from_int(1),
            PyObjectRef::from_int(2),
            PyObjectRef::from_int(3),
        ];
        let list = SimdList::from_py_list(items);
        assert!(list.is_int_list());
    }

    #[test]
    fn test_from_py_list_mixed() {
        let items = vec![PyObjectRef::from_int(1), PyObjectRef::from_float(2.5)];
        let list = SimdList::from_py_list(items);
        assert!(!list.is_int_list());
        assert!(!list.is_float_list());
    }
}
