//! NEON-accelerated collection operations for ARM64
//!
//! This module provides SIMD-accelerated operations for collections
//! using ARM NEON instructions on aarch64 platforms.
//!
//! ## Operations
//!
//! - `sum_i64`: Sum of 64-bit integers using vaddq_s64
//! - `sum_f64`: Sum of 64-bit floats using vaddq_f64
//! - `filter_gt_i64`: Filter integers greater than threshold using vcgtq_s64
//! - `map_mul2_i64`: Multiply integers by 2 using vshlq_n_s64

/// Sum 64-bit integers using NEON
#[cfg(target_arch = "aarch64")]
pub fn sum_i64_neon(values: &[i64]) -> i64 {
    use std::arch::aarch64::*;

    if values.len() < 2 {
        return values.iter().sum();
    }

    unsafe {
        let chunks = values.len() / 2;
        let mut acc = vdupq_n_s64(0);

        for i in 0..chunks {
            let ptr = values.as_ptr().add(i * 2);
            let v = vld1q_s64(ptr);
            acc = vaddq_s64(acc, v);
        }

        // Horizontal sum
        let sum = vgetq_lane_s64(acc, 0) + vgetq_lane_s64(acc, 1);

        // Handle remainder
        let remainder: i64 = values[chunks * 2..].iter().sum();
        sum + remainder
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn sum_i64_neon(values: &[i64]) -> i64 {
    values.iter().sum()
}

/// Sum 64-bit floats using NEON
#[cfg(target_arch = "aarch64")]
pub fn sum_f64_neon(values: &[f64]) -> f64 {
    use std::arch::aarch64::*;

    if values.len() < 2 {
        return values.iter().sum();
    }

    unsafe {
        let chunks = values.len() / 2;
        let mut acc = vdupq_n_f64(0.0);

        for i in 0..chunks {
            let ptr = values.as_ptr().add(i * 2);
            let v = vld1q_f64(ptr);
            acc = vaddq_f64(acc, v);
        }

        // Horizontal sum
        let sum = vgetq_lane_f64(acc, 0) + vgetq_lane_f64(acc, 1);

        // Handle remainder
        let remainder: f64 = values[chunks * 2..].iter().sum();
        sum + remainder
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn sum_f64_neon(values: &[f64]) -> f64 {
    values.iter().sum()
}

/// Filter integers greater than threshold using NEON
#[cfg(target_arch = "aarch64")]
pub fn filter_gt_i64_neon(values: &[i64], threshold: i64) -> Vec<usize> {
    use std::arch::aarch64::*;

    if values.len() < 2 {
        return values
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > threshold)
            .map(|(i, _)| i)
            .collect();
    }

    unsafe {
        let mut indices = Vec::new();
        let threshold_vec = vdupq_n_s64(threshold);
        let chunks = values.len() / 2;

        for i in 0..chunks {
            let ptr = values.as_ptr().add(i * 2);
            let v = vld1q_s64(ptr);
            let cmp = vcgtq_s64(v, threshold_vec);

            // Extract comparison results
            let mask0 = vgetq_lane_u64(cmp, 0);
            let mask1 = vgetq_lane_u64(cmp, 1);

            if mask0 != 0 {
                indices.push(i * 2);
            }
            if mask1 != 0 {
                indices.push(i * 2 + 1);
            }
        }

        // Handle remainder
        for (j, &v) in values[chunks * 2..].iter().enumerate() {
            if v > threshold {
                indices.push(chunks * 2 + j);
            }
        }

        indices
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn filter_gt_i64_neon(values: &[i64], threshold: i64) -> Vec<usize> {
    values
        .iter()
        .enumerate()
        .filter(|(_, &v)| v > threshold)
        .map(|(i, _)| i)
        .collect()
}

/// Multiply integers by 2 using NEON (left shift by 1)
#[cfg(target_arch = "aarch64")]
pub fn map_mul2_i64_neon(values: &[i64]) -> Vec<i64> {
    use std::arch::aarch64::*;

    if values.len() < 2 {
        return values.iter().map(|&v| v * 2).collect();
    }

    unsafe {
        let mut result = Vec::with_capacity(values.len());
        let chunks = values.len() / 2;

        for i in 0..chunks {
            let ptr = values.as_ptr().add(i * 2);
            let v = vld1q_s64(ptr);
            let doubled = vshlq_n_s64::<1>(v);

            result.push(vgetq_lane_s64(doubled, 0));
            result.push(vgetq_lane_s64(doubled, 1));
        }

        // Handle remainder
        for &v in &values[chunks * 2..] {
            result.push(v * 2);
        }

        result
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn map_mul2_i64_neon(values: &[i64]) -> Vec<i64> {
    values.iter().map(|&v| v * 2).collect()
}

/// Find index of value using NEON
#[cfg(target_arch = "aarch64")]
pub fn index_i64_neon(values: &[i64], target: i64) -> Option<usize> {
    use std::arch::aarch64::*;

    if values.len() < 2 {
        return values.iter().position(|&v| v == target);
    }

    unsafe {
        let target_vec = vdupq_n_s64(target);
        let chunks = values.len() / 2;

        for i in 0..chunks {
            let ptr = values.as_ptr().add(i * 2);
            let v = vld1q_s64(ptr);
            let cmp = vceqq_s64(v, target_vec);

            let mask0 = vgetq_lane_u64(cmp, 0);
            let mask1 = vgetq_lane_u64(cmp, 1);

            if mask0 != 0 {
                return Some(i * 2);
            }
            if mask1 != 0 {
                return Some(i * 2 + 1);
            }
        }

        // Handle remainder
        for (j, &v) in values[chunks * 2..].iter().enumerate() {
            if v == target {
                return Some(chunks * 2 + j);
            }
        }

        None
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn index_i64_neon(values: &[i64], target: i64) -> Option<usize> {
    values.iter().position(|&v| v == target)
}

/// Count occurrences using NEON
#[cfg(target_arch = "aarch64")]
pub fn count_i64_neon(values: &[i64], target: i64) -> usize {
    use std::arch::aarch64::*;

    if values.len() < 2 {
        return values.iter().filter(|&&v| v == target).count();
    }

    unsafe {
        let target_vec = vdupq_n_s64(target);
        let chunks = values.len() / 2;
        let mut count = 0usize;

        for i in 0..chunks {
            let ptr = values.as_ptr().add(i * 2);
            let v = vld1q_s64(ptr);
            let cmp = vceqq_s64(v, target_vec);

            let mask0 = vgetq_lane_u64(cmp, 0);
            let mask1 = vgetq_lane_u64(cmp, 1);

            if mask0 != 0 {
                count += 1;
            }
            if mask1 != 0 {
                count += 1;
            }
        }

        // Handle remainder
        for &v in &values[chunks * 2..] {
            if v == target {
                count += 1;
            }
        }

        count
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn count_i64_neon(values: &[i64], target: i64) -> usize {
    values.iter().filter(|&&v| v == target).count()
}

/// Check if running on ARM64 with NEON support
pub fn is_neon_available() -> bool {
    cfg!(target_arch = "aarch64")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_i64() {
        let values = vec![1i64, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(sum_i64_neon(&values), 55);
    }

    #[test]
    fn test_sum_f64() {
        let values = vec![1.0f64, 2.0, 3.0, 4.0, 5.0];
        assert!((sum_f64_neon(&values) - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_filter_gt_i64() {
        let values = vec![1i64, 5, 2, 8, 3, 9, 4];
        let indices = filter_gt_i64_neon(&values, 4);
        assert_eq!(indices, vec![1, 3, 5]);
    }

    #[test]
    fn test_map_mul2_i64() {
        let values = vec![1i64, 2, 3, 4];
        let doubled = map_mul2_i64_neon(&values);
        assert_eq!(doubled, vec![2, 4, 6, 8]);
    }

    #[test]
    fn test_index_i64() {
        let values = vec![10i64, 20, 30, 40, 50];
        assert_eq!(index_i64_neon(&values, 30), Some(2));
        assert_eq!(index_i64_neon(&values, 99), None);
    }

    #[test]
    fn test_count_i64() {
        let values = vec![1i64, 2, 1, 3, 1, 4, 1];
        assert_eq!(count_i64_neon(&values, 1), 4);
        assert_eq!(count_i64_neon(&values, 5), 0);
    }

    #[test]
    fn test_empty_slice() {
        let empty: Vec<i64> = vec![];
        assert_eq!(sum_i64_neon(&empty), 0);
        assert_eq!(filter_gt_i64_neon(&empty, 0), Vec::<usize>::new());
        assert_eq!(map_mul2_i64_neon(&empty), Vec::<i64>::new());
        assert_eq!(index_i64_neon(&empty, 0), None);
        assert_eq!(count_i64_neon(&empty, 0), 0);
    }

    #[test]
    fn test_single_element() {
        let single = vec![42i64];
        assert_eq!(sum_i64_neon(&single), 42);
        assert_eq!(filter_gt_i64_neon(&single, 40), vec![0]);
        assert_eq!(map_mul2_i64_neon(&single), vec![84]);
        assert_eq!(index_i64_neon(&single, 42), Some(0));
        assert_eq!(count_i64_neon(&single, 42), 1);
    }
}
