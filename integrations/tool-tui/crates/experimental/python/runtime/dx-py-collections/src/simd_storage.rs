//! SIMD storage types for homogeneous collections

/// Storage type for SIMD-optimized lists
#[derive(Debug, Clone)]
pub enum SimdStorage {
    /// Homogeneous integer storage
    Ints(Vec<i64>),
    /// Homogeneous float storage
    Floats(Vec<f64>),
    /// Mixed/heterogeneous storage (fallback)
    Mixed(Vec<PyObjectRef>),
    /// Empty storage
    Empty,
}

/// Reference to a Python object (placeholder)
#[derive(Debug, Clone, PartialEq)]
pub struct PyObjectRef {
    /// Type tag
    pub type_tag: u8,
    /// Value (tagged pointer or inline value)
    pub value: u64,
}

impl PyObjectRef {
    /// Create an integer reference
    pub fn from_int(value: i64) -> Self {
        Self {
            type_tag: 1, // Int
            value: value as u64,
        }
    }

    /// Create a float reference
    pub fn from_float(value: f64) -> Self {
        Self {
            type_tag: 2, // Float
            value: value.to_bits(),
        }
    }

    /// Check if this is an integer
    pub fn is_int(&self) -> bool {
        self.type_tag == 1
    }

    /// Check if this is a float
    pub fn is_float(&self) -> bool {
        self.type_tag == 2
    }

    /// Get as integer
    pub fn as_int(&self) -> Option<i64> {
        if self.is_int() {
            Some(self.value as i64)
        } else {
            None
        }
    }

    /// Get as float
    pub fn as_float(&self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.value))
        } else {
            None
        }
    }
}

impl SimdStorage {
    /// Create empty storage
    pub fn new() -> Self {
        Self::Empty
    }

    /// Create integer storage
    pub fn from_ints(values: Vec<i64>) -> Self {
        if values.is_empty() {
            Self::Empty
        } else {
            Self::Ints(values)
        }
    }

    /// Create float storage
    pub fn from_floats(values: Vec<f64>) -> Self {
        if values.is_empty() {
            Self::Empty
        } else {
            Self::Floats(values)
        }
    }

    /// Create mixed storage
    pub fn from_mixed(values: Vec<PyObjectRef>) -> Self {
        if values.is_empty() {
            Self::Empty
        } else {
            Self::Mixed(values)
        }
    }

    /// Get the length
    pub fn len(&self) -> usize {
        match self {
            Self::Ints(v) => v.len(),
            Self::Floats(v) => v.len(),
            Self::Mixed(v) => v.len(),
            Self::Empty => 0,
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if storage is homogeneous integers
    pub fn is_ints(&self) -> bool {
        matches!(self, Self::Ints(_))
    }

    /// Check if storage is homogeneous floats
    pub fn is_floats(&self) -> bool {
        matches!(self, Self::Floats(_))
    }

    /// Check if storage is mixed
    pub fn is_mixed(&self) -> bool {
        matches!(self, Self::Mixed(_))
    }

    /// Get as integer slice
    pub fn as_ints(&self) -> Option<&[i64]> {
        match self {
            Self::Ints(v) => Some(v),
            _ => None,
        }
    }

    /// Get as float slice
    pub fn as_floats(&self) -> Option<&[f64]> {
        match self {
            Self::Floats(v) => Some(v),
            _ => None,
        }
    }

    /// Get as mixed slice
    pub fn as_mixed(&self) -> Option<&[PyObjectRef]> {
        match self {
            Self::Mixed(v) => Some(v),
            _ => None,
        }
    }

    /// Sum integers using SIMD
    pub fn sum_ints(&self) -> Option<i64> {
        let ints = self.as_ints()?;
        Some(sum_ints_simd(ints))
    }

    /// Sum floats using SIMD
    pub fn sum_floats(&self) -> Option<f64> {
        let floats = self.as_floats()?;
        Some(sum_floats_simd(floats))
    }
}

impl Default for SimdStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// SIMD-accelerated integer sum
#[cfg(target_arch = "x86_64")]
fn sum_ints_simd(values: &[i64]) -> i64 {
    #[cfg(target_feature = "avx2")]
    {
        use std::arch::x86_64::*;

        if values.len() >= 4 {
            unsafe {
                let mut sum = _mm256_setzero_si256();
                let chunks = values.len() / 4;

                for i in 0..chunks {
                    let ptr = values.as_ptr().add(i * 4);
                    let v = _mm256_loadu_si256(ptr as *const __m256i);
                    sum = _mm256_add_epi64(sum, v);
                }

                // Horizontal sum
                let mut result = [0i64; 4];
                _mm256_storeu_si256(result.as_mut_ptr() as *mut __m256i, sum);
                let mut total = result[0] + result[1] + result[2] + result[3];

                // Handle remainder
                for &v in &values[chunks * 4..] {
                    total += v;
                }

                return total;
            }
        }
    }

    // Scalar fallback
    values.iter().sum()
}

#[cfg(not(target_arch = "x86_64"))]
fn sum_ints_simd(values: &[i64]) -> i64 {
    values.iter().sum()
}

/// SIMD-accelerated float sum
#[cfg(target_arch = "x86_64")]
fn sum_floats_simd(values: &[f64]) -> f64 {
    #[cfg(target_feature = "avx")]
    {
        use std::arch::x86_64::*;

        if values.len() >= 4 {
            unsafe {
                let mut sum = _mm256_setzero_pd();
                let chunks = values.len() / 4;

                for i in 0..chunks {
                    let ptr = values.as_ptr().add(i * 4);
                    let v = _mm256_loadu_pd(ptr);
                    sum = _mm256_add_pd(sum, v);
                }

                // Horizontal sum
                let mut result = [0f64; 4];
                _mm256_storeu_pd(result.as_mut_ptr(), sum);
                let mut total = result[0] + result[1] + result[2] + result[3];

                // Handle remainder
                for &v in &values[chunks * 4..] {
                    total += v;
                }

                return total;
            }
        }
    }

    // Scalar fallback
    values.iter().sum()
}

#[cfg(not(target_arch = "x86_64"))]
fn sum_floats_simd(values: &[f64]) -> f64 {
    values.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_storage() {
        let storage = SimdStorage::from_ints(vec![1, 2, 3, 4, 5]);
        assert!(storage.is_ints());
        assert_eq!(storage.len(), 5);
        assert_eq!(storage.sum_ints(), Some(15));
    }

    #[test]
    fn test_float_storage() {
        let storage = SimdStorage::from_floats(vec![1.0, 2.0, 3.0, 4.0]);
        assert!(storage.is_floats());
        assert_eq!(storage.len(), 4);
        assert_eq!(storage.sum_floats(), Some(10.0));
    }

    #[test]
    fn test_empty_storage() {
        let storage = SimdStorage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn test_simd_sum_large() {
        let values: Vec<i64> = (0..1000).collect();
        let storage = SimdStorage::from_ints(values);
        assert_eq!(storage.sum_ints(), Some(499500));
    }
}
