//! Version types and comparison operations
//!
//! Provides SIMD-accelerated version comparison for high-performance resolution.

/// Packed version for SIMD operations
///
/// Versions are packed into a fixed-size struct for efficient
/// parallel comparison using AVX2 instructions.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackedVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// Padding for 16-byte alignment (SIMD friendly)
    pub _padding: u32,
}

impl PackedVersion {
    /// Create a new packed version
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            _padding: 0,
        }
    }

    /// Create a zeroed version (0.0.0)
    pub const fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    /// Parse a version string like "1.2.3"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);

        Some(Self::new(major, minor, patch))
    }

    /// Check if this version satisfies >= constraint
    pub fn satisfies_gte(&self, constraint: &PackedVersion) -> bool {
        *self >= *constraint
    }

    /// Check if this version satisfies < constraint
    pub fn satisfies_lt(&self, constraint: &PackedVersion) -> bool {
        *self < *constraint
    }
}

impl Default for PackedVersion {
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialOrd for PackedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PackedVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => match self.minor.cmp(&other.minor) {
                std::cmp::Ordering::Equal => self.patch.cmp(&other.patch),
                ord => ord,
            },
            ord => ord,
        }
    }
}

/// SIMD version comparison - process 8 versions in parallel using AVX2
///
/// Returns a bitmask where bit i is set if candidates[i] >= constraint_min.
///
/// # Safety
/// This function requires AVX2 support. Use `compare_versions_scalar` as fallback.
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2")]
pub unsafe fn compare_versions_simd(
    constraint_min: &PackedVersion,
    candidates: &[PackedVersion; 8],
) -> u8 {
    use std::arch::x86_64::*;

    // Load constraint minimum values
    let min_major = _mm256_set1_epi32(constraint_min.major as i32);
    let min_minor = _mm256_set1_epi32(constraint_min.minor as i32);
    let min_patch = _mm256_set1_epi32(constraint_min.patch as i32);

    // Load 8 candidate major versions (stride of 4 u32s = 16 bytes per PackedVersion)
    let majors = _mm256_set_epi32(
        candidates[7].major as i32,
        candidates[6].major as i32,
        candidates[5].major as i32,
        candidates[4].major as i32,
        candidates[3].major as i32,
        candidates[2].major as i32,
        candidates[1].major as i32,
        candidates[0].major as i32,
    );

    let minors = _mm256_set_epi32(
        candidates[7].minor as i32,
        candidates[6].minor as i32,
        candidates[5].minor as i32,
        candidates[4].minor as i32,
        candidates[3].minor as i32,
        candidates[2].minor as i32,
        candidates[1].minor as i32,
        candidates[0].minor as i32,
    );

    let patches = _mm256_set_epi32(
        candidates[7].patch as i32,
        candidates[6].patch as i32,
        candidates[5].patch as i32,
        candidates[4].patch as i32,
        candidates[3].patch as i32,
        candidates[2].patch as i32,
        candidates[1].patch as i32,
        candidates[0].patch as i32,
    );

    // Compare major > min_major
    let major_gt = _mm256_cmpgt_epi32(majors, min_major);
    // Compare major == min_major
    let major_eq = _mm256_cmpeq_epi32(majors, min_major);

    // Compare minor > min_minor
    let minor_gt = _mm256_cmpgt_epi32(minors, min_minor);
    // Compare minor == min_minor
    let minor_eq = _mm256_cmpeq_epi32(minors, min_minor);

    // Compare patch >= min_patch (patch > min_patch - 1)
    let patch_ge = _mm256_cmpgt_epi32(patches, _mm256_sub_epi32(min_patch, _mm256_set1_epi32(1)));

    // Result: major > min OR (major == min AND (minor > min OR (minor == min AND patch >= min)))
    let minor_match = _mm256_or_si256(minor_gt, _mm256_and_si256(minor_eq, patch_ge));
    let result = _mm256_or_si256(major_gt, _mm256_and_si256(major_eq, minor_match));

    // Extract mask (one bit per comparison result)
    _mm256_movemask_ps(_mm256_castsi256_ps(result)) as u8
}

/// Scalar fallback for version comparison
///
/// Returns a bitmask where bit i is set if candidates[i] >= constraint_min.
pub fn compare_versions_scalar(
    constraint_min: &PackedVersion,
    candidates: &[PackedVersion],
) -> Vec<bool> {
    candidates.iter().map(|c| c.satisfies_gte(constraint_min)).collect()
}

/// Compare versions using SIMD if available, otherwise scalar
///
/// Returns a vector of booleans where index i is true if candidates[i] >= constraint_min.
pub fn compare_versions(constraint_min: &PackedVersion, candidates: &[PackedVersion]) -> Vec<bool> {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        if candidates.len() >= 8 && is_x86_feature_detected!("avx2") {
            let mut results = Vec::with_capacity(candidates.len());
            let chunks = candidates.chunks_exact(8);
            let remainder = chunks.remainder();

            for chunk in chunks {
                let arr: [PackedVersion; 8] = chunk.try_into().unwrap();
                let mask = unsafe { compare_versions_simd(constraint_min, &arr) };
                for i in 0..8 {
                    results.push((mask >> i) & 1 == 1);
                }
            }

            // Handle remainder with scalar
            for c in remainder {
                results.push(c.satisfies_gte(constraint_min));
            }

            return results;
        }
    }

    compare_versions_scalar(constraint_min, candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        let v1 = PackedVersion::new(1, 0, 0);
        let v2 = PackedVersion::new(1, 1, 0);
        let v3 = PackedVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_version_equality() {
        let v1 = PackedVersion::new(1, 2, 3);
        let v2 = PackedVersion::new(1, 2, 3);

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_version_parse() {
        assert_eq!(PackedVersion::parse("1.2.3"), Some(PackedVersion::new(1, 2, 3)));
        assert_eq!(PackedVersion::parse("3.12"), Some(PackedVersion::new(3, 12, 0)));
        assert_eq!(PackedVersion::parse("invalid"), None);
    }

    #[test]
    fn test_scalar_comparison() {
        let constraint = PackedVersion::new(1, 5, 0);
        let candidates = vec![
            PackedVersion::new(1, 4, 0), // false
            PackedVersion::new(1, 5, 0), // true (equal)
            PackedVersion::new(1, 6, 0), // true
            PackedVersion::new(2, 0, 0), // true
        ];

        let results = compare_versions_scalar(&constraint, &candidates);
        assert_eq!(results, vec![false, true, true, true]);
    }
}
